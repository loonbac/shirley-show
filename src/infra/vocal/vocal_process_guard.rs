use std::path::PathBuf;
use tokio::process::Child;
use tokio::sync::OwnedSemaphorePermit;

/// RAII Guard managing a vocal separation subprocess, its isolated working directory,
/// its partial output artifact, and the exclusive GPU semaphore permit.
///
/// Ensures deterministic kill-and-wait and directory purges upon error, timeout, or cancellation.
/// Crucially, on drop (such as future cancellation or client disconnect), the GPU semaphore permit
/// travels with the asynchronous cleanup task and is ONLY released after the child process has
/// fully terminated via `wait()`.
pub struct VocalProcessGuard {
    child: Option<Child>,
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
            child: None,
            work_dir,
            output_part,
            permit: Some(permit),
            disarmed: false,
        }
    }

    pub fn attach_child(&mut self, child: Child) {
        self.child = Some(child);
    }

    pub fn child_mut(&mut self) -> Option<&mut Child> {
        self.child.as_mut()
    }

    pub async fn wait_child(&mut self) -> std::io::Result<std::process::ExitStatus> {
        if let Some(mut child) = self.child.take() {
            child.wait().await
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No active child process attached to guard",
            ))
        }
    }

    pub async fn kill_and_wait(&mut self) {
        if let Some(mut child) = self.child.take() {
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

        // Permit is dropped here, after child has terminated and artifacts are purged.
        let _ = self.permit.take();
    }

    /// Disarms the guard on successful completion.
    /// Cleans up intermediate work directory, retains the output artifact, and releases the GPU permit.
    pub fn disarm(&mut self) {
        self.disarmed = true;
        if self.work_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.work_dir);
        }
        let _ = self.child.take();
        let _ = self.permit.take();
    }
}

impl Drop for VocalProcessGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }

        let child = self.child.take();
        let work_dir = self.work_dir.clone();
        let output_part = self.output_part.clone();
        let permit = self.permit.take();

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                // HOLD permit until child is killed and waited on!
                let _permit = permit;

                if let Some(mut child) = child {
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
                // _permit drops here AFTER child.wait().await and purges finish!
            });
        } else {
            // Synchronous fallback outside tokio runtime
            if let Some(mut child) = child {
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
