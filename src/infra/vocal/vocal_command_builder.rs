use std::path::Path;
use tokio::process::Command;

/// Output template used internally by the `karaoke-separator` wrapper: `NAME_(STEM)_MODEL`.
pub const CONTRACTUAL_OUTPUT_TEMPLATE: &str = "NAME_(STEM)_MODEL";

/// Concentrates the CLI invocation grammar for vocal separation subprocesses (`karaoke-separator`).
///
/// ### Observed Wrapper Contract (`/run/current-system/sw/bin/karaoke-separator`)
/// 1. Real syntax: `karaoke-separator <modelo> <cancion> [salida]`
/// 2. Positional argv:
///    - `$1`: model identifier (`gabox-invertido`, `frazer-becruily`, `anvuew`, `polarformer`).
///    - `$2`: input audio file path.
///    - `$3`: output directory path.
/// 3. Extra CLI flags (`--checkpoint`, `--config`, `--output-template`) DO NOT exist and are silently
///    ignored by the wrapper bash script.
/// 4. Output format is hardcoded as FLAC (`-of flac` / `output_format="flac"`).
/// 5. Output naming template is hardcoded as `NAME_(STEM)_MODEL`.
/// 6. PolarFormer model branch hardcodes checkpoint `$models_dir/model_bs_polarformer_float16.ckpt`
///    and config `$models_dir/model_bs_polarformer_float16.yaml` without flags.
///    Physical presence of these weights is guaranteed by local preflight checks in
///    `ProcessVocalSeparatorConfig::validate_polarformer_artifacts` before GPU acquisition.
#[derive(Debug)]
pub struct VocalCommandBuilder<'a> {
    binary: &'a Path,
    model: &'a str,
    source: &'a Path,
    output_dir: &'a Path,
    working_dir: Option<&'a Path>,
}

impl<'a> VocalCommandBuilder<'a> {
    pub fn new_chorus(
        binary: &'a Path,
        model: &'a str,
        source: &'a Path,
        output_dir: &'a Path,
    ) -> Self {
        Self {
            binary,
            model,
            source,
            output_dir,
            working_dir: None,
        }
    }

    pub fn new_instrumental(binary: &'a Path, source: &'a Path, output_dir: &'a Path) -> Self {
        Self {
            binary,
            model: "polarformer",
            source,
            output_dir,
            working_dir: None,
        }
    }

    pub fn current_dir(mut self, dir: &'a Path) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Builds the `tokio::process::Command` with exact argv `[modelo, input, output_dir]`.
    pub fn build(self) -> Command {
        let mut cmd = Command::new(self.binary);
        cmd.arg(self.model);
        cmd.arg(self.source);
        cmd.arg(self.output_dir);

        if let Some(cwd) = self.working_dir {
            cmd.current_dir(cwd);
        }
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        cmd
    }
}
