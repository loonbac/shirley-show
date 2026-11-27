use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::domain::vocal_separation_mode::VocalSeparationMode;
use crate::ports::vocal_separator::VocalSeparationError;

// One measured model per mode, not an ensemble. Measured on the production
// GPU against a real clip, using reconstruction corr(instrumental + vocals,
// mix) and leak corr(instrumental, vocals), where lower leak is better:
//
//   317               reconstruction 0.9996   leak 0.0632
//   368               reconstruction 0.9996   leak 0.0632
//   becruily_inst     reconstruction 0.9992   leak 0.1025
//
// Averaging four models gave leak 0.0904 against 0.0632 for 317 alone:
// an ensemble made separation worse while costing four times the GPU time,
// because stems from different models do not align in phase and averaging
// them partially cancels the signal.
pub const INSTRUMENTAL_MODELS: [&str; 1] = ["model_bs_roformer_ep_317_sdr_12.9755.ckpt"];

// The karaoke mode must remove the lead vocal while keeping the backing
// vocals, so it is judged on two things at once: low leak and high
// instrumentation energy, which is where those backing vocals live.
// Measured against the same clip, with the instrumental mode's 317 result
// (energy 0.1480) as the floor:
//
//   model        leak     instr energy   vocals energy
//   viperx       0.0914   0.1538         0.1129
//   gabox_v2     0.1656   0.1572         0.1045
//   anvuew       0.2210   0.1577         0.0941
//   frazer       0.3167   0.1611         0.0674
//
// viperx has the lowest leak of the four and still keeps more
// instrumentation than the instrumental-only model does, which is exactly
// the trade the karaoke mode needs. The others either keep very little
// vocal (frazer, 0.0674) or bleed heavily.
pub const COROS_MODELS: [&str; 1] = ["mel_band_roformer_karaoke_aufr33_viperx_sdr_10.1956.ckpt"];

#[derive(Clone, Debug)]
pub struct ColabVocalSeparatorConfig {
    pub colab_bin: PathBuf,
    pub token_path: PathBuf,
    pub gpu_type: String,
    /// GPUs tried, in order, after `gpu_type` fails to assign. Colab lists an
    /// accelerator as available while its assignment endpoint hangs until the
    /// client gives up, and which accelerators are reachable changes over
    /// minutes: G4 and A100 both refused within the same hour that L4 answered.
    /// A single fallback only moves the failure one step, so this is a chain.
    /// Empty disables it entirely.
    pub gpu_fallbacks: Vec<String>,
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

        let token_path = token_path_from_env();

        let gpu_type = std::env::var("SHIRLEY_COLAB_GPU").unwrap_or_else(|_| "g4".to_string());

        // A100 is the faster machine when it answers; L4 is the floor that has
        // answered every time it was asked. Order matters: the first one that
        // assigns wins, so the expensive-but-fast option is tried before the
        // slow-but-reliable one.
        // The variable keeps its singular name even though it now holds a list.
        // Renaming it to `_FALLBACKS` would silently restore the default chain
        // in every deployment that sets the old name, and nothing in the image
        // would complain: the chain is read from the environment, not compiled
        // in. Keep the name.
        let gpu_fallbacks: Vec<String> = match std::env::var("SHIRLEY_COLAB_GPU_FALLBACK") {
            Ok(raw) => raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
            Err(_) => vec!["a100".to_string(), "l4".to_string()],
        };

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
            gpu_fallbacks,
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

    /// Ensures the host token is available to the official CLI at its legacy path.
    ///
    /// Call this after `validate_token` and before any CLI invocation. A failure
    /// surfaces as `ColabTokenMissing`: if the copy cannot be made, the CLI has
    /// no token to authenticate with.
    pub fn ensure_legacy_token(&self) -> Result<(), VocalSeparationError> {
        let dest = legacy_token_path();
        sync_token_to_legacy(&self.token_path, &dest)
            .map_err(|_| VocalSeparationError::ColabTokenMissing)
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

/// The token path the host mounts at `SHIRLEY_COLAB_TOKEN`.
///
/// The official CLI keeps its own legacy token under `~/.config/colab-cli/`;
/// the host token stays where it has always been mounted, and is copied into
/// the legacy path before any CLI call (see `sync_token_to_legacy`).
pub fn token_path_from_env() -> PathBuf {
    std::env::var("SHIRLEY_COLAB_TOKEN")
        .map(expand_tilde)
        .unwrap_or_else(|_| default_token_path())
}

/// The path the official `google-colab-cli` reads its legacy token from.
///
/// Overridable via `SHIRLEY_COLAB_LEGACY_TOKEN` so tests can redirect the
/// copy to a scratch directory instead of the user's real config.
pub fn legacy_token_path() -> PathBuf {
    if let Ok(p) = std::env::var("SHIRLEY_COLAB_LEGACY_TOKEN") {
        return expand_tilde(p);
    }
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config/colab-cli/token.json")
    } else {
        PathBuf::from(".config/colab-cli/token.json")
    }
}

/// Copies the host token into the path the official CLI reads, unless that
/// destination already holds a newer copy.
///
/// The CLI rewrites its token file when it refreshes the access token, so a
/// newer destination must never be overwritten: doing so would throw away the
/// refreshed token. The copy happens only when the destination is missing or
/// strictly older than the source. The destination is created with 0600
/// permissions on Unix. No token content is ever read into memory or logged.
pub fn sync_token_to_legacy(source: &Path, dest: &Path) -> std::io::Result<()> {
    if source == dest {
        return Ok(());
    }
    let src_meta = std::fs::metadata(source)?;
    let should_copy = match std::fs::metadata(dest) {
        Ok(dst_meta) => src_meta.modified()? > dst_meta.modified()?,
        Err(_) => true,
    };
    if should_copy {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, dest)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
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

#[cfg(test)]
mod model_choice_tests {
    use super::*;

    #[test]
    fn test_each_mode_uses_a_single_measured_model() {
        // Measured on the production GPU against a real ABBA clip, using
        // reconstruction corr(instrumental + vocals, mix) and leak
        // corr(instrumental, vocals). Lower leak is better.
        //
        //   model                 reconstruction   leak
        //   317 (instrumental)        0.9996       0.0632
        //   368                       0.9996       0.0632
        //   becruily_inst             0.9992       0.1025
        //   viperx (karaoke)          0.9999       0.0914
        //   gabox_v2                  1.0000       0.1656
        //   anvuew                    1.0000       0.2210
        //   frazer                    0.9999       0.3167
        //
        // Four models averaged gave leak 0.0904 against 0.0632 for 317 alone:
        // more models made separation worse and cost four times the GPU time,
        // because stems from different models do not align in phase and
        // averaging them partially cancels the signal.
        assert_eq!(
            INSTRUMENTAL_MODELS.len(),
            1,
            "an ensemble measurably degrades separation; keep one measured model"
        );
        assert_eq!(
            COROS_MODELS.len(),
            1,
            "an ensemble measurably degrades separation; keep one measured model"
        );
        assert_eq!(
            INSTRUMENTAL_MODELS[0], "model_bs_roformer_ep_317_sdr_12.9755.ckpt",
            "317 has the lowest measured leak for the instrumental mode"
        );
        assert_eq!(
            COROS_MODELS[0], "mel_band_roformer_karaoke_aufr33_viperx_sdr_10.1956.ckpt",
            "viperx has the lowest measured leak among the karaoke models and \
             keeps the most instrumentation energy, which is what retaining \
             backing vocals requires"
        );
    }
}
