use std::path::PathBuf;
use std::sync::Arc;

use crate::agent::destructive_confirmation::PendingDestructiveRegistry;
use crate::agent::tool_dispatcher::AgentToolDispatcher;
use crate::config::{Config, DEFAULT_GROQ_MODEL};
use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;
use crate::infra::filesystem::local_poster_storage::LocalPosterStorage;
use crate::infra::sqlite::cleanup_repository::SqliteCleanupRepository;
use crate::infra::sqlite::connect::DbPool;
use crate::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use crate::infra::sqlite::presentation_summary_repository::PresentationSummaryRepository;
use crate::infra::sqlite::show_item_repository::SqliteShowItemRepository;
use crate::infra::sqlite::ticketing_repository::SqliteTicketingRepository;
use crate::infra::vocal::{ColabVocalSeparator, ColabVocalSeparatorConfig};
use crate::infra::youtube::{ProcessYoutubeDownloader, ProcessYoutubeDownloaderConfig};
use crate::ports::chat_completion_provider::ChatCompletionProvider;
use crate::ports::ticketing::EventRepository;
use crate::ports::vocal_separator::VocalSeparator;
use crate::ports::youtube_downloader::YoutubeDownloader;
use crate::state::staged_audio_registry::StagedAudioRegistry;
use crate::state::ticketing_services::TicketingServices;
use crate::use_cases::add_lyrics::AddLyrics;
use crate::use_cases::add_note::AddNote;
use crate::use_cases::agent_chat::{AgentChat, AgentLimits};
use crate::use_cases::create_presentation::CreatePresentation;
use crate::use_cases::delete_item::DeleteItem;
use crate::use_cases::delete_presentation::DeletePresentation;
use crate::use_cases::download_youtube_audio::DownloadYouTubeAudio;
use crate::use_cases::get_presentation::GetPresentation;
use crate::use_cases::list_presentations::ListPresentations;
use crate::use_cases::open_track_audio::OpenTrackAudio;
use crate::use_cases::rename_presentation::RenamePresentation;
use crate::use_cases::reorder_item::ReorderItem;
use crate::use_cases::separate_vocals::SeparateVocals;
use crate::use_cases::update_show_item::UpdateShowItem;
use crate::use_cases::upload_track::UploadTrack;

#[derive(Clone)]
pub struct AgentRuntime {
    pub agent_chat: Arc<AgentChat>,
    pub limits: AgentLimits,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    /// Decides what each request may reach, by its `Host` header.
    pub surface: crate::config::SurfaceConfig,
    /// Base URL used to wrap a new ticket's door secret into its printed QR.
    ///
    /// `None` keeps the bare 32-hex payload, so a deployment without this
    /// variable keeps printing tickets exactly as before.
    pub public_base_url: Option<String>,
    pub storage: Arc<LocalAudioStorage>,
    pub frontend_dir: PathBuf,
    pub item_repo: Arc<SqliteShowItemRepository>,
    pub summary_repo: Arc<PresentationSummaryRepository>,
    pub cleanup_repo: Arc<SqliteCleanupRepository>,
    pub list_presentations: Arc<ListPresentations<PresentationSummaryRepository>>,
    pub create_presentation: Arc<CreatePresentation<SqlitePresentationRepository>>,
    pub get_presentation: Arc<GetPresentation<SqlitePresentationRepository>>,
    pub rename_presentation: Arc<RenamePresentation<SqlitePresentationRepository>>,
    pub delete_presentation: Arc<DeletePresentation<SqlitePresentationRepository>>,
    pub add_lyrics: Arc<AddLyrics<SqliteShowItemRepository>>,
    pub add_note: Arc<AddNote<SqliteShowItemRepository>>,
    pub upload_track: Arc<UploadTrack<SqliteShowItemRepository>>,
    pub reorder_item: Arc<ReorderItem<SqliteShowItemRepository>>,
    pub update_show_item: Arc<UpdateShowItem<SqliteShowItemRepository, SqliteShowItemRepository>>,
    pub delete_item: Arc<DeleteItem<SqliteShowItemRepository, SqliteCleanupRepository>>,
    pub open_track_audio: Arc<OpenTrackAudio<SqliteShowItemRepository>>,
    pub staged_audio_registry: Arc<StagedAudioRegistry>,
    pub youtube_downloader: Arc<dyn YoutubeDownloader>,
    pub download_youtube_audio: Arc<DownloadYouTubeAudio>,
    pub gpu_semaphore: Arc<tokio::sync::Semaphore>,
    pub vocal_separator: Arc<dyn VocalSeparator>,
    pub separate_vocals: Arc<SeparateVocals<SqliteShowItemRepository>>,
    pub agent_runtime: AgentRuntime,
    pub agent_chat: Arc<AgentChat>,
    pub ticketing: TicketingServices,
    pub destructive_registry: Arc<PendingDestructiveRegistry>,
}

