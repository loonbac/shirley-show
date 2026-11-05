use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::domain::vocal_separation_mode::VocalSeparationMode;
use crate::ports::vocal_separator::VocalSeparationError;

pub const INSTRUMENTAL_MODELS: [&str; 4] = [
    "model_bs_roformer_ep_317_sdr_12.9755.ckpt",
    "model_bs_roformer_ep_368_sdr_12.9628.ckpt",
    "melband_roformer_inst_v1e.ckpt",
    "mel_band_roformer_instrumental_becruily.ckpt",
];

pub const COROS_MODELS: [&str; 4] = [
    "bs_roformer_karaoke_frazer_becruily.ckpt",
    "bs_roformer_karaoke_anvuew.ckpt",
    "mel_band_roformer_karaoke_gabox_v2.ckpt",
    "mel_band_roformer_karaoke_aufr33_viperx_sdr_10.1956.ckpt",
];

#[derive(Clone, Debug)]
pub struct ColabVocalSeparatorConfig {
    pub colab_bin: PathBuf,
    pub token_path: PathBuf,
    pub gpu_type: String,
    pub overlap: u32,
    /// Inference batch size. Measured on the production GPU: 25s at batch 1,
    /// 8.5s at batch 8, and flat above that (8.5s at 32 and 64), so 8 is the
    /// knee and larger values only spend memory.
    pub batch_size: u32,
    pub timeout_secs: u64,
    pub work_dir: PathBuf,
    pub ffprobe_bin: PathBuf,
    pub openssl_bin: PathBuf,
    pub gpu: Arc<Semaphore>,
}

impl ColabVocalSeparatorConfig {
    /// Builds configuration from environment variables with defaults.
    pub fn from_env(work_dir: PathBuf, gpu: Arc<Semaphore>) -> Self {
        let colab_bin = std::env::var("SHIRLEY_COLAB_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("colab"));

        let token_path = std::env::var("SHIRLEY_COLAB_TOKEN")
            .map(expand_tilde)
            .unwrap_or_else(|_| default_token_path());

        let gpu_type = std::env::var("SHIRLEY_COLAB_GPU").unwrap_or_else(|_| "g4".to_string());

        let overlap = std::env::var("SHIRLEY_COLAB_OVERLAP")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(12);

        let batch_size = std::env::var("SHIRLEY_COLAB_BATCH_SIZE")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(8);

        let timeout_secs = std::env::var("SHIRLEY_COLAB_TIMEOUT_SECS")
            .ok()
            .or_else(|| std::env::var("SHIRLEY_SEPARATION_TIMEOUT_SECS").ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1800);

        let work_dir = std::env::var("SHIRLEY_COLAB_WORK_DIR")
            .map(PathBuf::from)
            .unwrap_or(work_dir);

        let ffprobe_bin = std::env::var("FFPROBE_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("ffprobe"));

        let openssl_bin = std::env::var("OPENSSL_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("openssl"));

        Self {
            colab_bin,
            token_path,
            gpu_type,
            overlap,
            batch_size,
            timeout_secs,
            work_dir,
            ffprobe_bin,
            openssl_bin,
            gpu,
        }
    }

    /// Returns the contractual models ensemble for a given separation mode.
    pub fn models_for_mode(mode: VocalSeparationMode) -> &'static [&'static str] {
        match mode {
            VocalSeparationMode::Instrumental => &INSTRUMENTAL_MODELS,
            VocalSeparationMode::Coros => &COROS_MODELS,
        }
    }

    /// Validates physical existence and readability of the Colab OAuth token.json file,
    /// and ensures a non-empty `refresh_token` string is present.
    ///
    /// If invalid or missing, returns `ColabTokenMissing` immediately without starting a GPU.
    pub fn validate_token(&self) -> Result<(), VocalSeparationError> {
        Self::validate_token_file(&self.token_path)
    }

    /// Pure validation function testing physical existence and refresh_token presence.
    pub fn validate_token_file(path: &Path) -> Result<(), VocalSeparationError> {
        let content =
            std::fs::read_to_string(path).map_err(|_| VocalSeparationError::ColabTokenMissing)?;
        let parsed: serde_json::Value =
            serde_json::from_str(&content).map_err(|_| VocalSeparationError::ColabTokenMissing)?;

        let has_refresh = parsed
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if has_refresh {
            Ok(())
        } else {
            Err(VocalSeparationError::ColabTokenMissing)
        }
    }
}

fn expand_tilde(path_str: String) -> PathBuf {
    if let Some(stripped) = path_str.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }
    PathBuf::from(path_str)
}

fn default_token_path() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/colab/token.json")
    } else {
        PathBuf::from(".config/colab/token.json")
    }
}
