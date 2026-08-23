// 离线会记 v0.5.0: tauri commands 暴露用户/会员 API
// 前端 invoke 对应: register / login / logout / get_current_user / get_machine_id / activate_member / hotwords_get / hotwords_save

use log::{error, info};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use crate::database::repositories::user::{UsersRepository, HotwordsRepository, UserPublic};
use crate::database::repositories::user as user_repo;
use crate::state::AppState;
use crate::user::auth::{gen_salt, hash_password, validate_email, validate_password, verify_password};
use crate::user::membership::{activate_member_for_user, MEMBER_BUNDLE_KEY};
use crate::user::machine_id::get_machine_id;
use std::sync::Mutex;
use std::collections::HashMap;

#[derive(Default)]
pub struct SessionStore {
    pub map: Mutex<HashMap<String, i64>>, // session_token -> user_id
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AuthBootstrap {
    pub session: Option<String>,
    pub user: Option<UserPublic>,
    pub last_email: Option<String>,
}

fn make_session_token() -> String {
    use rand::Rng;
    let r: [u8; 16] = rand::thread_rng().gen();
    let mut hex = String::with_capacity(32);
    for b in r.iter() { hex.push_str(&format!("{:02x}", b)); }
    hex
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LoginResult {
    pub ok: bool,
    pub session: Option<String>,
    pub user: Option<UserPublic>,
    pub error: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RegisterResult {
    pub ok: bool,
    pub session: Option<String>,
    pub user: Option<UserPublic>,
    pub error: Option<String>,
}

fn err_kind(kind: &str) -> LoginResult {
    LoginResult { ok: false, session: None, user: None, error: Some(kind.to_string()) }
}

fn db_pool<R: Runtime>(app: &AppHandle<R>) -> Result<sqlx::SqlitePool, String> {
    let state: tauri::State<AppState> = app.state();
    Ok(state.db_manager.pool().clone())
}

/// v0.7.0+: 用户登录后, 同步该用户的热词配置到 Rust globals,
/// 这样用户的第一次录音能立即拿到正确热词, 不需要先打开设置页.
async fn load_user_hotwords_into_globals<R: Runtime>(
    app: &AppHandle<R>,
    user_id: i64,
) -> Result<(), String> {
    let pool = db_pool(app)?;
    let (builtin, custom, _enabled) = HotwordsRepository::get(&pool, user_id)
        .await
        .map_err(|e| e.to_string())?;
    crate::audio::hotwords_globals::set(builtin, custom);
    info!("[hotwords] loaded into globals for user_id={}", user_id);
    Ok(())
}

async fn persist_session_to_db<R: Runtime>(
    app: &AppHandle<R>,
    token: &str,
    user_id: i64,
) -> Result<(), String> {
    let pool = db_pool(app)?;
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO auth_sessions (token, user_id, created_at, last_seen_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(token) DO UPDATE SET last_seen_at = excluded.last_seen_at"
    )
    .bind(token)
    .bind(user_id)
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

async fn lookup_session_in_db<R: Runtime>(
    app: &AppHandle<R>,
    token: &str,
) -> Result<Option<i64>, String> {
    let pool = db_pool(app)?;
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT user_id FROM auth_sessions WHERE token = ?1 LIMIT 1"
    )
    .bind(token)
    .fetch_optional(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.map(|(id,)| id))
}

async fn delete_session_in_db<R: Runtime>(
    app: &AppHandle<R>,
    token: &str,
) -> Result<(), String> {
    let pool = db_pool(app)?;
    let _ = sqlx::query("DELETE FROM auth_sessions WHERE token = ?1")
        .bind(token)
        .execute(&pool)
        .await;
    Ok(())
}

pub(crate) async fn latest_session_in_db<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<Option<(String, i64)>, String> {
    let pool = db_pool(app)?;
    sqlx::query_as(
        "SELECT token, user_id FROM auth_sessions ORDER BY last_seen_at DESC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| e.to_string())
}

async fn last_login_email<R: Runtime>(app: &AppHandle<R>) -> Result<Option<String>, String> {
    let pool = db_pool(app)?;
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT email FROM users ORDER BY COALESCE(last_login_at, created_at) DESC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(row.map(|(email,)| email))
}

/// Restore the most recent local session without relying on WebView localStorage.
/// This survives frontend rebuilds and development-origin changes because the
/// session token and account remain in the app's SQLite database.
#[tauri::command]
pub async fn user_bootstrap<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
) -> Result<AuthBootstrap, String> {
    let last_email = last_login_email(&app).await?;
    let Some((token, user_id)) = latest_session_in_db(&app).await? else {
        return Ok(AuthBootstrap { last_email, ..Default::default() });
    };
    let pool = db_pool(&app)?;
    let Some(user) = UsersRepository::get_by_id(&pool, user_id)
        .await
        .map_err(|e| e.to_string())?
    else {
        let _ = delete_session_in_db(&app, &token).await;
        return Ok(AuthBootstrap { last_email, ..Default::default() });
    };
    if user.is_active == 0 {
        let _ = delete_session_in_db(&app, &token).await;
        return Ok(AuthBootstrap { last_email, ..Default::default() });
    }
    sessions.map.lock().unwrap().insert(token.clone(), user_id);
    let _ = load_user_hotwords_into_globals(&app, user_id).await;
    Ok(AuthBootstrap {
        session: Some(token),
        user: Some(UserPublic::from(user)),
        last_email,
    })
}

#[tauri::command]
pub async fn user_register<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
    email: String,
    password: String,
    display_name: Option<String>,
) -> Result<RegisterResult, String> {
    info!("[register] invoked email={} display_name={:?}", email, display_name);
    if !validate_email(&email) {
        info!("[register] fail invalid_email email={}", email);
        return Ok(RegisterResult { ok: false, session: None, user: None, error: Some("invalid_email".into()) });
    }
    if let Err(e) = validate_password(&password) {
        info!("[register] fail password validation: {} email={}", e, email);
        return Ok(RegisterResult { ok: false, session: None, user: None, error: Some(e.into()) });
    }
    let pool = match db_pool(&app) {
        Ok(p) => p,
        Err(e) => {
            error!("[register] db_pool failed: {}", e);
            return Ok(RegisterResult { ok: false, session: None, user: None, error: Some("db_error".into()) });
        }
    };
    let machine_id = get_machine_id();
    let salt = gen_salt();
    let hash = hash_password(&password, &salt);
    let id = match UsersRepository::create_user(&pool, &email, &hash, &salt, Some(&machine_id)).await {
        Ok(id) => id,
        Err(sqlx::Error::Database(db)) if db.message().contains("UNIQUE") => {
            info!("[register] email_exists email={}", email);
            return Ok(RegisterResult { ok: false, session: None, user: None, error: Some("email_exists".into()) });
        }
        Err(e) => {
            error!("[register] db error full: {:?}", e);
            error!("[register] db error msg: {}", e);
            return Ok(RegisterResult { ok: false, session: None, user: None, error: Some("db_error".into()) });
        }
    };
    if let Some(d) = display_name {
        let _ = sqlx::query("UPDATE users SET display_name = ?1 WHERE id = ?2")
            .bind(&d).bind(id).execute(&pool).await;
    }
    let user = UsersRepository::get_by_id(&pool, id).await
        .map_err(|e| e.to_string())?
        .ok_or("user not found after create")?;
    let token = make_session_token();
    sessions.map.lock().unwrap().insert(token.clone(), id);
    info!("[register] ok email={} user_id={}", email, id);
    let _ = persist_session_to_db(&app, &token, id).await;
    let _ = load_user_hotwords_into_globals(&app, id).await;
    Ok(RegisterResult { ok: true, session: Some(token), user: Some(UserPublic::from(user)), ..Default::default() })
}

#[tauri::command]
pub async fn user_login<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
    email: String,
    password: String,
) -> Result<LoginResult, String> {
    info!("[login] invoked email={}", email);
    let pool = match db_pool(&app) {
        Ok(p) => p,
        Err(e) => {
            error!("[login] db_pool failed: {}", e);
            return Ok(err_kind("server_misconfigured"));
        }
    };
    let user = match UsersRepository::get_by_email(&pool, &email).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            info!("[login] no user email={}", email);
            return Ok(err_kind("bad_credential"));
        }
        Err(e) => {
            error!("[login] get_by_email error: {:?}", e);
            return Ok(err_kind("db_error"));
        }
    };
    if user.is_active == 0 {
        info!("[login] banned user_id={}", user.id);
        return Ok(err_kind("banned"));
    }
    let salt_row: Option<(String,)> = sqlx::query_as("SELECT salt FROM users WHERE id = ?1")
        .bind(user.id).fetch_optional(&pool).await.map_err(|e| e.to_string())?;
    let salt = salt_row.map(|t| t.0).unwrap_or_default();
    let hash_row: Option<(String,)> = sqlx::query_as("SELECT password_hash FROM users WHERE id = ?1")
        .bind(user.id).fetch_optional(&pool).await.map_err(|e| e.to_string())?;
    let expected = hash_row.map(|t| t.0).unwrap_or_default();
    if !verify_password(&password, &salt, &expected) {
        return Ok(err_kind("bad_credential"));
    }
    let _ = UsersRepository::update_last_login(&pool, user.id).await;
    let token = make_session_token();
    sessions.map.lock().unwrap().insert(token.clone(), user.id);
    info!("[login] ok email={} user_id={}", email, user.id);
    let _ = persist_session_to_db(&app, &token, user.id).await;
    let _ = load_user_hotwords_into_globals(&app, user.id).await;
    Ok(LoginResult { ok: true, session: Some(token), user: Some(UserPublic::from(user)), ..Default::default() })
}

