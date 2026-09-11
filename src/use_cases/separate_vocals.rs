use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::domain::show_item::ShowItemPayload;
use crate::domain::vocal_separation_mode::VocalSeparationMode;
use crate::error::AppError;
use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;
use crate::infra::filesystem::staging_guard::StagingFileGuard;
use crate::ports::show_item_reader::ShowItemReader;
use crate::ports::vocal_separator::{VocalSeparationError, VocalSeparationRequest, VocalSeparator};
use crate::state::staged_audio_lease_guard::StagedAudioLeaseGuard;
use crate::state::staged_audio_registry::{StagedAudioRegistry, StagedAudioRegistryError};
use crate::use_cases::download_youtube_audio::{DownloadYouTubeAudio, DownloadYouTubeAudioError};

pub const DEFAULT_SEPARATION_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VocalAudioSource {
    StagedAudioId(String),
    TrackId(String),
    YoutubeUrl(String),
}

impl VocalAudioSource {
    /// Resolves audio source prioritizing `staged_audio_id` -> `track_id` -> `youtube_url`.
    /// Returns `SourceRequired` if none of the options contain a non-empty string.
    pub fn resolve(
        staged_audio_id: Option<String>,
        track_id: Option<String>,
        youtube_url: Option<String>,
    ) -> Result<Self, SeparateVocalsError> {
        if let Some(s) = staged_audio_id.filter(|s| !s.trim().is_empty()) {
            Ok(Self::StagedAudioId(s.trim().to_string()))
        } else if let Some(t) = track_id.filter(|t| !t.trim().is_empty()) {
            Ok(Self::TrackId(t.trim().to_string()))
        } else if let Some(u) = youtube_url.filter(|u| !u.trim().is_empty()) {
            Ok(Self::YoutubeUrl(u.trim().to_string()))
        } else {
            Err(SeparateVocalsError::SourceRequired)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SeparateVocalsInput {
    pub source: VocalAudioSource,
    pub mode: VocalSeparationMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparateVocalsOutput {
    pub staged_audio_id: String,
    pub mode: VocalSeparationMode,
    pub title: String,
    pub duration_secs: u32,
    pub size_bytes: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum SeparateVocalsError {
    #[error("Se requiere al menos una fuente de audio para separar")]
    SourceRequired,

    #[error("Archivo de audio temporal no encontrado")]
    StagedAudioNotFound,

    #[error("El archivo de audio ya fue consumido")]
    AudioAlreadyConsumed,

    #[error("El archivo de audio se encuentra en uso")]
    StagedAudioInUse,

    #[error("Pista no encontrada")]
    TrackNotFound,

    #[error("El elemento seleccionado no es una pista de audio")]
    SourceNotTrack,

    #[error("El archivo de audio no se encuentra disponible en disco")]
    AudioUnavailable,

    #[error(transparent)]
    VocalSeparation(#[from] VocalSeparationError),

    #[error(transparent)]
    YouTubeDownload(#[from] DownloadYouTubeAudioError),

    #[error("Error de base de datos o almacenamiento: {0}")]
    Storage(String),

    #[error("Error en el registro temporal: {0}")]
    Registry(#[from] StagedAudioRegistryError),
}

impl SeparateVocalsError {
    pub fn code(&self) -> &str {
        match self {
            Self::SourceRequired => "source_required",
            Self::StagedAudioNotFound => "staged_audio_not_found",
            Self::AudioAlreadyConsumed => "audio_already_consumed",
            Self::StagedAudioInUse => "staged_audio_in_use",
            Self::TrackNotFound => "track_not_found",
            Self::SourceNotTrack => "source_not_track",
            Self::AudioUnavailable => "audio_unavailable",
            Self::VocalSeparation(e) => e.code(),
            Self::YouTubeDownload(e) => e.code(),
            Self::Storage(_) => "storage_error",
            Self::Registry(e) => e.code(),
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::SourceRequired => "Se requiere al menos una fuente de audio para separar",
            Self::StagedAudioNotFound => "Archivo de audio temporal no encontrado",
            Self::AudioAlreadyConsumed => "El archivo de audio ya fue consumido",
            Self::StagedAudioInUse => "El archivo de audio se encuentra en uso",
            Self::TrackNotFound => "Pista no encontrada",
            Self::SourceNotTrack => "El elemento seleccionado no es una pista de audio",
            Self::AudioUnavailable => "El archivo de audio no se encuentra disponible en disco",
            Self::VocalSeparation(e) => e.message(),
            Self::YouTubeDownload(e) => e.message(),
            Self::Storage(msg) => msg.as_str(),
            Self::Registry(e) => e.message(),
        }
    }
}

impl From<AppError> for SeparateVocalsError {
    fn from(err: AppError) -> Self {
        Self::Storage(err.to_string())
    }
}

pub struct SeparateVocals<R: ShowItemReader> {
    separator: Arc<dyn VocalSeparator>,
    storage: Arc<LocalAudioStorage>,
    registry: Arc<StagedAudioRegistry>,
    item_reader: Arc<R>,
    download_youtube_audio: Arc<DownloadYouTubeAudio>,
    timeout: Duration,
}

struct ResolvedAudioSource {
    source_path: PathBuf,
    source_title: String,
    lease_guard: Option<StagedAudioLeaseGuard>,
}

impl<R: ShowItemReader> SeparateVocals<R> {
    pub fn new(
        separator: Arc<dyn VocalSeparator>,
        storage: Arc<LocalAudioStorage>,
        registry: Arc<StagedAudioRegistry>,
        item_reader: Arc<R>,
        download_youtube_audio: Arc<DownloadYouTubeAudio>,
    ) -> Self {
        Self {
            separator,
            storage,
            registry,
            item_reader,
            download_youtube_audio,
            timeout: DEFAULT_SEPARATION_TIMEOUT,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub async fn probe(&self, timeout: Duration) -> bool {
        self.separator.probe(timeout).await
    }

    pub async fn execute(
        &self,
        input: SeparateVocalsInput,
    ) -> Result<SeparateVocalsOutput, SeparateVocalsError> {
        // 1. Resolve source audio into path, title, and optional single-use lease guard
        let resolved = self.resolve_source(input.source).await?;

        // 2. Reserve an output staging file protected by RAII StagingFileGuard
        let (stage_id, stage_path) = self
            .storage
            .create_staging_file()
            .map_err(|e| SeparateVocalsError::Storage(e.to_string()))?;
        let staging_guard =
            StagingFileGuard::new(self.storage.clone(), stage_id.clone(), stage_path.clone());

        // 3. Invoke VocalSeparator port with structured parameters
        let request = VocalSeparationRequest {
            source_path: resolved.source_path,
            output_path: stage_path.clone(),
            mode: input.mode,
            timeout: self.timeout,
        };

        let separated = self.separator.separate(request).await?;

        // 4. On successful separation, defuse the output guard and register in StagedAudioRegistry with 15-min TTL
        let final_title = format!("{}{}", resolved.source_title, input.mode.title_suffix());
        staging_guard.defuse();

        let descriptor = self.registry.register(
            stage_id,
            stage_path,
            final_title.clone(),
            separated.duration_secs,
            separated.size_bytes,
        );

        // 5. Destructively commit temporary source audio if applicable (e.g. staged / youtube)
        if let Some(guard) = resolved.lease_guard {
            guard
                .commit_processed()
                .map_err(SeparateVocalsError::from)?;
        }

        Ok(SeparateVocalsOutput {
            staged_audio_id: descriptor.staged_audio_id,
            mode: input.mode,
            title: final_title,
            duration_secs: separated.duration_secs,
            size_bytes: separated.size_bytes,
        })
    }

    async fn resolve_source(
        &self,
        source: VocalAudioSource,
    ) -> Result<ResolvedAudioSource, SeparateVocalsError> {
        match source {
            VocalAudioSource::StagedAudioId(id) => self.resolve_staged_source(&id),
            VocalAudioSource::TrackId(id) => self.resolve_track_source(&id),
            VocalAudioSource::YoutubeUrl(url) => self.resolve_youtube_source(&url).await,
        }
    }

    fn resolve_staged_source(&self, id: &str) -> Result<ResolvedAudioSource, SeparateVocalsError> {
        let guard = match self.registry.reserve_guarded(id) {
            Ok(g) => g,
            Err(StagedAudioRegistryError::NotFound) => {
                return Err(SeparateVocalsError::StagedAudioNotFound);
            }
            Err(StagedAudioRegistryError::AlreadyConsumed) => {
                return Err(SeparateVocalsError::AudioAlreadyConsumed);
            }
            Err(StagedAudioRegistryError::InUse) => {
                return Err(SeparateVocalsError::StagedAudioInUse);
            }
            Err(e) => return Err(SeparateVocalsError::Registry(e)),
        };
        let source_path = guard.file_path.clone();
        let source_title = guard.title.clone();
        Ok(ResolvedAudioSource {
            source_path,
            source_title,
            lease_guard: Some(guard),
        })
    }

    fn resolve_track_source(&self, id: &str) -> Result<ResolvedAudioSource, SeparateVocalsError> {
        let item = self
            .item_reader
            .get_by_id(id)
            .map_err(|e| SeparateVocalsError::Storage(e.to_string()))?
            .ok_or(SeparateVocalsError::TrackNotFound)?;

        let track_payload = match item.payload {
            ShowItemPayload::Track(t) => t,
            _ => return Err(SeparateVocalsError::SourceNotTrack),
        };

        let source_path = self
            .storage
            .resolve_path_str(&track_payload.audio_key)
            .map_err(|e| SeparateVocalsError::Storage(e.to_string()))?;

        if !source_path.exists() {
            return Err(SeparateVocalsError::AudioUnavailable);
        }

        Ok(ResolvedAudioSource {
            source_path,
            source_title: track_payload.title,
            lease_guard: None,
        })
    }

    async fn resolve_youtube_source(
        &self,
        url: &str,
    ) -> Result<ResolvedAudioSource, SeparateVocalsError> {
        let downloaded = self.download_youtube_audio.execute(url, None).await?;
        let guard = self
            .registry
            .reserve_guarded(&downloaded.staged_audio_id)
            .map_err(|e| match e {
                StagedAudioRegistryError::NotFound => SeparateVocalsError::StagedAudioNotFound,
                StagedAudioRegistryError::AlreadyConsumed => {
                    SeparateVocalsError::AudioAlreadyConsumed
                }
                StagedAudioRegistryError::InUse => SeparateVocalsError::StagedAudioInUse,
                other => SeparateVocalsError::Registry(other),
            })?;
        let source_path = guard.file_path.clone();
        Ok(ResolvedAudioSource {
            source_path,
            source_title: downloaded.title,
            lease_guard: Some(guard),
        })
    }
}
