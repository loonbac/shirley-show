use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::ports::vocal_separator::VocalSeparationError;

#[derive(Clone, Debug)]
pub struct ProcessVocalSeparatorConfig {
    pub vocal_sep_bin: Option<PathBuf>,
    pub ffprobe_bin: PathBuf,
    pub ffmpeg_bin: PathBuf,
    pub polarformer_checkpoint: Option<PathBuf>,
    pub polarformer_config: Option<PathBuf>,
    pub work_root: PathBuf,
    pub gpu: Arc<Semaphore>,
}

impl ProcessVocalSeparatorConfig {
    /// Builds configuration from environment variables and defaults.
    pub fn from_env(work_root: PathBuf, gpu: Arc<Semaphore>) -> Self {
        let vocal_sep_bin = std::env::var("VOCAL_SEP_BIN")
            .ok()
            .map(PathBuf::from);

        let ffprobe_bin = std::env::var("FFPROBE_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("ffprobe"));

        let ffmpeg_bin = std::env::var("FFMPEG_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("ffmpeg"));

        let polarformer_checkpoint = std::env::var("POLARFORMER_CHECKPOINT")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                dirs_fallback_models_dir().map(|d| d.join("model_bs_polarformer_float16.ckpt"))
            });

        let polarformer_config = std::env::var("POLARFORMER_CONFIG")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                dirs_fallback_models_dir().map(|d| d.join("model_bs_polarformer_float16.yaml"))
            });

        Self {
            vocal_sep_bin,
            ffprobe_bin,
            ffmpeg_bin,
            polarformer_checkpoint,
            polarformer_config,
            work_root,
            gpu,
        }
    }

    /// Resolves the vocal separation executable without shell evaluation.
    ///
    /// If an explicit binary path was configured or set via `VOCAL_SEP_BIN`, it MUST exist,
    /// be a regular file, and be executable. An invalid explicit binary NEVER silently falls back to PATH.
    /// In absence of an explicit binary, resolves `karaoke-separator` from system `PATH`.
    pub fn resolve_vocal_binary(&self) -> Result<PathBuf, VocalSeparationError> {
        let explicit = self.vocal_sep_bin.clone().or_else(|| {
            std::env::var("VOCAL_SEP_BIN").ok().map(PathBuf::from)
        });

        if let Some(path) = explicit {
            if is_executable_file(&path) {
                Ok(path)
            } else {
                Err(VocalSeparationError::SeparatorNotConfigured)
            }
        } else {
            find_executable_in_path("karaoke-separator")
                .ok_or(VocalSeparationError::SeparatorNotConfigured)
        }
    }

    /// Validates physical existence and readability of PolarFormer checkpoint and config.
    /// Absence indicates `instrumental_unavailable` and does NOT acquire GPU.
    pub fn validate_polarformer_artifacts(&self) -> Result<(&Path, &Path), VocalSeparationError> {
        let ckpt = self
            .polarformer_checkpoint
            .as_deref()
            .ok_or(VocalSeparationError::InstrumentalUnavailable)?;
        let conf = self
            .polarformer_config
            .as_deref()
            .ok_or(VocalSeparationError::InstrumentalUnavailable)?;

        if ckpt.is_file() && conf.is_file() {
            Ok((ckpt, conf))
        } else {
            Err(VocalSeparationError::InstrumentalUnavailable)
        }
    }
}

fn dirs_fallback_models_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let models_dir = PathBuf::from(home)
            .join("Proyectos")
            .join("separador-vocal")
            .join("modelos");
        if models_dir.is_dir() {
            return Some(models_dir);
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    if let Ok(meta) = std::fs::metadata(path) {
        meta.is_file() && (meta.permissions().mode() & 0o111 != 0)
    } else {
        false
    }
}

fn find_executable_in_path(name: &str) -> Option<PathBuf> {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if is_executable_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}
