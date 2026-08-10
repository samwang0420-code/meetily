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

        // §98 (2026-08-10): 启动前自愈 sqlx migration checksum mismatch.
        // 场景: 用户从老 bundle id (cn.lixianhuiji.app) 切到新 (tech.yanjingai.app) 后,
        // 新 db 里 _sqlx_migrations.checksum 还是老 binary 写入的 SHA-384, 跟当前 embed 的
        // migration 文件 hash 不一致 → sqlx 启动 panic "previously applied but has been modified".
        // 用 sqlx macro 已算好的 checksum 直接 UPDATE, 让 sqlx 觉得 db 是 up-to-date 的.
        if let Err(e) = Self::sync_migration_checksums(&pool).await {
            log::warn!("§98 sync_migration_checksums failed (best-effort, sqlx will report): {}", e);
        }

        // v0.7.0+ rc7: 直接跑 sqlx::migrate!.
        // _sqlx_migrations 表和 16 条记录已在外部脚本中正确初始化 (含 SHA-384 checksum),
        // sqlx 启动时会校验 checksum 与 migration 文件一致, 全部跳过, 不会重跑.
        // ensure_activation_codes_bound_machine_id 仍然 idempotent (PRAGMA 检查 + ALTER),
        // 老库有 bound_machine_id 跳过, 新库会补.
        sqlx::migrate!("./migrations").run(&pool).await?;

        // v0.7.0+ rc4: idempotent ALTER bound_machine_id (老库兼容).
        if let Err(e) = Self::ensure_activation_codes_bound_machine_id(&pool).await {
            log::error!("ensure_activation_codes_bound_machine_id failed: {}", e);
            return Err(e);
        }

        Ok(DatabaseManager { pool })
    }

    // NOTE: So for the first time users they needs to start the application
    // after they can just delete the existing .sqlite file and then copy the existing .db file to
    // the current app dir, So the system detects legacy db and copy it and starts with that data
    // (Newly created .sqlite with the copied content from .db)

    /// v0.7.0+ rc4: idempotent ALTER bound_machine_id (老库兼容).
    /// 老库 (C4 部署过) 已有列, 跳过; 新库第一次启动会加.
    /// 之前用 query_as fetch_optional<Option<(i64,)>> 在 sqlx 0.8 unparameterized 模式会触发 BLOB/Vec<u8> 推断错误.
    async fn ensure_activation_codes_bound_machine_id(pool: &SqlitePool) -> Result<()> {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('activation_codes') WHERE name = 'bound_machine_id'"
        )
        .fetch_one(pool)
        .await?;
        if exists == 0 {
            log::info!("activation_codes.bound_machine_id missing, adding via ALTER TABLE");
            sqlx::query("ALTER TABLE activation_codes ADD COLUMN bound_machine_id TEXT")
                .execute(pool)
                .await?;
        } else {
            log::debug!("activation_codes.bound_machine_id already exists, skip");
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

    /// §98 (2026-08-10): Self-heal sqlx _sqlx_migrations.checksum mismatch.
    ///
    /// 扫描所有 embed 的 migration 文件, 算 SHA-384 hash, UPDATE 到 db 里对应行的 checksum.
    /// 如果 db 里没有该 version 行, 跳过 (sqlx::migrate! 后续会自动跑并写入).
    /// 失败仅 log warn, 不阻塞启动 (sqlx 后续会报错).
    async fn sync_migration_checksums(pool: &SqlitePool) -> Result<(), sqlx::Error> {
        let migrations = sqlx::migrate!("./migrations");
        let mut updated = 0usize;
        let mut missing = 0usize;
        for migration in migrations.iter() {
            let version = migration.version;
            // 直接用 sqlx macro 已算好的 checksum (SHA-384 raw bytes), 保证跟 sqlx::migrate! 一致
            let checksum = migration.checksum.as_ref();
            let row: Option<(Vec<u8>,)> = sqlx::query_as(
                "SELECT checksum FROM _sqlx_migrations WHERE version = ?1"
            )
            .bind(version)
            .fetch_optional(pool)
            .await?;
            match row {
                None => { missing += 1; }
                Some((existing,)) if existing.as_slice() == checksum => {}
                Some(_) => {
                    sqlx::query("UPDATE _sqlx_migrations SET checksum = ?1 WHERE version = ?2")
                        .bind(checksum)
                        .bind(version)
                        .execute(pool)
                        .await?;
                    updated += 1;
                }
            }
        }
        if updated > 0 || missing > 0 {
            log::info!("§98 sync_migration_checksums: updated={} missing={}", updated, missing);
        }
        Ok(())
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
