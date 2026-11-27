use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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

/// The accelerators `colab new --gpu` accepts, exactly as the CLI spells them.
///
/// The list is not a preference order; it is the contract printed by the CLI
/// itself: `unsupported value '14'. Supported: T4, L4, G4, H100, A100.`
/// Asking for anything else burns an attempt on a request that cannot succeed,
/// which is what a configured `14` did. The attempt order is
/// [`PERFORMANCE_GPU_ORDER`], which is a different thing.
pub const SUPPORTED_COLAB_GPUS: [&str; 5] = ["T4", "L4", "G4", "H100", "A100"];

/// Every accelerator the CLI accepts, fastest first.
///
/// Performance is the ordering rule, not the configuration: a separation runs
/// powerful models, so the device with the most throughput finishes it in the
/// least time, and asking a slower machine first would make every request wait
/// for it while a faster one was free.
///
/// * `G4` (RTX PRO 6000 Blackwell, 102 GB) is the measured fastest device in
///   this codebase.
/// * `A100` is the device this project measured its models on; it is slower than
///   G4 and faster than the rest.
/// * `L4` is the reliable floor: it is the one that has answered every time it
///   was asked, at a real cost in throughput.
/// * `T4` is last and is not in the default chain. A separation on it does not
///   fit the pipeline reserve, so it is only ever reached when the
///   configuration names it explicitly and everything faster has refused.
///
/// `H100` is deliberately absent: the CLI accepts the spelling, but this account
/// cannot be assigned one. Every H100 request in production answered HTTP 400,
/// so a slot in the ranking was a slot spent on an impossible assignment. See
/// [`GPU_UNAVAILABLE_FOR_ACCOUNT`].
pub const PERFORMANCE_GPU_ORDER: [&str; 4] = ["G4", "A100", "L4", "T4"];

/// Accelerators the CLI accepts but this account cannot be assigned.
///
/// The Colab UI greys `H100` out for this account and production proved it: every
/// H100 assignment returned HTTP 400. The user's instruction is to discard it,
/// so it is filtered exactly like a value the CLI rejects — configured or not, it
/// never becomes an attempt. Keeping the name here instead of deleting it
/// documents the difference between "the CLI does not know it" and "this account
/// cannot use it".
pub const GPU_UNAVAILABLE_FOR_ACCOUNT: [&str; 1] = ["H100"];

/// Splits the request budget in two: GPU assignment may not touch the half that
/// upload, exec and download still need.
///
/// The old arithmetic handed the first accelerator `request.timeout - elapsed`,
/// so one hanging device spent the whole budget and the fallbacks never ran.
pub const PIPELINE_BUDGET_DIVISOR: u32 = 2;

/// Bound for a single `colab new` attempt.
///
/// A short bound is the point, and 90s was not short enough. Measured in
/// production, one attempt was: `POST` answered 503 after ~30s, a 10s wait, a
/// second `POST` answered 503 after ~30s, a 20s wait — and only then the 90s cap
/// fired. None of those extra seconds bought an assignment the CLI was not
/// already retrying on its own, they only made every busy accelerator cost a
/// minute and a half.
///
/// The cap therefore covers one assign POST plus margin. A busy best GPU is
/// abandoned quickly, stopped, and retried in the next round, which is where the
/// speed comes from: the rotation reaches a free device sooner instead of
/// queueing behind a slow one.
pub const GPU_ATTEMPT_TIMEOUT_DEFAULT: Duration = Duration::from_secs(40);

/// Below this much usable assignment budget an attempt is not worth starting.
pub const MIN_USEFUL_GPU_ATTEMPT: Duration = Duration::from_secs(2);

/// How many full passes over the chain are made while the budget lasts.
///
/// One pass through a four-GPU chain is what a single 503 burst needs; a second
/// and later pass covers the case where every accelerator refuses for a moment
/// and the best one frees up a minute later. Each pass starts again at the best
/// device, so a GPU that was busy is never permanently demoted. The count is
/// capped so an instant refusal cannot spin the CLI in a hot loop.
pub const GPU_MAX_ROUNDS_DEFAULT: u32 = 4;

/// The chain used when the configuration names no usable accelerator.
///
/// Same order as the documented defaults: the fastest measured option first,
/// then A100, then the measured floor. `T4` is deliberately absent, and `H100`
/// is absent because this account cannot assign one.
pub const DEFAULT_GPU_CHAIN: [&str; 3] = ["G4", "A100", "L4"];

/// Canonical, uppercase name of a configured accelerator, or `None` when the
/// CLI does not accept it.
///
/// Configured values are lowercase by convention (`g4`); the CLI only accepts
/// uppercase. Normalizing here is what keeps the command boundary free of
/// string juggling and skips unusable values instead of attempting them.
pub fn normalize_gpu(value: &str) -> Option<String> {
    let upper = value.trim().to_ascii_uppercase();
    if upper.is_empty() {
        return None;
    }
    SUPPORTED_COLAB_GPUS
        .iter()
        .find(|gpu| **gpu == upper)
        .map(|gpu| gpu.to_string())
}