#[tauri::command]
pub async fn user_get_current<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
    session: String,
) -> Result<Option<UserPublic>, String> {
    // Fast path: in-memory SessionStore. 进程内有效.
    let user_id_from_mem = {
        let m = sessions.map.lock().unwrap();
        m.get(&session).copied()
    };
    let id = match user_id_from_mem {
        Some(i) => Some(i),
        None => {
            // Fallback: 查 DB. app 重启后 SessionStore 是空的, 但 token 在 auth_sessions 表里.
            info!("[get_current] session not in memory, checking DB");
            let db_id = lookup_session_in_db(&app, &session).await?;
            if let Some(uid) = db_id {
                // 把 token 灌回 SessionStore, 后续内存命中快
                sessions.map.lock().unwrap().insert(session.clone(), uid);
                info!("[get_current] session restored from DB user_id={}", uid);
                Some(uid)
            } else {
                None
            }
        }
    };
    let id = match id { Some(i) => i, None => return Ok(None) };
    let pool = db_pool(&app)?;
    let user = UsersRepository::get_by_id(&pool, id).await.map_err(|e| e.to_string())?;
    // §P1-B13 (audit 2026-08-23): a banned user (is_active = 0) keeps their
    // auth_sessions row until expiry, so they would remain "logged in" forever
    // even after admin disabled them. Force-evict the session here so the
    // banned user is logged out immediately and the client receives `None`.
    if let Some(ref u) = user {
        if u.is_active == 0 {
            info!(
                "[get_current] user_id={} is inactive, evicting session",
                u.id
            );
            sessions.map.lock().unwrap().remove(&session);
            let _ = delete_session_in_db(&app, &session).await;
            return Ok(None);
        }
    }
    Ok(user.map(UserPublic::from))
}

