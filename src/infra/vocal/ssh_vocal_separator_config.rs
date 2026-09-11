use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::config::JoshuaPcConfig;

#[derive(Clone, Debug)]
pub struct SshVocalSeparatorConfig {
    pub joshua_pc: JoshuaPcConfig,
    pub ssh_bin: PathBuf,
    pub scp_bin: PathBuf,
    pub ffprobe_bin: PathBuf,
    pub gpu: Arc<Semaphore>,
}

impl SshVocalSeparatorConfig {
    pub fn new(joshua_pc: JoshuaPcConfig, gpu: Arc<Semaphore>) -> Self {
        Self {
            joshua_pc,
            ssh_bin: PathBuf::from("ssh"),
            scp_bin: PathBuf::from("scp"),
            ffprobe_bin: PathBuf::from("ffprobe"),
            gpu,
        }
    }

    pub fn with_custom_bins(
        joshua_pc: JoshuaPcConfig,
        ssh_bin: PathBuf,
        scp_bin: PathBuf,
        ffprobe_bin: PathBuf,
        gpu: Arc<Semaphore>,
    ) -> Self {
        Self {
            joshua_pc,
            ssh_bin,
            scp_bin,
            ffprobe_bin,
            gpu,
        }
    }
}