/// Whether this account can really assign `value`, as opposed to whether the CLI
/// accepts the spelling.
///
/// `normalize_gpu` answers the CLI's question; this answers the account's. `H100`
/// is the case that separates them: `colab new --gpu H100` is a valid command and
/// a guaranteed HTTP 400 here, so configured or not it is dropped before it can
/// spend an attempt.
pub fn gpu_is_supported(value: &str) -> bool {
    match normalize_gpu(value) {
        Some(gpu) => !GPU_UNAVAILABLE_FOR_ACCOUNT.contains(&gpu.as_str()),
        None => false,
    }
}

/// The accelerators to attempt, ALWAYS ordered by performance.
///
/// The configuration decides WHICH devices are eligible; it never decides the
/// order. Production still carries `SHIRLEY_COLAB_GPU=a100` from an old
/// mitigation, and honouring that order put the third-fastest device first: a
/// separation then waited for A100 capacity while G4 sat free, which is
/// the slowness the user reported.
///
/// The eligible set is the normalized configured values plus the fast defaults,
/// so a busy configured pair is never the whole rotation: once the configured
/// accelerators refuse, the remaining supported ones are tried too. Values the
/// CLI does not accept (`14` was one) are dropped instead of attempted, and so
/// are values the CLI accepts but this account cannot assign (`H100`). T4 stays
/// at the end of [`PERFORMANCE_GPU_ORDER`], reachable only when the configuration
/// names it.
pub fn gpu_candidates(primary: &str, fallbacks: &[String]) -> Vec<String> {
    let mut configured: Vec<String> = Vec::new();
    for raw in std::iter::once(primary).chain(fallbacks.iter().map(String::as_str)) {
        if let Some(gpu) = normalize_gpu(raw) {
            // `H100` is dropped exactly like an invalid value: the CLI knows the
            // name, this account cannot use it, and attempting it wastes the
            // attempt that a real device could have used.
            if !GPU_UNAVAILABLE_FOR_ACCOUNT.contains(&gpu.as_str()) && !configured.contains(&gpu)
            {
                configured.push(gpu);
            }
        }
    }

    // Nothing configured is usable: the documented chain is used instead of
    // failing a paid request on a configuration typo.
    if configured.is_empty() {
        return DEFAULT_GPU_CHAIN.iter().map(|g| g.to_string()).collect();
    }

    let mut eligible: Vec<String> = configured;
    for gpu in DEFAULT_GPU_CHAIN {
        if !eligible.iter().any(|e| e == gpu) {
            eligible.push(gpu.to_string());
        }
    }

    // Rank by throughput. `eligible` only ever holds names from
    // `PERFORMANCE_GPU_ORDER`, so nothing is lost here.
    PERFORMANCE_GPU_ORDER
        .iter()
        .filter(|gpu| eligible.iter().any(|e| e == *gpu))
        .map(|gpu| gpu.to_string())
        .collect()
}

/// The chain the separator actually rotates through, fastest first.
pub fn gpu_chain(primary: &str, fallbacks: &[String]) -> Vec<String> {
    gpu_candidates(primary, fallbacks)
}

/// Part of the remaining request budget reserved for the separation pipeline.
///
/// `pipeline_reserve` plus [`gpu_assign_window`] is the whole budget, so the
/// upload/exec/download arc can never be starved by accelerator retries. The
/// reserve is taken from the budget available when assignment STARTS and is not
/// recomputed per attempt: `remaining - remaining/2` re-halved whatever was left,
/// so with a 4 x 3 x 90s cap the rotation could spend nearly the whole budget
/// and leave the pipeline seconds.
pub fn pipeline_reserve(budget: Duration) -> Duration {
    budget / PIPELINE_BUDGET_DIVISOR
}

/// The fixed window that GPU assignment may spend, computed once per request
/// from the budget available when assignment begins.
///
/// Every attempt is bounded by what is left of THIS window, so the reserve for
/// upload/exec/download never shrinks with each retry.
pub fn gpu_assign_window(budget: Duration) -> Duration {
    budget.saturating_sub(pipeline_reserve(budget))
}

/// Part of a budget that GPU assignment may spend.
///
/// Kept as the name most call sites want; it is [`gpu_assign_window`] and is
/// likewise meant to be called once, not per attempt.
pub fn gpu_start_budget(budget: Duration) -> Duration {
    gpu_assign_window(budget)
}

/// Bound for the next `colab new` given the window still left, or zero when the
/// window cannot pay for a meaningful attempt.
///
/// Zero is deliberate instead of a tiny timeout: firing a request that is
/// guaranteed to be cut off only wastes the last seconds the pipeline needs.
pub fn gpu_attempt_timeout(remaining_window: Duration, attempt_cap: Duration) -> Duration {
    if remaining_window < MIN_USEFUL_GPU_ATTEMPT {
        return Duration::ZERO;
    }
    attempt_cap.min(remaining_window)
}