#[tauri::command]
pub async fn user_logout<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
    session: String,
) -> Result<bool, String> {
    // §P1-B13 (audit 2026-08-23): the previous code swallowed DB errors so a
    // failed DELETE on auth_sessions left the session valid in the DB. The
    // in-memory SessionStore would still clear it but a fresh `lookup_session_in_db`
    // would resurrect the user. Surface DB errors so the client can decide
    // whether to retry.
    delete_session_in_db(&app, &session).await?;
    let mut m = sessions.map.lock().unwrap();
    Ok(m.remove(&session).is_some())
}

#[tauri::command]
pub fn system_machine_id() -> String {
    get_machine_id()
}

// §P1-B12 (audit 2026-08-23): user_activate_member is a self-service Pro
// upgrade backdoor — any logged-in user could call it via the 5-tap hidden
// button on /account (frontend/src/app/account/page.tsx). The command is no
// longer registered in the Tauri invoke_handler, so calling it from a client
// is impossible. The function body is kept under `#[cfg(test)]` so a future
// developer can reintroduce it deliberately, but the production path no
// longer exposes it. Also remove the 5-tap UI handler.
#[cfg(test)]
#[tauri::command]
pub async fn user_activate_member<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
    session: String,
) -> Result<UserPublic, String> {
    let user_id = {
        let m = sessions.map.lock().unwrap();
        m.get(&session).copied()
    };
    let id = match user_id { Some(i) => i, None => return Err("not_logged_in".into()) };
    let pool = db_pool(&app)?;
    let machine_id = get_machine_id();
    activate_member_for_user(&pool, id, &machine_id).await
        .map_err(|e| format!("activation_failed: {}", e))?;
    let user = UsersRepository::get_by_id(&pool, id).await.map_err(|e| e.to_string())?
        .ok_or("user gone")?;
    Ok(UserPublic::from(user))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HotwordsConfig {
    pub builtin: String,
    pub custom: String,
    pub enabled: bool,
}

#[tauri::command]
pub async fn hotwords_get<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
    session: String,
) -> Result<HotwordsConfig, String> {
    let user_id = {
        let m = sessions.map.lock().unwrap();
        m.get(&session).copied()
    };
    let id = match user_id { Some(i) => i, None => return Ok(HotwordsConfig {
        builtin: "none".into(), custom: "".into(), enabled: false,
    })};
    let pool = db_pool(&app)?;
    let (b, c, e) = HotwordsRepository::get(&pool, id).await.map_err(|e| e.to_string())?;
    Ok(HotwordsConfig { builtin: b, custom: c, enabled: e })
}

