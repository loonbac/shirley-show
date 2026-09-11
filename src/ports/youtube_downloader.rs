use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use crate::domain::youtube_url::YouTubeUrl;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YoutubeDownloadTarget {
    pub stage_id: String,
    pub staging_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedYoutubeAudio {
    pub title: String,
    pub duration_secs: u32,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum YoutubeDownloadError {
    #[error("La herramienta de descarga no está disponible en el servidor")]
    DownloaderNotConfigured,
    #[error("La descarga tardó demasiado tiempo y fue cancelada")]
    DownloadTimeout,
    #[error("No se pudo descargar el audio del video de YouTube")]
    DownloadFailed,
    #[error("El video supera el límite máximo de 20 minutos")]
    DurationExceeded,
    #[error("El audio descargado supera el límite máximo de 100 MiB")]
    FileSizeExceeded,
    #[error("El archivo de audio descargado no tiene un formato válido")]
    InvalidAudioOutput,
}

impl YoutubeDownloadError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DownloaderNotConfigured => "downloader_not_configured",
            Self::DownloadTimeout => "download_timeout",
            Self::DownloadFailed => "download_failed",
            Self::DurationExceeded => "duration_exceeded",
            Self::FileSizeExceeded => "file_size_exceeded",
            Self::InvalidAudioOutput => "invalid_audio_output",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::DownloaderNotConfigured => {
                "La herramienta de descarga no está disponible en el servidor"
            }
            Self::DownloadTimeout => "La descarga tardó demasiado tiempo y fue cancelada",
            Self::DownloadFailed => "No se pudo descargar el audio del video de YouTube",
            Self::DurationExceeded => "El video supera el límite máximo de 20 minutos",
            Self::FileSizeExceeded => "El audio descargado supera el límite máximo de 100 MiB",
            Self::InvalidAudioOutput => "El archivo de audio descargado no tiene un formato válido",
        }
    }
}

pub trait YoutubeDownloader: Send + Sync {
    fn download(
        &self,
        url: YouTubeUrl,
        target: YoutubeDownloadTarget,
    ) -> Pin<Box<dyn Future<Output = Result<DownloadedYoutubeAudio, YoutubeDownloadError>> + Send>>;
}
