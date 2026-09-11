pub mod events;
pub mod expenses;
pub mod summary;

use crate::infra::sqlite::connect::DbPool;

#[derive(Clone)]
pub struct SqliteTicketingRepository {
    pub(crate) pool: DbPool,
}

impl SqliteTicketingRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}