#[tauri::command]
pub async fn hotwords_save<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
    session: String,
    builtin: String,
    custom: String,
    enabled: bool,
) -> Result<(), String> {
    let user_id = {
        let m = sessions.map.lock().unwrap();
        m.get(&session).copied()
    };
    let id = match user_id { Some(i) => i, None => return Err("not_logged_in".into()) };
    let pool = db_pool(&app)?;
    HotwordsRepository::upsert(&pool, id, &builtin, &custom, enabled)
        .await.map_err(|e| e.to_string())?;
    Ok(())
}


/// §91 P0-P2 hotwords: 列出所有可用 pack + 词数, UI 渲染下拉.
#[tauri::command]
pub async fn hotwords_list_packs() -> Result<Vec<serde_json::Value>, String> {
    use std::path::PathBuf;
    use std::fs;
    // hotwords_data/ 路径: src-tauri/scripts/hotwords_data/ (运行时从 src/ 旁)
    let candidates = vec![
        PathBuf::from("scripts/hotwords_data"),
        PathBuf::from("../scripts/hotwords_data"),
        PathBuf::from("frontend/src-tauri/scripts/hotwords_data"),
    ];
    let data_dir = candidates.into_iter().find(|p| p.exists()).unwrap_or_else(|| PathBuf::from("scripts/hotwords_data"));
    let packs = vec![
        ("none", "hotwords.none", 0, ""),
        ("general", "THUOCL IT/技术工程", 300, "Apache-2.0"),
        ("legal", "LaWGPT + THUOCL 法律精选", 538, "Apache-2.0 + MIT"),
        ("medical", "OMAHA + THUOCL 医疗精选", 488, "CC-BY-4.0 + Apache-2.0"),
        ("finance", "THUOCL 财经", 176, "Apache-2.0"),
    ];
    let mut out = vec![];
    for (id, name, count, license) in packs {
        // 验证文件存在
        let json_path = match id {
            "general" => data_dir.join("thuocl_it.json"),
            "legal" => data_dir.join("lawgpt_legal_vocab.json"),
            "medical" => data_dir.join("omaha_medical.json"),
            "finance" => data_dir.join("thuocl_caijing.json"),
            _ => continue,
        };
        if !json_path.exists() {
            continue;
        }
        out.push(serde_json::json!({
            "id": id,
            "name": name,
            "word_count": count,
            "license": license,
        }));
    }
    let _ = fs::metadata(&data_dir); // 避免 unused warning
    Ok(out)
}

#[tauri::command]
pub async fn hotwords_set_globals(pack: String, custom: String) -> Result<(), String> {
    crate::audio::hotwords_globals::set(pack, custom);
    Ok(())
}

pub const fn member_bundle_key() -> &'static str {
    MEMBER_BUNDLE_KEY
}

// Re-export

// ============================================================================
// v0.6.10+: 商业化 commands (C1+C2+C3+C7)
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct QuotaStatusCmd {
    pub tier: String,
    pub month_meetings_used: i64,
    pub month_meetings_limit: i64,
    pub segments_per_transcript_limit: i64,
    pub can_record: bool,
    pub reason: Option<String>,
}

/// C2: 获取当前用户配额状态
#[tauri::command]
pub async fn quota_get_status<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
    session: Option<String>,
) -> Result<QuotaStatusCmd, String> {
    let user_id = match session.and_then(|s| sessions.map.lock().unwrap().get(&s).copied()) {
        Some(id) => id,
        None => {
            // 匿名用户: 用 IndexedDB 数 meetings 数. 后端无法追踪匿名, 这里返 quota.anonymous
            return Ok(QuotaStatusCmd {
                tier: "anonymous".into(),
                month_meetings_used: 0,  // 前端自己估
                month_meetings_limit: crate::user::quota::ANONYMOUS_FREE_RECORDINGS,
                segments_per_transcript_limit: crate::user::quota::FREE_SEGMENTS_PER_TRANSCRIPT_LIMIT,
                can_record: true,
                reason: None,
            });
        }
    };
    let pool = db_pool(&app)?;
    let (existing_key, membership, used) = UsersRepository::get_quota(&pool, user_id).await
        .map_err(|e| e.to_string())?;
    let current_month = crate::user::quota::current_month_key();
    // 跨月重置
    let effective_used = if existing_key == current_month { used } else { 0 };
    let status = crate::user::quota::compute_quota(&membership, effective_used);
    Ok(QuotaStatusCmd {
        tier: status.tier,
        month_meetings_used: status.month_meetings_used,
        month_meetings_limit: status.month_meetings_limit,
        segments_per_transcript_limit: status.segments_per_transcript_limit,
        can_record: status.can_record,
        reason: status.reason,
    })
}

