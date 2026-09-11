use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::error::AppError;

pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConnection = r2d2::PooledConnection<SqliteConnectionManager>;

pub fn init_pool(path: &str) -> Result<DbPool, AppError> {
    let manager = SqliteConnectionManager::file(path).with_init(|conn| {
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;
        Ok(())
    });

    let pool = Pool::builder()
        .max_size(16)
        .build(manager)
        .map_err(|e| AppError::Database(format!("Failed to build connection pool: {e}")))?;

    // Set WAL mode on DB initialization
    let conn = pool.get().map_err(|e| {
        AppError::Database(format!("Failed to acquire connection for WAL init: {e}"))
    })?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(|e| AppError::Database(format!("Failed to set WAL journal mode: {e}")))?;

    Ok(pool)
}