impl AppState {
    pub fn new(pool: DbPool) -> Self {
        Self::new_with_items(
            pool,
            Arc::new(LocalAudioStorage::new(PathBuf::from("audio"))),
        )
    }

    pub fn new_with_items(pool: DbPool, storage: Arc<LocalAudioStorage>) -> Self {
        let staged_audio_registry = Arc::new(StagedAudioRegistry::new());
        let youtube_downloader: Arc<dyn YoutubeDownloader> = Arc::new(
            ProcessYoutubeDownloader::new(ProcessYoutubeDownloaderConfig::default()),
        );
        Self::new_with_all_components(pool, storage, staged_audio_registry, youtube_downloader)
    }

    pub fn new_with_all_components(
        pool: DbPool,
        storage: Arc<LocalAudioStorage>,
        staged_audio_registry: Arc<StagedAudioRegistry>,
        youtube_downloader: Arc<dyn YoutubeDownloader>,
    ) -> Self {
        let config = Config::from_env().unwrap_or_else(|_| Config {
            bind_addr: "127.0.0.1:3000".parse().unwrap(),
            data_dir: PathBuf::from("data"),
            frontend_dir: PathBuf::from("frontend"),
            groq: Default::default(),
            yt_dlp_bin: None,
            agent_timeout_secs: 120,
            separation_timeout_secs: 600,
            remote_separation_timeout_secs: 1500,
            joshua_pc: None,
            surface: crate::config::SurfaceConfig::new_three(None, None, None),
        });
        Self::new_with_all_components_and_config(
            pool,
            storage,
            staged_audio_registry,
            youtube_downloader,
            &config,
        )
    }

    pub fn new_with_all_components_and_config(
        pool: DbPool,
        storage: Arc<LocalAudioStorage>,
        staged_audio_registry: Arc<StagedAudioRegistry>,
        youtube_downloader: Arc<dyn YoutubeDownloader>,
        config: &Config,
    ) -> Self {
        let pres_repo = Arc::new(SqlitePresentationRepository::new(pool.clone()));
        let summary_repo = Arc::new(PresentationSummaryRepository::new(pool.clone()));
        let item_repo = Arc::new(SqliteShowItemRepository::new(pool.clone()));
        let cleanup_repo = Arc::new(SqliteCleanupRepository::new(pool.clone()));

        let list_presentations = Arc::new(ListPresentations::new(summary_repo.clone()));
        let create_presentation = Arc::new(CreatePresentation::new(pres_repo.clone()));
        let get_presentation = Arc::new(GetPresentation::new(pres_repo.clone()));
        let rename_presentation = Arc::new(RenamePresentation::new(pres_repo.clone()));
        let delete_presentation = Arc::new(DeletePresentation::new(pres_repo));
        let add_lyrics = Arc::new(AddLyrics::new(item_repo.clone()));
        let add_note = Arc::new(AddNote::new(item_repo.clone()));
        let upload_track = Arc::new(UploadTrack::new(item_repo.clone(), storage.clone()));
        let reorder_item = Arc::new(ReorderItem::new(item_repo.clone()));
        let update_show_item = Arc::new(UpdateShowItem::new(item_repo.clone(), item_repo.clone()));
        let delete_item = Arc::new(DeleteItem::new(
            item_repo.clone(),
            storage.clone(),
            cleanup_repo.clone(),
        ));
        let open_track_audio = Arc::new(OpenTrackAudio::new(item_repo.clone(), storage.clone()));
        let download_youtube_audio = Arc::new(DownloadYouTubeAudio::new(
            youtube_downloader.clone(),
            storage.clone(),
            staged_audio_registry.clone(),
        ));

        let gpu_semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        // Separation always runs on Colab (S1). The Joshua PC and local process
        // backends are retired from the production path.
        let colab_config = ColabVocalSeparatorConfig::from_env(
            storage.vocal_work_dir().to_path_buf(),
            gpu_semaphore.clone(),
        );
        // The Colab GPU is only alive while a separation runs, so the timeout must
        // cover session startup plus the whole ensemble, not just one model.
        let separation_timeout = std::time::Duration::from_secs(colab_config.timeout_secs);
        let colab_separator = Arc::new(ColabVocalSeparator::new(colab_config));

        let vocal_separator: Arc<dyn VocalSeparator> = colab_separator;
        // Vigilante de runtimes huérfanos: si el release falla en silencio, nadie
        // reintenta el apagado y la GPU sigue facturando hasta que Colab la
        // expire. Se le pasa el semáforo de GPU como señal de trabajo en curso.
        let is_remote = true;

        let separate_vocals = Arc::new(
            SeparateVocals::new(
                vocal_separator.clone(),
                storage.clone(),
                staged_audio_registry.clone(),
                item_repo.clone(),
                download_youtube_audio.clone(),
            )
            .with_timeout(separation_timeout),
        );

        let ticketing_repo = Arc::new(SqliteTicketingRepository::new(pool.clone()));
        let posters_dir = config.data_dir.join("posters");
        let poster_storage = Arc::new(LocalPosterStorage::new(posters_dir));
        if let Ok(events) = ticketing_repo.list_events() {
            let active_posters: Vec<String> = events.into_iter().filter_map(|e| e.imagen).collect();
            poster_storage.reconcile_orphans(&active_posters);
        }
        let public_base_url = std::env::var("SHIRLEY_PUBLIC_BASE_URL")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let ticketing = TicketingServices::new(
            ticketing_repo,
            poster_storage,
            public_base_url.clone(),
        );
        let destructive_registry = Arc::new(PendingDestructiveRegistry::new());

        let dispatcher = AgentToolDispatcher::from_components(
            create_presentation.clone(),
            list_presentations.clone(),
            get_presentation.clone(),
            add_lyrics.clone(),
            add_note.clone(),
            upload_track.clone(),
            item_repo.clone(),
            download_youtube_audio.clone(),
            staged_audio_registry.clone(),
            separate_vocals.clone(),
            rename_presentation.clone(),
            reorder_item.clone(),
            update_show_item.clone(),
        )
        .with_ticketing(ticketing.clone())
        .with_public_base_url(std::env::var("SHIRLEY_PUBLIC_BASE_URL").ok())
        .with_destructive_presentation(delete_presentation.clone(), delete_item.clone())
        .with_destructive_registry(destructive_registry.clone());

        let limits = AgentLimits {
            max_iterations: 5,
            request_timeout: std::time::Duration::from_secs(config.agent_timeout_secs),
            separation_timeout,
        };
        let agent_chat = Arc::new(
            AgentChat::new(
                None,
                dispatcher,
                limits.clone(),
                DEFAULT_GROQ_MODEL.to_string(),
            )
            .with_remote_vocal_enabled(is_remote)
            .with_staged_audio_registry(staged_audio_registry.clone()),
        );

        let agent_runtime = AgentRuntime {
            agent_chat: agent_chat.clone(),
            limits,
        };
        Self {
            pool,
            surface: config.surface.clone(),
            public_base_url: std::env::var("SHIRLEY_PUBLIC_BASE_URL")
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
            destructive_registry,
            storage,
            frontend_dir: PathBuf::from("frontend"),
            item_repo,
            summary_repo,
            cleanup_repo,
            list_presentations,
            create_presentation,
            get_presentation,
            rename_presentation,
            delete_presentation,
            add_lyrics,
            add_note,
            upload_track,
            reorder_item,
            update_show_item,
            delete_item,
            open_track_audio,
            staged_audio_registry,
            youtube_downloader,
            download_youtube_audio,
            gpu_semaphore,
            vocal_separator,
            separate_vocals,
            agent_runtime,
            agent_chat,
            ticketing,
        }
    }

