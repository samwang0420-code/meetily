use sqlx::{migrate::MigrateDatabase, Result, Sqlite, SqlitePool, Transaction};
use std::fs;
use std::path::Path;
use tauri::Manager;

#[derive(Clone)]
pub struct DatabaseManager {
    pool: SqlitePool,
}

impl DatabaseManager {
    pub async fn new(tauri_db_path: &str, backend_db_path: &str) -> Result<Self> {
        if let Some(parent_dir) = Path::new(tauri_db_path).parent() {
            if !parent_dir.exists() {
                fs::create_dir_all(parent_dir).map_err(|e| sqlx::Error::Io(e))?;
            }
        }

        if !Path::new(tauri_db_path).exists() {
            if Path::new(backend_db_path).exists() {
                log::info!(
                    "Copying database from {} to {}",
                    backend_db_path,
                    tauri_db_path
                );
                fs::copy(backend_db_path, tauri_db_path).map_err(|e| sqlx::Error::Io(e))?;
            } else {
                log::info!("Creating database at {}", tauri_db_path);
                Sqlite::create_database(tauri_db_path).await?;
            }
        }

        let pool = SqlitePool::connect(tauri_db_path).await?;

        // v0.7.0+ rc3: 在 sqlx::migrate! 之前手工跑 idempotent ALTER,
        // 修复 migration 20260719000000 的 bound_machine_id 列兼容问题.
        // 原因: SQLite ALTER TABLE ADD COLUMN 不支持 IF NOT EXISTS,
        // 老库 (C4 部署过) 已有 bound_machine_id 列, 重跑 ALTER 会 duplicate column panic.
        if let Err(e) = Self::ensure_activation_codes_bound_machine_id(&pool).await {
            log::error!("ensure_activation_codes_bound_machine_id failed: {}", e);
            return Err(e);
        }
        // 标记 migration 20260719000000 为已跑 (它的 SQL 内容已经被手动处理)
        if let Err(e) = Self::mark_migration_applied(&pool, "20260719000000", "lixianhuiji_commercial_hardening").await {
            log::warn!("mark_migration_applied for 20260719000000 failed (non-fatal): {}", e);
        }

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(DatabaseManager { pool })
    }

    // NOTE: So for the first time users they needs to start the application
    // after they can just delete the existing .sqlite file and then copy the existing .db file to
    // the current app dir, So the system detects legacy db and copy it and starts with that data
    // (Newly created .sqlite with the copied content from .db)