/// C2: 录制成功后调用, +1 计数 (跨月重置)
#[tauri::command]
pub async fn quota_increment_after_record<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
    session: Option<String>,
) -> Result<QuotaStatusCmd, String> {
    let user_id = match session.and_then(|s| sessions.map.lock().unwrap().get(&s).copied()) {
        Some(id) => id,
        None => return Err("not_logged_in".into()),
    };
    let pool = db_pool(&app)?;
    let current_month = crate::user::quota::current_month_key();
    let _new_used = UsersRepository::increment_monthly_meetings(&pool, user_id, &current_month)
        .await.map_err(|e| e.to_string())?;
    let (_, membership, used) = UsersRepository::get_quota(&pool, user_id).await
        .map_err(|e| e.to_string())?;
    let status = crate::user::quota::compute_quota(&membership, used);
    Ok(QuotaStatusCmd {
        tier: status.tier,
        month_meetings_used: status.month_meetings_used,
        month_meetings_limit: status.month_meetings_limit,
        segments_per_transcript_limit: status.segments_per_transcript_limit,
        can_record: status.can_record,
        reason: status.reason,
    })
}

/// C3: 用户点 "我想升级" 时记录意向 + 返回支付指南
#[tauri::command]
pub async fn lead_record_upgrade<R: Runtime>(
    app: AppHandle<R>,
    email: String,
    contact: Option<String>,
    note: Option<String>,
) -> Result<i64, String> {
    let pool = db_pool(&app)?;
    user_repo::UpgradeLeadsRepository::create(
        &pool,
        &email,
        contact.as_deref(),
        note.as_deref(),
    ).await.map_err(|e| e.to_string())
}

/// C3+C7: admin 手动激活 (用 operator_token 鉴权)
#[derive(Debug, Deserialize)]
pub struct AdminActivateRequest {
    pub operator_token: String,
    pub email: String,
    pub channel: String,         // 'wxpay' | 'usdt' | 'card' | 'admin_grant'
    pub amount_cents: i64,
    pub proof: Option<String>,
    pub notes: Option<String>,
}

/// Admin token 鉴权
/// 生产环境: 必须 ADMIN_OPERATOR_TOKEN 匹配
/// dev 环境 (debug build 或显式 LIXIANHUIJI_DEV_MODE=1): 任何非空 token 通过
fn check_admin_token(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    if let Ok(expected) = std::env::var("ADMIN_OPERATOR_TOKEN") {
        if !expected.is_empty() && token == expected {
            return true;
        }
    }
    let dev_mode_explicit =
        std::env::var("LIXIANHUIJI_DEV_MODE").map(|v| v == "1" || v == "true").unwrap_or(false);
    cfg!(debug_assertions) || dev_mode_explicit
}

#[tauri::command]
pub async fn admin_activate_member<R: Runtime>(
    app: AppHandle<R>,
    req: AdminActivateRequest,
) -> Result<bool, String> {
    if !check_admin_token(&req.operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    // 通过 email 找 user_id
    let user = UsersRepository::get_by_email(&pool, &req.email).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "user_not_found".to_string())?;
    // 拿 machine_id (如果有)
    let machine_id = user.machine_id.clone().unwrap_or_else(|| "manual".to_string());
    // 激活
    activate_member_for_user(&pool, user.id, &machine_id).await
        .map_err(|e| e.to_string())?;
    // 记录 order
    user_repo::ActivationOrdersRepository::create(
        &pool,
        &req.email,
        req.amount_cents,
        &req.channel,
        req.proof.as_deref(),
        "admin",
        req.notes.as_deref(),
    ).await.map_err(|e| e.to_string())?;
    Ok(true)
}

