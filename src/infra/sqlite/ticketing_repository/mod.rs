pub mod events;
pub mod expenses;
pub mod interests;
pub mod summary;
pub mod tickets;

pub use tickets::DEFAULT_TICKETS_LIMIT;

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
