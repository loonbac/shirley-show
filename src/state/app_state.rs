use std::path::PathBuf;
use std::sync::Arc;

use crate::infra::filesystem::local_audio_storage::LocalAudioStorage;
use crate::infra::sqlite::cleanup_repository::SqliteCleanupRepository;
use crate::infra::sqlite::connect::DbPool;
use crate::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use crate::infra::sqlite::presentation_summary_repository::PresentationSummaryRepository;
use crate::infra::sqlite::show_item_repository::SqliteShowItemRepository;
use crate::use_cases::add_lyrics::AddLyrics;
use crate::use_cases::add_note::AddNote;
use crate::use_cases::create_presentation::CreatePresentation;
use crate::use_cases::delete_item::DeleteItem;
use crate::use_cases::delete_presentation::DeletePresentation;
use crate::use_cases::get_presentation::GetPresentation;
use crate::use_cases::list_presentations::ListPresentations;
use crate::use_cases::open_track_audio::OpenTrackAudio;
use crate::use_cases::rename_presentation::RenamePresentation;
use crate::use_cases::reorder_item::ReorderItem;
use crate::use_cases::update_show_item::UpdateShowItem;
use crate::use_cases::upload_track::UploadTrack;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
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
}

impl AppState {
    pub fn new(pool: DbPool) -> Self {
        Self::new_with_items(pool, Arc::new(LocalAudioStorage::new(PathBuf::from("audio"))))
    }

    pub fn new_with_items(pool: DbPool, storage: Arc<LocalAudioStorage>) -> Self {
        let pres_repo = Arc::new(SqlitePresentationRepository::new(pool.clone()));
        let summary_repo = Arc::new(PresentationSummaryRepository::new(pool.clone()));
        let item_repo = Arc::new(SqliteShowItemRepository::new(pool.clone()));
        let cleanup_repo = Arc::new(SqliteCleanupRepository::new(pool.clone()));

        Self {
            pool,
            storage: storage.clone(),
            frontend_dir: PathBuf::from("frontend"),
            item_repo: item_repo.clone(),
            summary_repo: summary_repo.clone(),
            cleanup_repo: cleanup_repo.clone(),
            list_presentations: Arc::new(ListPresentations::new(summary_repo)),
            create_presentation: Arc::new(CreatePresentation::new(pres_repo.clone())),
            get_presentation: Arc::new(GetPresentation::new(pres_repo.clone())),
            rename_presentation: Arc::new(RenamePresentation::new(pres_repo.clone())),
            delete_presentation: Arc::new(DeletePresentation::new(pres_repo)),
            add_lyrics: Arc::new(AddLyrics::new(item_repo.clone())),
            add_note: Arc::new(AddNote::new(item_repo.clone())),
            upload_track: Arc::new(UploadTrack::new(item_repo.clone(), storage.clone())),
            reorder_item: Arc::new(ReorderItem::new(item_repo.clone())),
            update_show_item: Arc::new(UpdateShowItem::new(item_repo.clone(), item_repo.clone())),
            delete_item: Arc::new(DeleteItem::new(item_repo.clone(), storage.clone(), cleanup_repo)),
            open_track_audio: Arc::new(OpenTrackAudio::new(item_repo, storage)),
        }
    }
}
