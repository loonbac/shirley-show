use std::path::{Path, PathBuf};

use crate::ports::vocal_separator::VocalSeparationError;

/// Resolves the non-vocal (karaoke/instrumental) WAV stem from a model's dedicated output directory.
///
/// Enforces strict invariants:
/// 1. Only regular `.wav` files are accepted (no symlinks, no subdirectories, no extra files).
/// 2. Every file must contain the model token.
/// 3. Exactly one complementary vocal stem (containing `vocal` case-insensitively) must be present.
/// 4. Exactly one non-vocal stem (not containing `vocal` case-insensitively) must be present.
/// 5. Missing, extra, duplicate, or ambiguous files cause immediate rejection.
pub fn resolve_stem(dir: &Path, model_token: &str) -> Result<PathBuf, VocalSeparationError> {
    resolve_stem_with_tokens(dir, &[model_token])
}

pub fn resolve_stem_with_tokens(
    dir: &Path,
    model_tokens: &[&str],
) -> Result<PathBuf, VocalSeparationError> {
    if !dir.is_dir() {
        return Err(VocalSeparationError::InvalidSeparatorOutput);
    }

    let entries = std::fs::read_dir(dir).map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?;

    let mut non_vocal_stem: Option<PathBuf> = None;
    let mut vocal_stem_found = false;
    let mut total_files = 0usize;

    for entry in entries {
        let entry = entry.map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?;
        let path = entry.path();

        // Check symlinks explicitly
        let sym_meta = std::fs::symlink_metadata(&path)
            .map_err(|_| VocalSeparationError::InvalidSeparatorOutput)?;
        if sym_meta.is_symlink() {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        if !path.is_file() {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        // Must be a .wav file
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase());
        if ext.as_deref() != Some("wav") {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(VocalSeparationError::InvalidSeparatorOutput)?;
        let filename_lower = filename.to_ascii_lowercase();

        // Must contain at least one expected model token
        let has_model_token = model_tokens
            .iter()
            .any(|t| filename_lower.contains(&t.to_ascii_lowercase()));
        if !has_model_token {
            return Err(VocalSeparationError::InvalidSeparatorOutput);
        }

        total_files += 1;

        if filename_lower.contains("vocal") {
            if vocal_stem_found {
                // Duplicate vocal stem
                return Err(VocalSeparationError::InvalidSeparatorOutput);
            }
            vocal_stem_found = true;
        } else {
            if non_vocal_stem.is_some() {
                // Duplicate non-vocal stem
                return Err(VocalSeparationError::InvalidSeparatorOutput);
            }
            non_vocal_stem = Some(path);
        }
    }

    if total_files != 2 || !vocal_stem_found {
        return Err(VocalSeparationError::InvalidSeparatorOutput);
    }

    non_vocal_stem.ok_or(VocalSeparationError::InvalidSeparatorOutput)
}
