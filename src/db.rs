use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use std::{path::Path, time::Duration};

pub async fn init_pool(db_path: &str) -> SqlitePool {
    if let Some(parent) = Path::new(db_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).expect("Failed to create database directory");
    }
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(db_path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .foreign_keys(true)
                .busy_timeout(Duration::from_secs(10)),
        )
        .await
        .expect("Failed to connect to database")
}

// Embedded sources work regardless of the runtime working directory.
const MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_create_users.sql",
        include_str!("../migrations/001_create_users.sql"),
    ),
    (
        "002_create_groups.sql",
        include_str!("../migrations/002_create_groups.sql"),
    ),
    (
        "003_create_address_books.sql",
        include_str!("../migrations/003_create_address_books.sql"),
    ),
    (
        "004_create_peers.sql",
        include_str!("../migrations/004_create_peers.sql"),
    ),
    (
        "005_create_tags.sql",
        include_str!("../migrations/005_create_tags.sql"),
    ),
    (
        "006_create_devices.sql",
        include_str!("../migrations/006_create_devices.sql"),
    ),
    (
        "007_create_audit_log.sql",
        include_str!("../migrations/007_create_audit_log.sql"),
    ),
    (
        "008_sessions_and_indexes.sql",
        include_str!("../migrations/008_sessions_and_indexes.sql"),
    ),
    (
        "009_peer_groups.sql",
        include_str!("../migrations/009_peer_groups.sql"),
    ),
];

pub async fn run_migrations(pool: &SqlitePool) {
    apply_migrations(pool, MIGRATIONS).await.expect("Database migration failed; data was preserved, check migration history and duplicate personal books");
}

pub async fn apply_migrations(
    pool: &SqlitePool,
    migrations: &[(&'static str, &'static str)],
) -> Result<(), sqlx::Error> {
    // Preserve the upstream tracker; store SQL sources to detect subsequent edits.
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS _migrations (name TEXT PRIMARY KEY, applied_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP);
        CREATE TABLE IF NOT EXISTS _migration_sources (name TEXT PRIMARY KEY, sql TEXT NOT NULL);")
        .execute(pool).await?;
    for (name, source) in migrations {
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        let previous: Option<String> =
            sqlx::query_scalar("SELECT sql FROM _migration_sources WHERE name = ?")
                .bind(name)
                .fetch_optional(&mut *tx)
                .await?;
        if previous.as_ref().is_some_and(|old| old != source) {
            return Err(sqlx::Error::Protocol(format!(
                "Applied migration {name} was modified"
            )));
        }
        let applied: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = ?)")
                .bind(name)
                .fetch_one(&mut *tx)
                .await?;
        if !applied {
            tracing::info!("Applying migration: {name}");
            sqlx::raw_sql(*source).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO _migrations (name) VALUES (?)")
                .bind(name)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("INSERT OR IGNORE INTO _migration_sources (name, sql) VALUES (?, ?)")
            .bind(name)
            .bind(source)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    Ok(())
}
