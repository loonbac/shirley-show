use std::path::PathBuf;
use std::process::{ExitStatus, Output, Stdio};
use std::time::Duration;
use tokio::process::{Child, Command};
use tokio::sync::OwnedSemaphorePermit;

use crate::ports::vocal_separator::VocalSeparationError;

/// RAII Guard managing vocal separation subprocesses, their isolated working directory,
/// their partial output artifact, and the exclusive GPU semaphore permit.
///
/// Ensures deterministic kill-and-wait across ALL spawned processes (separator and auxiliaries
/// like ffmpeg and ffprobe) upon error, timeout, or cancellation.
/// Crucially, on drop (such as future cancellation or client disconnect), the GPU semaphore permit
/// travels with the asynchronous cleanup task and is ONLY released after EVERY child process has
/// fully terminated via `wait()`.
pub struct VocalProcessGuard {
    children: Vec<Child>,
    work_dir: PathBuf,
    output_part: Option<PathBuf>,
    permit: Option<OwnedSemaphorePermit>,
    disarmed: bool,
}

impl VocalProcessGuard {
    pub fn new(
        work_dir: PathBuf,
        output_part: Option<PathBuf>,
        permit: OwnedSemaphorePermit,
    ) -> Self {
        Self {
            children: Vec::new(),
            work_dir,
            output_part,
            permit: Some(permit),
            disarmed: false,
        }
    }

    pub fn attach_child(&mut self, child: Child) {
        self.children.push(child);
    }

    pub async fn wait_child(&mut self) -> std::io::Result<ExitStatus> {
        if self.children.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No active child process attached to guard",
            ));
        }
        let last_idx = self.children.len() - 1;
        let res = self.children[last_idx].wait().await;
        if res.is_ok() {
            self.children.pop();
        }
        res
    }

    /// Runs a subprocess under the guard's kill-and-wait protection with a timeout,
    /// waiting for exit status.
    pub async fn run_status(
        &mut self,
        mut cmd: Command,
        timeout: Duration,
    ) -> Result<ExitStatus, VocalSeparationError> {
        cmd.kill_on_drop(true);
        let child = cmd
            .spawn()
            .map_err(|_| VocalSeparationError::SeparationFailed)?;
        self.children.push(child);
        let idx = self.children.len() - 1;

        let res = tokio::time::timeout(timeout, self.children[idx].wait()).await;
        match res {
            Ok(Ok(status)) => {
                self.children.swap_remove(idx);
                Ok(status)
            }
            Ok(Err(_)) => {
                self.kill_and_wait().await;
                Err(VocalSeparationError::SeparationFailed)
            }
            Err(_) => {
                self.kill_and_wait().await;
                Err(VocalSeparationError::SeparationTimeout)
            }
        }
    }

    /// Runs a subprocess under the guard's kill-and-wait protection with a timeout,
    /// capturing stdout and stderr.
    pub async fn run_output(
        &mut self,
        mut cmd: Command,
        timeout: Duration,
    ) -> Result<Output, VocalSeparationError> {
        cmd.kill_on_drop(true);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|_| VocalSeparationError::TranscodeFailed)?;
        let mut stdout_pipe = child.stdout.take();
        let mut stderr_pipe = child.stderr.take();

        self.children.push(child);
        let idx = self.children.len() - 1;

        let read_stdout = async {
            let mut out = Vec::new();
            if let Some(ref mut pipe) = stdout_pipe {
                let _ = tokio::io::AsyncReadExt::read_to_end(pipe, &mut out).await;
            }
            out
        };

        let read_stderr = async {
            let mut err = Vec::new();
            if let Some(ref mut pipe) = stderr_pipe {
                let _ = tokio::io::AsyncReadExt::read_to_end(pipe, &mut err).await;
            }
            err
        };

        let wait_child = self.children[idx].wait();

        let combined = async {
            let (status_res, stdout, stderr) = tokio::join!(wait_child, read_stdout, read_stderr);
            status_res.map(|status| Output {
                status,
                stdout,
                stderr,
            })
        };

        let res = tokio::time::timeout(timeout, combined).await;
        match res {
            Ok(Ok(output)) => {
                self.children.swap_remove(idx);
                Ok(output)
            }
            Ok(Err(_)) => {
                self.kill_and_wait().await;
                Err(VocalSeparationError::TranscodeFailed)
            }
            Err(_) => {
                self.kill_and_wait().await;
                Err(VocalSeparationError::SeparationTimeout)
            }
        }
    }

    pub async fn kill_and_wait(&mut self) {
        let children = std::mem::take(&mut self.children);
        for mut child in children {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }

        if self.work_dir.exists() {
            let _ = tokio::fs::remove_dir_all(&self.work_dir).await;
        }

        if let Some(ref part) = self.output_part {
            if part.exists() {
                let _ = tokio::fs::remove_file(part).await;
            }
        }

        // Permit is dropped here, AFTER all children have terminated and artifacts are purged.
        let _ = self.permit.take();
    }

    /// Disarms the guard on successful completion.
    /// Cleans up intermediate work directory, retains the output artifact, and releases the GPU permit.
    pub fn disarm(&mut self) {
        self.disarmed = true;
        if self.work_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.work_dir);
        }
        self.children.clear();
        let _ = self.permit.take();
    }
}

impl Drop for VocalProcessGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }

        let children = std::mem::take(&mut self.children);
        let work_dir = self.work_dir.clone();
        let output_part = self.output_part.clone();
        let permit = self.permit.take();

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                // HOLD permit until ALL children are killed and waited on!
                let _permit = permit;

                for mut child in children {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                }

                if work_dir.exists() {
                    let _ = tokio::fs::remove_dir_all(&work_dir).await;
                }

                if let Some(ref part) = output_part {
                    if part.exists() {
                        let _ = tokio::fs::remove_file(part).await;
                    }
                }
                // _permit drops here AFTER wait() and purges finish!
            });
        } else {
            // Synchronous fallback outside tokio runtime
            for mut child in children {
                let _ = child.start_kill();
            }
            if work_dir.exists() {
                let _ = std::fs::remove_dir_all(&work_dir);
            }
            if let Some(ref part) = output_part {
                if part.exists() {
                    let _ = std::fs::remove_file(part);
                }
            }
        }
    }
}
