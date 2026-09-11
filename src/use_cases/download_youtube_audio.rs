use std::sync::Arc;

use crate::domain::youtube_url::{YouTubeUrl, YouTubeUrlError};
use crate::error::AppError;
use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;
use crate::infra::filesystem::staging_guard::StagingFileGuard;
use crate::ports::youtube_downloader::{
    DownloadedYoutubeAudio, YoutubeDownloadError, YoutubeDownloadTarget, YoutubeDownloader,
};
use crate::state::staged_audio_registry::StagedAudioRegistry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadYouTubeAudioOutput {
    pub staged_audio_id: String,
    pub title: String,
    pub duration_secs: u32,
    pub size_bytes: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum DownloadYouTubeAudioError {
    #[error("{0}")]
    Url(#[from] YouTubeUrlError),
    #[error("{0}")]
    Download(#[from] YoutubeDownloadError),
    #[error("{0}")]
    Storage(#[from] AppError),
}

impl DownloadYouTubeAudioError {
    pub fn code(&self) -> &str {
        match self {
            Self::Url(e) => e.code(),
            Self::Download(e) => e.code(),
            Self::Storage(e) => match e {
                AppError::NotFound { code, .. } => code,
                AppError::Validation { code, .. } => code,
                AppError::BadRequest { code, .. } => code,
                AppError::PayloadTooLarge { code, .. } => code,
                AppError::UnsupportedMediaType { code, .. } => code,
                AppError::Conflict { code, .. } => code,
                AppError::Database(_) => "database_error",
                AppError::Internal(_) => "internal_storage_error",
            },
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Url(e) => e.message(),
            Self::Download(e) => e.message(),
            Self::Storage(e) => match e {
                AppError::NotFound { message, .. } => message.as_str(),
                AppError::Validation { message, .. } => message.as_str(),
                AppError::BadRequest { message, .. } => message.as_str(),
                AppError::PayloadTooLarge { message, .. } => message.as_str(),
                AppError::UnsupportedMediaType { message, .. } => message.as_str(),
                AppError::Conflict { message, .. } => message.as_str(),
                AppError::Database(_) => "Error de base de datos",
                AppError::Internal(_) => "Error interno de almacenamiento",
            },
        }
    }
}

pub struct DownloadYouTubeAudio {
    downloader: Arc<dyn YoutubeDownloader>,
    storage: Arc<LocalAudioStorage>,
    registry: Arc<StagedAudioRegistry>,
}

impl DownloadYouTubeAudio {
    pub fn new(
        downloader: Arc<dyn YoutubeDownloader>,
        storage: Arc<LocalAudioStorage>,
        registry: Arc<StagedAudioRegistry>,
    ) -> Self {
        Self {
            downloader,
            storage,
            registry,
        }
    }

    pub async fn execute(
        &self,
        raw_url: &str,
        requested_title: Option<&str>,
    ) -> Result<DownloadYouTubeAudioOutput, DownloadYouTubeAudioError> {
        // 1. Parse and validate YouTubeUrl before spawning any process
        let youtube_url = YouTubeUrl::parse(raw_url)?;

        // 2. Reserve a stage_id and guard the staging file
        let (stage_id, stage_path) = self.storage.create_staging_file()?;
        let guard =
            StagingFileGuard::new(self.storage.clone(), stage_id.clone(), stage_path.clone());

        // 3. Invoke downloader port with canonical URL and internal target
        let target = YoutubeDownloadTarget {
            stage_id: stage_id.clone(),
            staging_dir: self.storage.staging_dir().to_path_buf(),
        };

        let downloaded: DownloadedYoutubeAudio = self
            .downloader
            .download(youtube_url.clone(), target)
            .await?;

        // 4. Resolve and sanitize title:
        //    requested title (if non-empty) > metadata title (if non-empty) > 11-character video ID
        let final_title =
            self.resolve_title(requested_title, &downloaded.title, youtube_url.video_id());

        // 5. Transfer cleanup responsibility to registry by defusing the staging guard
        guard.defuse();

        let descriptor = self.registry.register(
            stage_id,
            stage_path,
            final_title,
            downloaded.duration_secs,
            downloaded.size_bytes,
        );

        Ok(DownloadYouTubeAudioOutput {
            staged_audio_id: descriptor.staged_audio_id,
            title: descriptor.title,
            duration_secs: descriptor.duration_secs,
            size_bytes: descriptor.size_bytes,
        })
    }

    fn resolve_title(
        &self,
        requested_title: Option<&str>,
        metadata_title: &str,
        video_id: &str,
    ) -> String {
        if let Some(req) = requested_title
            && let Some(clean) = sanitize_title(req)
        {
            return clean;
        }

        if let Some(clean_meta) = sanitize_title(metadata_title) {
            return clean_meta;
        }

        video_id.to_string()
    }
}

fn sanitize_title(raw: &str) -> Option<String> {
    // Strip control characters and collapse multiple whitespace
    let cleaned: String = raw
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();

    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}
