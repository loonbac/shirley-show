use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use crate::error::AppError;
pub use crate::ports::ticketing::ImageFormat;
use crate::ports::ticketing::PosterStorage;

const MAX_POSTER_SIZE_BYTES: usize = 10 * 1024 * 1024; // 10 MiB

pub struct LocalPosterStorage {
    posters_dir: PathBuf,
    trash_dir: PathBuf,
}

impl LocalPosterStorage {
    pub fn new(posters_dir: PathBuf) -> Self {
        let trash_dir = posters_dir.join(".trash");
        let _ = fs::create_dir_all(&posters_dir);
        let _ = fs::create_dir_all(&trash_dir);

        let storage = Self {
            posters_dir,
            trash_dir,
        };
        storage.reconcile_startup();
        storage
    }

    pub fn reconcile_startup(&self) -> usize {
        let mut cleaned = 0;
        // 1. Clean .trash dir
        if let Ok(entries) = fs::read_dir(&self.trash_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && fs::remove_file(&path).is_ok() {
                    cleaned += 1;
                }
            }
        }

        // 2. Clean .part files in posters_dir
        if let Ok(entries) = fs::read_dir(&self.posters_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_part = path.is_file()
                    && path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|name| name.ends_with(".part"));
                if is_part && fs::remove_file(&path).is_ok() {
                    cleaned += 1;
                }
            }
        }
        cleaned
    }

    pub fn reconcile_orphans(&self, active_posters: &[String]) -> usize {
        let mut cleaned = 0;
        let active_set: std::collections::HashSet<&str> =
            active_posters.iter().map(|s| s.as_str()).collect();

        if let Ok(entries) = fs::read_dir(&self.posters_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.starts_with('.') || name.ends_with(".part") {
                    continue;
                }
                let is_image = name.ends_with(".jpg")
                    || name.ends_with(".jpeg")
                    || name.ends_with(".png")
                    || name.ends_with(".webp");
                if is_image && !active_set.contains(name) && fs::remove_file(&path).is_ok() {
                    cleaned += 1;
                }
            }
        }
        cleaned
    }

    fn is_safe_filename(filename: &str) -> bool {
        !filename.is_empty()
            && !filename.contains('/')
            && !filename.contains('\\')
            && !filename.contains("..")
            && filename
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    }
}

impl PosterStorage for LocalPosterStorage {
    fn store_poster(&self, evento_id: &str, data: &[u8]) -> Result<String, AppError> {
        if data.len() > MAX_POSTER_SIZE_BYTES {
            return Err(AppError::payload_too_large(
                "poster_too_large",
                "El afiche excede el límite máximo de 10 MiB",
            ));
        }

        let format = ImageFormat::detect(data)?;
        let clean_event_id: String = evento_id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .collect();

        let filename = format!(
            "{}_{}.{}",
            clean_event_id,
            Uuid::new_v4(),
            format.extension()
        );
        let part_path = self.posters_dir.join(format!("{}.part", filename));
        let final_path = self.posters_dir.join(&filename);

        fs::write(&part_path, data)
            .map_err(|e| AppError::Internal(format!("Error escribiendo afiche temporal: {e}")))?;

        if let Err(e) = fs::rename(&part_path, &final_path) {
            let _ = fs::remove_file(&part_path);
            return Err(AppError::Internal(format!(
                "Error promocionando afiche a su ruta final: {e}"
            )));
        }

        Ok(filename)
    }

    fn get_poster(&self, filename: &str) -> Result<Option<(Vec<u8>, ImageFormat)>, AppError> {
        if !Self::is_safe_filename(filename) {
            return Ok(None);
        }

        let path = self.posters_dir.join(filename);
        if !path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&path)
            .map_err(|e| AppError::Internal(format!("Error leyendo afiche de disco: {e}")))?;

        let format = ImageFormat::detect(&bytes)?;
        Ok(Some((bytes, format)))
    }

    fn delete_poster(&self, filename: &str) -> Result<bool, AppError> {
        if !Self::is_safe_filename(filename) {
            return Ok(false);
        }

        let path = self.posters_dir.join(filename);
        if !path.exists() {
            return Ok(false);
        }

        // Try moving to trash first, or remove directly
        let trash_target = self
            .trash_dir
            .join(format!("{}_{}", Uuid::new_v4(), filename));
        if fs::rename(&path, &trash_target).is_ok() {
            let _ = fs::remove_file(&trash_target);
            Ok(true)
        } else {
            let removed = fs::remove_file(&path).is_ok();
            Ok(removed)
        }
    }
}
