use std::path::PathBuf;
use tokio::process::Child;

use super::purge_youtube_attempt::purge_youtube_attempt;

pub struct YoutubeProcessGuard {
    child: Option<Child>,
    staging_dir: PathBuf,
    stage_id: String,
    disarmed: bool,
}

impl YoutubeProcessGuard {
    pub fn new(child: Child, staging_dir: PathBuf, stage_id: String) -> Self {
        Self {
            child: Some(child),
            staging_dir,
            stage_id,
            disarmed: false,
        }
    }

    pub fn child_mut(&mut self) -> Option<&mut Child> {
        self.child.as_mut()
    }

    pub fn disarm(&mut self) {
        self.disarmed = true;
    }

    pub async fn kill_and_wait(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        purge_youtube_attempt(&self.staging_dir, &self.stage_id);
    }
}

impl Drop for YoutubeProcessGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }

        if let Some(mut child) = self.child.take() {
            let staging_dir = self.staging_dir.clone();
            let stage_id = self.stage_id.clone();

            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    purge_youtube_attempt(&staging_dir, &stage_id);
                });
            } else {
                let _ = child.start_kill();
                purge_youtube_attempt(&staging_dir, &stage_id);
            }
        } else {
            purge_youtube_attempt(&self.staging_dir, &self.stage_id);
        }
    }
}