/// C7: admin 看激活订单 (前端 admin 后台用)
#[tauri::command]
pub async fn admin_list_activation_orders<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    limit: Option<i64>,
) -> Result<Vec<(i64, String, i64, String, Option<String>, Option<String>, String, Option<String>)>, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    user_repo::ActivationOrdersRepository::list_all(&pool, limit.unwrap_or(100)).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn admin_list_upgrade_leads<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    limit: Option<i64>,
) -> Result<Vec<(i64, String, Option<String>, String)>, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    user_repo::UpgradeLeadsRepository::list_recent(&pool, limit.unwrap_or(100)).await
        .map_err(|e| e.to_string())
}


// ─────────────────────────────────────────────────────────────
// C4: Pro 激活码 (v0.6.10+)
// ─────────────────────────────────────────────────────────────

/// 兑换结果. 成功时返 Pro 有效期到期时间.
#[derive(Debug, Serialize, Deserialize)]
pub struct RedeemResult {
    pub success: bool,
    pub tier: String,
    pub expires_at: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

/// C4 admin: 批量生残激活码.
/// - count: 生残几条 (1..=200)
/// - tier: 'member' (Pro 买断)
/// - duration_days: 有效期天数 (默认 365)
/// - operator_token: admin 权限 (复用 C7 §16 修复后的 check_admin_token)
#[tauri::command]
pub async fn admin_generate_activation_codes<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    count: i64,
    tier: String,
    duration_days: Option<i64>,
    note: Option<String>,
) -> Result<Vec<String>, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    if count < 1 || count > 200 {
        return Err(format!("count 必须在 1..=200 之间, 当前 {count}"));
    }
    let tier = match tier.as_str() {
        "member" | "pro" => "member".to_string(),
        other => return Err(format!("不支持的 tier: {other}")),
    };
    let dur = duration_days.unwrap_or(365);
    let pool = db_pool(&app)?;
    let operator = check_admin_email(&operator_token);

    let mut codes = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let code = crate::user::activation_code::generate_code();
        // 默认 +30 天 grace 后过期
        let exp_secs = chrono::Utc::now().timestamp()
            + dur * 86_400
            + 30 * 86_400;
        let expires_at = chrono::DateTime::<chrono::Utc>::from_timestamp(exp_secs, 0)
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(dur as i64 + 30))
            .to_rfc3339();

        user_repo::ActivationCodesRepository::insert(
            &pool,
            &code,
            &tier,
            dur,
            &expires_at,
            operator.as_deref(),
            note.as_deref(),
        )
        .await
        .map_err(|e| format!("DB insert 失败: {e}"))?;

        codes.push(crate::user::activation_code::mask_for_display(&code));
    }

    Ok(codes)
}

/// C4 admin: 列出全部激活码 (脱敏, 不可见 secret).
#[tauri::command]
pub async fn admin_list_activation_codes<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<user_repo::ActivationCodeRow>, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    user_repo::ActivationCodesRepository::list_paginated(
        &pool,
        limit.unwrap_or(50),
        offset.unwrap_or(0),
    )
    .await
    .map_err(|e| e.to_string())
}

/// C4 admin: 撤销 (删除) 未使用的激活码.
#[tauri::command]
pub async fn admin_revoke_activation_code<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    code: String,
) -> Result<u64, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    user_repo::ActivationCodesRepository::revoke_unused(&pool, &code)
        .await
        .map_err(|e| e.to_string())
}