    /// v0.7.0+ rc3: 检查 activation_codes 表是否有 bound_machine_id 列, 没有就 ALTER.
    /// 老库 (C4 部署过) 已有, 跳过; 新库第一次启动会加.
    async fn ensure_activation_codes_bound_machine_id(pool: &SqlitePool) -> Result<()> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM pragma_table_info('activation_codes') WHERE name = 'bound_machine_id'"
        )
        .fetch_optional(pool)
        .await?;
        let exists = row.map(|r| r.0 > 0).unwrap_or(false);
        if !exists {
            log::info!("activation_codes.bound_machine_id missing, adding via ALTER TABLE");
            sqlx::query("ALTER TABLE activation_codes ADD COLUMN bound_machine_id TEXT")
                .execute(pool)
                .await?;
            log::info!("activation_codes.bound_machine_id added");
        } else {
            log::debug!("activation_codes.bound_machine_id already exists, skip");
        }
        Ok(())
    }

    /// v0.7.0+ rc3: 手动把 migration 标记为已成功跑过, 避免 sqlx::migrate! 重跑.
    /// 因为 20260719000000 的 SQL 内容 (ALTER TABLE) 已经被 ensure_activation_codes_bound_machine_id 处理,
    /// 这里只插入 _sqlx_migrations 记录, 不再执行文件里的 SQL.
    async fn mark_migration_applied(pool: &SqlitePool, version: &str, description: &str) -> Result<()> {
        let row: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM _sqlx_migrations WHERE version = ?1"
        )
        .bind(version)
        .fetch_optional(pool)
        .await?;
        let exists = row.map(|r| r.0 > 0).unwrap_or(false);
        if !exists {
            log::info!("Manually marking migration {} as applied", version);
            sqlx::query(
                "INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
                 VALUES (?1, ?2, datetime('now'), 1, 0, 0)"
            )
            .bind(version)
            .bind(description)
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    pub async fn new_from_app_handle(app_handle: &tauri::AppHandle) -> Result<Self> {
        // Resolve the app's data directory
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir).map_err(|e| sqlx::Error::Io(e))?;
        }

        // Define database paths
        let tauri_db_path = app_data_dir
            .join("meeting_minutes.sqlite")
            .to_string_lossy()
            .to_string();
        // Legacy backend DB path (for auto-migration if exists)
        let backend_db_path = app_data_dir
            .join("meeting_minutes.db")
            .to_string_lossy()
            .to_string();

        // WAL file paths for defensive cleanup
        let wal_path = app_data_dir.join("meeting_minutes.sqlite-wal");
        let shm_path = app_data_dir.join("meeting_minutes.sqlite-shm");

        log::info!("Tauri DB path: {}", tauri_db_path);
        log::info!("Legacy backend DB path: {}", backend_db_path);

        // Try to open database with defensive WAL handling
        match Self::new(&tauri_db_path, &backend_db_path).await {
            Ok(db_manager) => {
                log::info!("Database opened successfully");
                Ok(db_manager)
            }
            Err(e) => {
                // Check if error is due to corrupted WAL file
                let error_msg = e.to_string();
                if error_msg.contains("malformed") || error_msg.contains("corrupt") {
                    log::warn!("Database appears corrupted, likely due to orphaned WAL file. Attempting recovery...");
                    log::warn!("Error details: {}", error_msg);

                    // Delete potentially corrupted WAL/SHM files
                    if wal_path.exists() {
                        match fs::remove_file(&wal_path) {
                            Ok(_) => log::info!("Removed orphaned WAL file: {:?}", wal_path),
                            Err(e) => log::warn!("Failed to remove WAL file: {}", e),
                        }
                    }
                    if shm_path.exists() {
                        match fs::remove_file(&shm_path) {
                            Ok(_) => log::info!("Removed orphaned SHM file: {:?}", shm_path),
                            Err(e) => log::warn!("Failed to remove SHM file: {}", e),
                        }
                    }

                    // Retry connection without WAL files
                    log::info!("Retrying database connection after WAL cleanup...");
                    match Self::new(&tauri_db_path, &backend_db_path).await {
                        Ok(db_manager) => {
                            log::info!("Database opened successfully after WAL recovery");
                            Ok(db_manager)
                        }
                        Err(retry_err) => {
                            log::error!("Database connection failed even after WAL cleanup: {}", retry_err);
                            Err(retry_err)
                        }
                    }
                } else {
                    // Not a WAL-related error, propagate original error
                    log::error!("Database connection failed: {}", error_msg);
                    Err(e)
                }
            }
        }
    }

    /// Check if this is the first launch (sqlite database doesn't exist yet)
    pub async fn is_first_launch(app_handle: &tauri::AppHandle) -> Result<bool> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");

        let tauri_db_path = app_data_dir.join("meeting_minutes.sqlite");

        Ok(!tauri_db_path.exists())
    }

    /// Import a legacy database from the specified path and initialize
    pub async fn import_legacy_database(
        app_handle: &tauri::AppHandle,
        legacy_db_path: &str,
    ) -> Result<Self> {
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .expect("failed to get app data dir");

        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir).map_err(|e| sqlx::Error::Io(e))?;
        }

        // Copy legacy database to app data directory as meeting_minutes.db
        let target_legacy_path = app_data_dir.join("meeting_minutes.db");
        log::info!(
            "Copying legacy database from {} to {}",
            legacy_db_path,
            target_legacy_path.display()
        );

        fs::copy(legacy_db_path, &target_legacy_path).map_err(|e| sqlx::Error::Io(e))?;

        // Now use the standard initialization which will detect and migrate the legacy db
        Self::new_from_app_handle(app_handle).await
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn with_transaction<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut Transaction<'_, Sqlite>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut tx = self.pool.begin().await?;
        let result = f(&mut tx).await;

        match result {
            Ok(val) => {
                tx.commit().await?;
                Ok(val)
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err)
            }
        }
    }

    /// Cleanup database connection and checkpoint WAL
    /// This should be called on application shutdown to ensure:
    /// - All WAL changes are written to the main database file
    /// - The .wal and .shm files are deleted
    /// - Connection pool is gracefully closed
    pub async fn cleanup(&self) -> Result<()> {
        log::info!("Starting database cleanup...");

        // Force checkpoint of WAL to main database file and remove WAL file
        // TRUNCATE mode: checkpoints all pages AND deletes the WAL file
        match sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .execute(&self.pool)
            .await
        {
            Ok(_) => log::info!("WAL checkpoint completed successfully"),
            Err(e) => log::warn!("WAL checkpoint failed (non-fatal): {}", e),
        }

        // Close the connection pool gracefully
        self.pool.close().await;
        log::info!("Database connection pool closed");

        Ok(())
    }
}
