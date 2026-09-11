use std::path::PathBuf;
use std::process::{ExitStatus, Output, Stdio};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::OwnedSemaphorePermit;

use super::ssh_command_builder::SshCommandBuilder;
use crate::ports::vocal_separator::VocalSeparationError;

/// RAII Guard managing remote execution subprocesses, the remote working directory
/// on nixos-pc (/tmp/shirley-vocal/<op_id>), the local partial output file,
/// and the exclusive concurrency permit.
pub struct RemoteDirectoryGuard {
    children: Vec<Child>,
    remote_dir: Option<String>,
    output_part: Option<PathBuf>,
    permit: Option<OwnedSemaphorePermit>,
    command_builder: SshCommandBuilder,
    disarmed: bool,
}

async fn spawn_with_retry(cmd: &mut Command) -> Result<Child, VocalSeparationError> {
    let mut attempts = 0;
    loop {
        match cmd.spawn() {
            Ok(child) => return Ok(child),
            Err(e) if e.raw_os_error() == Some(26) && attempts < 5 => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(15)).await;
                continue;
            }
            Err(_) => return Err(VocalSeparationError::SeparationFailed),
        }
    }
}

impl RemoteDirectoryGuard {
    pub fn new(
        output_part: Option<PathBuf>,
        permit: OwnedSemaphorePermit,
        command_builder: SshCommandBuilder,
    ) -> Self {
        Self {
            children: Vec::new(),
            remote_dir: None,
            output_part,
            permit: Some(permit),
            command_builder,
            disarmed: false,
        }
    }

    pub fn arm_remote_dir(&mut self, remote_dir: &str) {
        self.remote_dir = Some(remote_dir.to_string());
    }

    pub fn remote_dir(&self) -> Option<&str> {
        self.remote_dir.as_deref()
    }

    pub fn attach_child(&mut self, child: Child) {
        self.children.push(child);
    }

    pub async fn run_status(
        &mut self,
        mut cmd: Command,
        timeout: Duration,
    ) -> Result<ExitStatus, VocalSeparationError> {
        cmd.kill_on_drop(true);
        let child = spawn_with_retry(&mut cmd).await?;
        self.children.push(child);
        let idx = self.children.len() - 1;

        let res = tokio::time::timeout(timeout, self.children[idx].wait()).await;
        match res {
            Ok(Ok(status)) => {
                self.children.swap_remove(idx);
                Ok(status)
            }
            Ok(Err(_)) => {
                self.kill_and_cleanup().await;
                Err(VocalSeparationError::SeparationFailed)
            }
            Err(_) => {
                self.kill_and_cleanup().await;
                Err(VocalSeparationError::SeparationTimeout)
            }
        }
    }

    pub async fn run_output(
        &mut self,
        mut cmd: Command,
        timeout: Duration,
    ) -> Result<Output, VocalSeparationError> {
        cmd.kill_on_drop(true);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        let mut child = spawn_with_retry(&mut cmd).await?;
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
                self.kill_and_cleanup().await;
                Err(VocalSeparationError::SeparationFailed)
            }
            Err(_) => {
                self.kill_and_cleanup().await;
                Err(VocalSeparationError::SeparationTimeout)
            }
        }
    }

    pub async fn run_pipeline_script(
        &mut self,
        mut cmd: Command,
        script: &str,
        timeout: Duration,
    ) -> Result<ExitStatus, VocalSeparationError> {
        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = spawn_with_retry(&mut cmd).await?;

        let mut stdin_pipe = child.stdin.take();
        let mut stdout_pipe = child.stdout.take();
        let mut stderr_pipe = child.stderr.take();

        self.children.push(child);
        let idx = self.children.len() - 1;

        let write_stdin = async {
            if let Some(ref mut pipe) = stdin_pipe {
                let _ = pipe.write_all(script.as_bytes()).await;
                let _ = pipe.flush().await;
            }
        };

        let drain_stdout = async {
            let mut buf = Vec::new();
            if let Some(ref mut pipe) = stdout_pipe {
                let _ = tokio::io::AsyncReadExt::read_to_end(pipe, &mut buf).await;
            }
            buf
        };

        let drain_stderr = async {
            let mut buf = Vec::new();
            if let Some(ref mut pipe) = stderr_pipe {
                let _ = tokio::io::AsyncReadExt::read_to_end(pipe, &mut buf).await;
            }
            buf
        };

        let wait_child = self.children[idx].wait();

        let combined = async {
            tokio::join!(write_stdin, drain_stdout, drain_stderr);
            wait_child.await
        };

        let res = tokio::time::timeout(timeout, combined).await;
        match res {
            Ok(Ok(status)) => {
                self.children.swap_remove(idx);
                Ok(status)
            }
            Ok(Err(_)) => {
                self.kill_and_cleanup().await;
                Err(VocalSeparationError::SeparationFailed)
            }
            Err(_) => {
                self.kill_and_cleanup().await;
                Err(VocalSeparationError::SeparationTimeout)
            }
        }
    }

    pub async fn kill_and_cleanup(&mut self) {
        // 1. Kill and wait all child processes
        let children = std::mem::take(&mut self.children);
        for mut child in children {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }

        // 2. Best-effort cleanup of remote directory
        if let Some(ref dir) = self.remote_dir {
            let mut cleanup_cmd = self.command_builder.build_cleanup(dir);
            cleanup_cmd.stdin(Stdio::null());
            cleanup_cmd.stdout(Stdio::null());
            cleanup_cmd.stderr(Stdio::null());
            if let Ok(mut child) = cleanup_cmd.spawn() {
                let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
            }
        }

        // 3. Purge local partial output
        if let Some(ref part) = self.output_part {
            let _ = tokio::fs::remove_file(part).await;
        }

        // 4. Release permit
        let _ = self.permit.take();
    }

    /// Complete successfully: performs explicit remote cleanup, preserves local output, and releases permit.
    pub async fn complete(&mut self) {
        self.disarmed = true;
        self.children.clear();

        if let Some(ref dir) = self.remote_dir {
            let mut cleanup_cmd = self.command_builder.build_cleanup(dir);
            cleanup_cmd.stdin(Stdio::null());
            cleanup_cmd.stdout(Stdio::null());
            cleanup_cmd.stderr(Stdio::null());
            if let Ok(mut child) = cleanup_cmd.spawn() {
                let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
            }
        }

        let _ = self.permit.take();
    }
}

impl Drop for RemoteDirectoryGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }

        let children = std::mem::take(&mut self.children);
        let remote_dir = self.remote_dir.take();
        let output_part = self.output_part.take();
        let permit = self.permit.take();
        let builder = self.command_builder.clone();

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                // HOLD permit until cleanup finishes!
                let _permit = permit;

                for mut child in children {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                }

                if let Some(ref dir) = remote_dir {
                    let mut cleanup_cmd = builder.build_cleanup(dir);
                    cleanup_cmd.stdin(Stdio::null());
                    cleanup_cmd.stdout(Stdio::null());
                    cleanup_cmd.stderr(Stdio::null());
                    if let Ok(mut c) = cleanup_cmd.spawn() {
                        let _ = tokio::time::timeout(Duration::from_secs(5), c.wait()).await;
                    }
                }

                if let Some(ref part) = output_part {
                    let _ = tokio::fs::remove_file(part).await;
                }
            });
        }
    }
}