/// C4 client: 兑换激活码.
/// 用户已登录.
#[tauri::command]
pub async fn user_redeem_activation_code<R: Runtime>(
    app: AppHandle<R>,
    sessions: tauri::State<'_, SessionStore>,
    session: String,
    code: String,
) -> Result<RedeemResult, String> {
    // 1) session 校验
    let user_id = {
        let m = sessions.map.lock().unwrap();
        m.get(&session).copied()
    };
    let user_id = match user_id { Some(i) => i, None => return Err("not_logged_in".into()) };

    // 2) 格式 + checksum
    let normalized = match crate::user::activation_code::validate_code(&code) {
        Ok(s) => s,
        Err(e) => {
            return Ok(RedeemResult {
                success: false,
                tier: String::new(),
                expires_at: String::new(),
                error_code: Some("invalid_format".into()),
                error_message: Some(format!("格式错误: {e}")),
            });
        }
    };

    let pool = db_pool(&app)?;

    // 3) DB 查
    let row = match user_repo::ActivationCodesRepository::find_by_code(&pool, &normalized).await {
        Ok(Some(r)) => r,
        Ok(None) => return Ok(RedeemResult {
            success: false,
            tier: String::new(),
            expires_at: String::new(),
            error_code: Some("not_found".into()),
            error_message: Some("激活码不存在, 请检查拼写或联系客服".into()),
        }),
        Err(e) => return Ok(RedeemResult {
            success: false,
            tier: String::new(),
            expires_at: String::new(),
            error_code: Some("db_error".into()),
            error_message: Some(format!("DB 错误: {e}")),
        }),
    };

    // 4) 已过期?
    let now = chrono::Utc::now();
    let exp = chrono::DateTime::parse_from_rfc3339(&row.expires_at)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or(now);
    if exp < now {
        return Ok(RedeemResult {
            success: false,
            tier: row.tier,
            expires_at: row.expires_at,
            error_code: Some("expired".into()),
            error_message: Some("激活码已过期, 请联系客服换发新码".into()),
        });
    }

    // 5) 已被用过?
    if row.used_by_user_id.is_some() {
        return Ok(RedeemResult {
            success: false,
            tier: row.tier,
            expires_at: row.expires_at,
            error_code: Some("already_used".into()),
            error_message: Some(format!(
                "激活码已被使用 ({})",
                row.used_at.unwrap_or_default()
            )),
        });
    }

    // 5b) v0.7.0+: 当前用户的 machine_id 与激活码绑定的 machine_id 必须一致.
    // 首次兑换 (row.bound_machine_id IS NULL): 写入当前 machine_id 锁死.
    // 后续兑换 (已有 bound_machine_id): 必须一致, 否则拒绝 (防止一码多机).
    let current_machine_id = get_machine_id();
    if let Some(bound) = row.bound_machine_id.as_deref() {
        if !bound.is_empty() && bound != current_machine_id {
            return Ok(RedeemResult {
                success: false,
                tier: row.tier,
                expires_at: row.expires_at,
                error_code: Some("machine_mismatch".into()),
                error_message: Some(
                    "此激活码已绑定到其他设备, 请联系客服解绑 (admin_unbind_machine)".into()
                ),
            });
        }
    }

    // 6+7) §P1-B13 (audit 2026-08-23): the previous two-step non-transactional
    // path could mark the code consumed (step 6) and then fail to upgrade the
    // user (step 7), permanently burning the code without granting membership.
    // Wrap both updates in a single sqlx transaction so they commit together
    // or roll back together.
    let now_iso = chrono::Utc::now().to_rfc3339();
    let mut tx = pool.begin().await.map_err(|e| format!("DB begin tx: {e}"))?;
    let affected = sqlx::query(
        "UPDATE activation_codes SET used_by_user_id = ?1, used_at = ?2, bound_machine_id = ?3
         WHERE code = ?4 AND used_by_user_id IS NULL",
    )
    .bind(user_id)
    .bind(&now_iso)
    .bind(&current_machine_id)
    .bind(&normalized)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        // Best-effort rollback; if it fails we still surface the original error.
        let _ = tokio::runtime::Handle::try_current().map(|_| ());
        format!("DB mark_used: {e}")
    })?
    .rows_affected();
    if affected == 0 {
        // 并发竞争: 别人刚拿走 — no updates were made, no need to commit.
        tx.rollback().await.map_err(|e| format!("DB rollback: {e}"))?;
        return Ok(RedeemResult {
            success: false,
            tier: row.tier,
            expires_at: row.expires_at,
            error_code: Some("already_used".into()),
            error_message: Some("激活码已被使用 (并发竞争)".into()),
        });
    }

    // 升级用户到 member (同时绑定 machine_id, 首次激活必备)
    let upgrade_result = sqlx::query(
        "UPDATE users
         SET membership = ?, membership_activated_at = ?, license_key = ?, activated_via_code = ?,
             machine_id = COALESCE(machine_id, ?)
         WHERE id = ?",
    )
    .bind(&row.tier)
    .bind(&now_iso)
    .bind(&normalized)
    .bind(&normalized)
    .bind(&current_machine_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await;
    match upgrade_result {
        Ok(_) => {
            tx.commit().await.map_err(|e| format!("DB commit: {e}"))?;
        }
        Err(e) => {
            // If the upgrade fails we MUST roll back the code consumption so the
            // user can retry with the same code.
            let _ = tx.rollback().await;
            return Err(format!("DB upgrade user: {e}"));
        }
    }

    Ok(RedeemResult {
        success: true,
        tier: row.tier,
        expires_at: row.expires_at,
        error_code: None,
        error_message: None,
    })
}