/// The timeout a separation request may use: the configured value, or the
/// caller's real remaining deadline when it is tighter.
///
/// This is the root cause of the 502s-vs-300s mismatch in production. The agent
/// turn cuts every request at `separation_timeout` (600s), but the Colab
/// separator configured itself from `SHIRLEY_COLAB_TIMEOUT_SECS`, whose default
/// is 1800s and which the deployment never set. The assignment window is half of
/// the request budget, so it was computed from 1800s and became ~502s: the
/// accelerator rotation was still running when the turn cancelled the request at
/// ~600s, and upload/exec/download never got a single second.
///
/// Deriving the request budget from the deadline that will really cut it — and
/// keeping the fixed pipeline reserve — is what stops that.
pub fn request_timeout_for(configured: Duration, deadline: Option<Duration>) -> Duration {
    match deadline {
        Some(left) => configured.min(left),
        None => configured,
    }
}

/// The value to hand `colab exec --timeout` for a step that still has `budget`.
///
/// The official CLI does not wait for the process to finish: it fails the call
/// when the kernel prints nothing for `--timeout` seconds, and its default is
/// 30. That default is shorter than a real separation spends silent — the
/// remote script installs packages and loads the separation models before it
/// prints anything — so production died at 30s with empty stdout and this on
/// stderr:
///
/// ```text
/// TimeoutError: Timeout waiting for output: no message from the kernel for
/// 30s. Use --timeout to allow longer silences.
/// ```
///
/// The kernel silence allowed is therefore the whole budget the step still has,
/// in whole seconds rounded up, so our own tokio timeout remains the only bound
/// that can cut a healthy separation short. `--timeout 0` means "no limit" in
/// the CLI and is never emitted: that would remove the bound instead of
/// raising it.
pub fn cli_silence_timeout_secs(budget: Duration) -> u64 {
    let rounded_up = budget.as_secs() + u64::from(budget.subsec_nanos() > 0);
    rounded_up.max(1)
}

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
    /// How long one `colab new` may take before the chain rotates.
    pub gpu_attempt_timeout: Duration,
    /// How many full passes over the chain are allowed while budget remains.
    pub gpu_max_rounds: u32,
    /// Optional cap on the GPU ASSIGNMENT window only.
    ///
    /// The fallback composition uses it to give Colab a short chance to hand
    /// over a GPU without shortening the pipeline of a GPU that did assign.
    /// `None` is the historical behaviour.
    pub assignment_budget_cap: Option<Duration>,
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
    /// Optional mirror of the one-line failure diagnostics that are always
    /// written to stderr.
    ///
    /// The product error deliberately hides stderr from the user, so the only
    /// channel is the process log; production is read with `docker logs` and a
    /// test cannot observe that stream. This sink lets a test assert the same
    /// line without changing what production does: the `eprintln!` is
    /// unconditional, and the sink only observes it. `None` (the default) adds
    /// nothing.
    pub diagnostic_sink: Option<Arc<Mutex<Vec<String>>>>,
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
        // answered every time it was asked. These values decide WHICH
        // accelerators are eligible; the attempt order is always
        // `PERFORMANCE_GPU_ORDER`, fastest first, so a deployment that still
        // carries `SHIRLEY_COLAB_GPU=a100` from an old mitigation can never push
        // A100 ahead of G4.
        //
        // The variable keeps its singular name even though it now holds a list.
        // Renaming it to `_FALLBACKS` would silently restore the default chain
        // in every deployment that sets the old name, and nothing in the image
        // would complain: the chain is read from the environment, not compiled
        // in. Keep the name.
        //
        // A value this account cannot use is treated as unset rather than as an
        // empty chain: production carries `14`, which the CLI rejects with
        // `unsupported value '14'. Supported: T4, L4, G4, H100, A100.`, and a
        // deployment may carry `h100`, which the CLI accepts but always answers
        // with HTTP 400. Either way, keeping it would leave the primary as the
        // only option and turn one 503 into a failed, already-paid request.
        let gpu_fallbacks: Vec<String> = match std::env::var("SHIRLEY_COLAB_GPU_FALLBACK") {
            Ok(raw) => {
                let parsed: Vec<String> = raw
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
                if !parsed.is_empty() && parsed.iter().all(|v| !gpu_is_supported(v)) {
                    default_gpu_fallbacks()
                } else {
                    parsed
                }
            }
            Err(_) => default_gpu_fallbacks(),
        };

        let gpu_attempt_timeout = GPU_ATTEMPT_TIMEOUT_DEFAULT;
        let gpu_max_rounds = GPU_MAX_ROUNDS_DEFAULT;

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
            gpu_attempt_timeout,
            gpu_max_rounds,
            assignment_budget_cap: None,
            overlap,
            batch_size,
            timeout_secs,
            work_dir,
            ffprobe_bin,
            openssl_bin,
            gpu,
            diagnostic_sink: None,
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

/// The fallbacks used when the variable is unset or names nothing usable.
fn default_gpu_fallbacks() -> Vec<String> {
    vec!["a100".to_string(), "l4".to_string()]
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
