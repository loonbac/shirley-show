use chrono::Utc;
use rusqlite::Connection;

use crate::error::AppError;
use crate::infra::sqlite::connect::DbPool;

const MIGRATION_0001: &str = include_str!("../../../migrations/0001_shirley_show_core.sql");

pub fn migrate_db(conn: &mut Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )
    .map_err(|e| AppError::Database(format!("Failed to create schema_migrations table: {e}")))?;

    let applied: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 1;",
            [],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("Failed to query schema_migrations: {e}")))?;

    if applied == 0 {
        let tx = conn
            .transaction()
            .map_err(|e| AppError::Database(format!("Failed to begin transaction for migration: {e}")))?;

        tx.execute_batch(MIGRATION_0001)
            .map_err(|e| AppError::Database(format!("Failed to apply migration 0001: {e}")))?;

        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (1, ?1);",
            [Utc::now().to_rfc3339()],
        )
        .map_err(|e| AppError::Database(format!("Failed to record migration version 1: {e}")))?;

        tx.commit()
            .map_err(|e| AppError::Database(format!("Failed to commit migration transaction: {e}")))?;
    }

    Ok(())
}

pub fn migrate_pool(pool: &DbPool) -> Result<(), AppError> {
    let mut conn = pool
        .get()
        .map_err(|e| AppError::Database(format!("Failed to acquire connection for migration: {e}")))?;
    migrate_db(&mut conn)
}
