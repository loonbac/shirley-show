use chrono::Utc;
use rusqlite::Connection;

use crate::error::AppError;
use crate::infra::sqlite::connect::DbPool;

const MIGRATION_0001: &str = include_str!("../../../migrations/0001_shirley_show_core.sql");
const MIGRATION_0002: &str = include_str!("../../../migrations/0002_ticketing.sql");

pub fn migrate_db(conn: &mut Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )
    .map_err(|e| AppError::Database(format!("Failed to create schema_migrations table: {e}")))?;

    let migrations: &[(i32, &str)] = &[
        (1, MIGRATION_0001),
        (2, MIGRATION_0002),
    ];

    for (version, sql) in migrations {
        let applied: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1;",
                [version],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(format!("Failed to query schema_migrations: {e}")))?;

        if applied == 0 {
            let tx = conn.transaction().map_err(|e| {
                AppError::Database(format!("Failed to begin transaction for migration {version}: {e}"))
            })?;

            tx.execute_batch(sql)
                .map_err(|e| AppError::Database(format!("Failed to apply migration {version}: {e}")))?;

            tx.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2);",
                rusqlite::params![version, Utc::now().to_rfc3339()],
            )
            .map_err(|e| AppError::Database(format!("Failed to record migration version {version}: {e}")))?;

            tx.commit().map_err(|e| {
                AppError::Database(format!("Failed to commit migration transaction {version}: {e}"))
            })?;
        }
    }

    Ok(())
}

pub fn migrate_pool(pool: &DbPool) -> Result<(), AppError> {
    let mut conn = pool.get().map_err(|e| {
        AppError::Database(format!("Failed to acquire connection for migration: {e}"))
    })?;
    migrate_db(&mut conn)
}
