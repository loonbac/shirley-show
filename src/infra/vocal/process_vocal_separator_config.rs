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
    pub work_root: PathBuf,
    pub gpu: Arc<Semaphore>,
}

impl ProcessVocalSeparatorConfig {
    /// Builds configuration from environment variables and defaults.
    pub fn from_env(work_root: PathBuf, gpu: Arc<Semaphore>) -> Self {
        let vocal_sep_bin = std::env::var("VOCAL_SEP_BIN").ok().map(PathBuf::from);

        let ffprobe_bin = std::env::var("FFPROBE_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("ffprobe"));

        let ffmpeg_bin = std::env::var("FFMPEG_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("ffmpeg"));

        // NOTE on POLARFORMER_CHECKPOINT and POLARFORMER_CONFIG:
        // We deliberately DO NOT read `POLARFORMER_CHECKPOINT` or `POLARFORMER_CONFIG` overrides.
        // The real wrapper `/run/current-system/sw/bin/karaoke-separator` hardcodes:
        //   models_dir="$HOME/Proyectos/separador-vocal/modelos"
        // and passes exclusively:
        //   "$models_dir/model_bs_polarformer_float16.ckpt"
        //   "$models_dir/model_bs_polarformer_float16.yaml"
        // without accepting CLI flags or environment overrides for weights.
        // Permitting overrides in Shirley Show would create a false-positive vulnerability
        // where preflight passes on alternate files, acquires the GPU semaphore permit,
        // and then the wrapper fails loading its internal hardcoded paths.
        // PolarFormer availability is validated strictly against the wrapper's contractual paths.

        Self {
            vocal_sep_bin,
            ffprobe_bin,
            ffmpeg_bin,
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
        let explicit = self
            .vocal_sep_bin
            .clone()
            .or_else(|| std::env::var("VOCAL_SEP_BIN").ok().map(PathBuf::from));

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

    /// Returns the exact contractual PolarFormer paths hardcoded in `/run/current-system/sw/bin/karaoke-separator`:
    /// - `$HOME/Proyectos/separador-vocal/modelos/model_bs_polarformer_float16.ckpt`
    /// - `$HOME/Proyectos/separador-vocal/modelos/model_bs_polarformer_float16.yaml`
    pub fn contractual_polarformer_paths() -> Result<(PathBuf, PathBuf), VocalSeparationError> {
        let home =
            std::env::var("HOME").map_err(|_| VocalSeparationError::InstrumentalUnavailable)?;
        let models_dir = PathBuf::from(home)
            .join("Proyectos")
            .join("separador-vocal")
            .join("modelos");
        let ckpt = models_dir.join("model_bs_polarformer_float16.ckpt");
        let conf = models_dir.join("model_bs_polarformer_float16.yaml");
        Ok((ckpt, conf))
    }

    /// Pure validation function testing physical existence, regular file status, and readability
    /// by actively opening both PolarFormer checkpoint and config artifacts with `File::open`.
    pub fn validate_polarformer_artifacts_at(
        ckpt: &Path,
        conf: &Path,
    ) -> Result<(), VocalSeparationError> {
        let ckpt_file =
            std::fs::File::open(ckpt).map_err(|_| VocalSeparationError::InstrumentalUnavailable)?;
        let conf_file =
            std::fs::File::open(conf).map_err(|_| VocalSeparationError::InstrumentalUnavailable)?;

        let ckpt_meta = ckpt_file
            .metadata()
            .map_err(|_| VocalSeparationError::InstrumentalUnavailable)?;
        let conf_meta = conf_file
            .metadata()
            .map_err(|_| VocalSeparationError::InstrumentalUnavailable)?;

        if ckpt_meta.is_file() && conf_meta.is_file() {
            Ok(())
        } else {
            Err(VocalSeparationError::InstrumentalUnavailable)
        }
    }

    /// Validates physical existence and readability of the exact contractual PolarFormer
    /// checkpoint and config consumed by `/run/current-system/sw/bin/karaoke-separator`.
    /// Actively opens both files to verify read permissions.
    /// Absence or unreadability indicates `instrumental_unavailable` and does NOT acquire GPU.
    pub fn validate_polarformer_artifacts(
        &self,
    ) -> Result<(PathBuf, PathBuf), VocalSeparationError> {
        let (ckpt, conf) = Self::contractual_polarformer_paths()?;
        Self::validate_polarformer_artifacts_at(&ckpt, &conf)?;
        Ok((ckpt, conf))
    }
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