/// helper: 把 admin token (env var) 映射为 email 描述 (供 DB audit). dev 下没设 env 时返 None.
fn check_admin_email(_token: &str) -> Option<String> {
    None // 当前 env 没存 user→email 映射; 全用 None, 不阻塞审计数据
}

// ============================================================================
// v0.7.0+: 商业化运营 admin 命令 (退款 / 解绑 / 封号 / 配额)
// ============================================================================

/// admin 用户管理 - 列出全部用户 (退款/封号前先看清单)
#[tauri::command]
pub async fn admin_list_users<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    limit: Option<i64>,
) -> Result<Vec<AdminUserRow>, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    let lim = limit.unwrap_or(50).clamp(1, 500);
    let rows = UsersRepository::list_users_admin(&pool, lim).await
        .map_err(|e| format!("DB list_users: {e}"))?;
    Ok(rows.into_iter().map(|(id, email, membership, machine_id, used, active)| AdminUserRow {
        id,
        email,
        membership,
        machine_id: machine_id.unwrap_or_default(),
        month_meetings_used: used,
        is_active: active == 1,
    }).collect())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AdminUserRow {
    pub id: i64,
    pub email: String,
    pub membership: String,
    pub machine_id: String,
    pub month_meetings_used: i64,
    pub is_active: bool,
}

/// admin 退款: 撤销会员资格 (membership='free', 清 license_key)
#[tauri::command]
pub async fn admin_revoke_membership<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    user_id: i64,
) -> Result<bool, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    // 确认用户存在
    let u = UsersRepository::get_by_id(&pool, user_id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "user_not_found".to_string())?;
    if u.membership != "member" {
        return Err(format!("user {} 不是 member (当前: {})", user_id, u.membership));
    }
    UsersRepository::revoke_membership(&pool, user_id).await
        .map_err(|e| format!("DB revoke: {e}"))?;
    log::info!("[admin] revoke_membership user_id={} email={}", user_id, u.email);
    Ok(true)
}

/// admin 解绑机器 (用户换电脑后可重新激活)
#[tauri::command]
pub async fn admin_unbind_machine<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    user_id: i64,
) -> Result<bool, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    let u = UsersRepository::get_by_id(&pool, user_id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "user_not_found".to_string())?;
    UsersRepository::unbind_machine(&pool, user_id).await
        .map_err(|e| format!("DB unbind: {e}"))?;
    log::info!("[admin] unbind_machine user_id={} email={} old_machine={:?}",
        user_id, u.email, u.machine_id);
    Ok(true)
}

/// admin 封号 / 解封
#[tauri::command]
pub async fn admin_set_user_active<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    user_id: i64,
    active: bool,
) -> Result<bool, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    let u = UsersRepository::get_by_id(&pool, user_id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "user_not_found".to_string())?;
    UsersRepository::set_active(&pool, user_id, active).await
        .map_err(|e| format!("DB set_active: {e}"))?;
    log::info!("[admin] set_user_active user_id={} email={} active={}", user_id, u.email, active);
    Ok(true)
}

/// admin 重置本月配额 (客服手动刷新免费用户月额度)
#[tauri::command]
pub async fn admin_reset_user_quota<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    user_id: i64,
) -> Result<bool, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    let u = UsersRepository::get_by_id(&pool, user_id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "user_not_found".to_string())?;
    UsersRepository::reset_month_quota(&pool, user_id).await
        .map_err(|e| format!("DB reset_quota: {e}"))?;
    log::info!("[admin] reset_user_quota user_id={} email={}", user_id, u.email);
    Ok(true)
}

/// v0.7.0+: 退款 + 解绑一键 (客服最常见操作)
#[tauri::command]
pub async fn admin_refund_user<R: Runtime>(
    app: AppHandle<R>,
    operator_token: String,
    user_id: i64,
    reason: Option<String>,
) -> Result<bool, String> {
    if !check_admin_token(&operator_token) {
        return Err("unauthorized".into());
    }
    let pool = db_pool(&app)?;
    let u = UsersRepository::get_by_id(&pool, user_id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "user_not_found".to_string())?;
    if u.membership != "member" {
        return Err(format!("user {} 不是 member, 无需退款", user_id));
    }
    UsersRepository::revoke_membership(&pool, user_id).await
        .map_err(|e| format!("DB revoke: {e}"))?;
    UsersRepository::unbind_machine(&pool, user_id).await
        .map_err(|e| format!("DB unbind: {e}"))?;
    log::info!("[admin] refund_user user_id={} email={} reason={:?}",
        user_id, u.email, reason);
    Ok(true)
}