    pub fn with_youtube_downloader(mut self, downloader: Arc<dyn YoutubeDownloader>) -> Self {
        self.youtube_downloader = downloader.clone();
        let download_youtube_audio = Arc::new(DownloadYouTubeAudio::new(
            downloader,
            self.storage.clone(),
            self.staged_audio_registry.clone(),
        ));
        self.download_youtube_audio = download_youtube_audio.clone();
        self.separate_vocals = Arc::new(SeparateVocals::new(
            self.vocal_separator.clone(),
            self.storage.clone(),
            self.staged_audio_registry.clone(),
            self.item_repo.clone(),
            download_youtube_audio,
        ));
        self
    }

    pub fn with_vocal_separator(mut self, separator: Arc<dyn VocalSeparator>) -> Self {
        let prev_timeout = self.separate_vocals.timeout();
        self.vocal_separator = separator.clone();
        self.separate_vocals = Arc::new(
            SeparateVocals::new(
                separator,
                self.storage.clone(),
                self.staged_audio_registry.clone(),
                self.item_repo.clone(),
                self.download_youtube_audio.clone(),
            )
            .with_timeout(prev_timeout),
        );
        self
    }

    pub fn with_agent_provider(
        mut self,
        provider: Option<Arc<dyn ChatCompletionProvider>>,
        limits: AgentLimits,
        model: String,
    ) -> Self {
        let dispatcher = AgentToolDispatcher::from_state(&self);
        let is_remote = self.agent_chat.is_remote_vocal_enabled();
        let agent_chat = Arc::new(
            AgentChat::new(provider, dispatcher, limits.clone(), model)
                .with_remote_vocal_enabled(is_remote)
                .with_staged_audio_registry(self.staged_audio_registry.clone()),
        );
        self.agent_runtime = AgentRuntime {
            agent_chat: agent_chat.clone(),
            limits,
        };
        self.agent_chat = agent_chat;
        self
    }
}
