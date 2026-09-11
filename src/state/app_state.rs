use std::sync::Arc;

use crate::infra::sqlite::connect::DbPool;
use crate::infra::sqlite::presentation_repository::SqlitePresentationRepository;
use crate::use_cases::create_presentation::CreatePresentation;
use crate::use_cases::delete_presentation::DeletePresentation;
use crate::use_cases::get_presentation::GetPresentation;
use crate::use_cases::list_presentations::ListPresentations;
use crate::use_cases::rename_presentation::RenamePresentation;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub list_presentations: Arc<ListPresentations<SqlitePresentationRepository>>,
    pub create_presentation: Arc<CreatePresentation<SqlitePresentationRepository>>,
    pub get_presentation: Arc<GetPresentation<SqlitePresentationRepository>>,
    pub rename_presentation: Arc<RenamePresentation<SqlitePresentationRepository>>,
    pub delete_presentation: Arc<DeletePresentation<SqlitePresentationRepository>>,
}

impl AppState {
    pub fn new(pool: DbPool) -> Self {
        let repo = Arc::new(SqlitePresentationRepository::new(pool.clone()));

        Self {
            pool,
            list_presentations: Arc::new(ListPresentations::new(repo.clone())),
            create_presentation: Arc::new(CreatePresentation::new(repo.clone())),
            get_presentation: Arc::new(GetPresentation::new(repo.clone())),
            rename_presentation: Arc::new(RenamePresentation::new(repo.clone())),
            delete_presentation: Arc::new(DeletePresentation::new(repo)),
        }
    }
}
