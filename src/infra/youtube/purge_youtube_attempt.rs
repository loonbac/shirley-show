use std::fs;
use std::path::Path;

pub fn purge_youtube_attempt(staging_dir: &Path, stage_id: &str) {
    if !staging_dir.is_dir() {
        return;
    }

    let Ok(entries) = fs::read_dir(staging_dir) else {
        return;
    };

    let prefix = format!("{stage_id}.");
    let part_name = format!("{stage_id}.part");

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with(&prefix) || file_name == part_name {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
    }
}
