// 离线会记 v0.5.0: 用户账号 repository
// 字段保持最小化, 与原 Meetily licensing 表语义对齐

use chrono::{DateTime, Utc};

fn map_row(r: sqlx::sqlite::SqliteRow) -> crate::database::models::UserModel {
    use sqlx::Row;
    let id: i64 = r.get("id");
    let email: String = r.get("email");
    let display_name: Option<String> = r.get("display_name");
    let created_at_s: String = r.get("created_at");
    let last_login_at_s: Option<String> = r.get("last_login_at");
    let membership: String = r.get("membership");
    let membership_activated_at_s: Option<String> = r.get("membership_activated_at");
    let license_key: Option<String> = r.get("license_key");
    let machine_id: Option<String> = r.get("machine_id");
    let is_active: i64 = r.get("is_active");
    crate::database::models::UserModel {
        id, email, display_name,
        created_at: parse_dt(&created_at_s),
        last_login_at: last_login_at_s.as_deref().and_then(parse_dt_opt),
        membership,
        membership_activated_at: membership_activated_at_s.as_deref().and_then(parse_dt_opt),
        license_key,
        machine_id,
        is_active,
    }
}

fn parse_dt(s: &str) -> crate::database::models::DateTimeUtc {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| crate::database::models::DateTimeUtc(d.with_timezone(&chrono::Utc)))
        .unwrap_or_else(|_| crate::database::models::DateTimeUtc(chrono::Utc::now()))
}
fn parse_dt_opt(s: &str) -> Option<crate::database::models::DateTimeUtc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| crate::database::models::DateTimeUtc(d.with_timezone(&chrono::Utc)))
}

use serde::{Deserialize, Serialize};
use sqlx::{Error as SqlxError, Row, SqlitePool};



#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserPublic {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub membership: String,
    pub member_since: Option<String>,
    pub machine_id: Option<String>,
}

impl From<crate::database::models::UserModel> for UserPublic {
    fn from(u: crate::database::models::UserModel) -> Self {
        UserPublic {
            id: u.id,
            email: u.email,
            display_name: u.display_name,
            membership: u.membership,
            member_since: u.membership_activated_at.map(|d| format!("{:?}", d)),
            machine_id: u.machine_id,
        }
    }
}

pub struct UsersRepository;

impl UsersRepository {
    pub async fn create_user(
        pool: &SqlitePool,
        email: &str,
        password_hash: &str,
        salt: &str,
        machine_id: Option<&str>,
    ) -> Result<i64, SqlxError> {
        let now = Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO users (email, password_hash, salt, machine_id, created_at, membership) VALUES (?, ?, ?, ?, ?, 'free')"
        )
        .bind(email)
        .bind(password_hash)
        .bind(salt)
        .bind(machine_id)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_by_email(pool: &SqlitePool, email: &str) -> Result<Option<crate::database::models::UserModel>, SqlxError> {
        let row = sqlx::query("SELECT id, email, display_name, created_at, last_login_at, membership, membership_activated_at, license_key, machine_id, is_active FROM users WHERE email = ?1")
            .bind(email)
            .fetch_optional(pool)
            .await?;
        Ok(row.map(map_row))
    }

    pub async fn get_by_id(pool: &SqlitePool, id: i64) -> Result<Option<crate::database::models::UserModel>, SqlxError> {
        let row = sqlx::query("SELECT id, email, display_name, created_at, last_login_at, membership, membership_activated_at, license_key, machine_id, is_active FROM users WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        Ok(row.map(map_row))
    }

    pub async fn update_last_login(pool: &SqlitePool, id: i64) -> Result<(), SqlxError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE users SET last_login_at = ?1 WHERE id = ?2")
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn activate_membership(
        pool: &SqlitePool,
        id: i64,
        license_key: &str,
    ) -> Result<(), SqlxError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE users SET membership = 'member', membership_activated_at = ?1, license_key = ?2 WHERE id = ?3")
            .bind(&now)
            .bind(license_key)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn set_machine_id(pool: &SqlitePool, id: i64, machine_id: &str) -> Result<(), SqlxError> {
        sqlx::query("UPDATE users SET machine_id = ?1 WHERE id = ?2")
            .bind(machine_id)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// v0.7.0+: 退款 / 撤销会员 (admin). membership='free', 清 license_key + activated_at.
    /// 月配额不重置 (本月已用)。activation_codes 表 used_by_user_id 保留做审计。
    pub async fn revoke_membership(pool: &SqlitePool, id: i64) -> Result<(), SqlxError> {
        sqlx::query(
            "UPDATE users SET membership = 'free', membership_activated_at = NULL, license_key = NULL WHERE id = ?1",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// v0.7.0+: 解绑机器 (admin). machine_id=NULL, 用户可在新机器上重新登录激活。
    /// 不动 membership — 只是解绑硬件, 会员保留。
    pub async fn unbind_machine(pool: &SqlitePool, id: i64) -> Result<(), SqlxError> {
        sqlx::query("UPDATE users SET machine_id = NULL WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// v0.7.0+: 封号 / 解封 (admin). is_active=0 → 登录时直接拒。
    pub async fn set_active(pool: &SqlitePool, id: i64, active: bool) -> Result<(), SqlxError> {
        sqlx::query("UPDATE users SET is_active = ?1 WHERE id = ?2")
            .bind(if active { 1i64 } else { 0i64 })
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// v0.7.0+: 重置本月配额 (admin). 用于客服手动刷新免费用户月额度。
    pub async fn reset_month_quota(pool: &SqlitePool, id: i64) -> Result<(), SqlxError> {
        sqlx::query("UPDATE users SET month_meetings_used = 0 WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// v0.7.0+: 列出用户 (admin). (id, email, membership, machine_id, month_used, is_active)
    pub async fn list_users_admin(
        pool: &SqlitePool,
        limit: i64,
    ) -> Result<Vec<(i64, String, String, Option<String>, i64, i64)>, SqlxError> {
        let rows: Vec<(i64, String, String, Option<String>, i64, i64)> = sqlx::query_as(
            "SELECT id, email, membership, machine_id, month_meetings_used, is_active
             FROM users ORDER BY id DESC LIMIT ?1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}

pub struct HotwordsRepository;

impl HotwordsRepository {
    pub async fn get(pool: &SqlitePool, user_id: i64) -> Result<(String, String, bool), SqlxError> {
        let row: Option<(String, String, i64)> = sqlx::query_as(
            "SELECT builtin_pack, custom_words, enabled FROM hotwords_config WHERE user_id = ?1"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        Ok(match row {
            Some((b, c, e)) => (b, c, e == 1),
            None => ("none".to_string(), String::new(), false),
        })
    }

    pub async fn upsert(
        pool: &SqlitePool,
        user_id: i64,
        builtin: &str,
        custom: &str,
        enabled: bool,
    ) -> Result<(), SqlxError> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO hotwords_config (user_id, builtin_pack, custom_words, enabled, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(user_id) DO UPDATE SET builtin_pack=excluded.builtin_pack, custom_words=excluded.custom_words, enabled=excluded.enabled, updated_at=excluded.updated_at"
        )
        .bind(user_id)
        .bind(builtin)
        .bind(custom)
        .bind(if enabled { 1i64 } else { 0i64 })
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(())
    }
}

// v0.6.10+: 配额追踪
impl UsersRepository {
    /// v0.7.x: 取 user 的 membership tier. 找不到返 "anonymous".
    pub async fn get_membership(pool: &SqlitePool, user_id: i64) -> Result<String, SqlxError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT membership FROM users WHERE id = ?1"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        Ok(row.map(|(m,)| m).unwrap_or_else(|| "free".into()))
    }

    pub async fn get_quota(
        pool: &SqlitePool,
        user_id: i64,
    ) -> Result<(String, String, i64), SqlxError> {
        let row: Option<(Option<String>, String, i64)> = sqlx::query_as(
            "SELECT month_quota_key, membership, month_meetings_used FROM users WHERE id = ?1"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        Ok(match row {
            Some((k, m, u)) => (k.unwrap_or_default(), m, u),
            None => (String::new(), "free".into(), 0),
        })
    }

    /// 录制成功后 +1 计数. 如果跨月则重置计数.
    pub async fn increment_monthly_meetings(
        pool: &SqlitePool,
        user_id: i64,
        current_month: &str,
    ) -> Result<i64, SqlxError> {
        // 如果 existing key != current_month, 重置并设新 key
        let (existing_key, _m, used) = Self::get_quota(pool, user_id).await?;
        if existing_key != current_month {
            sqlx::query("UPDATE users SET month_quota_key = ?1, month_meetings_used = 1 WHERE id = ?2")
                .bind(current_month)
                .bind(user_id)
                .execute(pool)
                .await?;
            Ok(1)
        } else {
            let new_used = used + 1;
            sqlx::query("UPDATE users SET month_meetings_used = ?1 WHERE id = ?2")
                .bind(new_used)
                .bind(user_id)
                .execute(pool)
                .await?;
            Ok(new_used)
        }
    }
}

// v0.6.10+: 激活订单 (admin 客服手工激活时记录)
pub struct ActivationOrdersRepository;
impl ActivationOrdersRepository {
    pub async fn create(
        pool: &SqlitePool,
        email: &str,
        amount_cents: i64,
        channel: &str,
        proof: Option<&str>,
        operator: &str,
        notes: Option<&str>,
    ) -> Result<i64, SqlxError> {
        let now = chrono::Utc::now().to_rfc3339();
        let r = sqlx::query(
            "INSERT INTO activation_orders (email, tier, amount_cents, currency, channel, proof, operator_email, created_at, notes)
             VALUES (?1, 'member', ?2, 'CNY', ?3, ?4, ?5, ?6, ?7)"
        )
        .bind(email)
        .bind(amount_cents)
        .bind(channel)
        .bind(proof)
        .bind(operator)
        .bind(&now)
        .bind(notes)
        .execute(pool)
        .await?;
        Ok(r.last_insert_rowid())
    }

    pub async fn list_all(
        pool: &SqlitePool,
        limit: i64,
    ) -> Result<Vec<(i64, String, i64, String, Option<String>, Option<String>, String, Option<String>)>, SqlxError> {
        let rows = sqlx::query_as::<_, (i64, String, i64, String, Option<String>, Option<String>, String, Option<String>)>(
            "SELECT id, email, amount_cents, channel, proof, operator_email, created_at, notes
             FROM activation_orders ORDER BY id DESC LIMIT ?1"
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}

// v0.6.10+: Pro 激活码 (C4)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationCodeRow {
    pub id: i64,
    pub code: String,
    pub tier: String,
    pub duration_days: i64,
    pub expires_at: String,
    pub used_by_user_id: Option<i64>,
    pub used_at: Option<String>,
    pub bound_user_id: Option<i64>,
    pub generated_by_operator: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
    /// v0.7.0+: 首次兑换时锁定的 machine_id, 防止一码多机.
    pub bound_machine_id: Option<String>,
}

fn map_activation_code_row(r: sqlx::sqlite::SqliteRow) -> ActivationCodeRow {
    use sqlx::Row;
    ActivationCodeRow {
        id: r.get("id"),
        code: r.get("code"),
        tier: r.get("tier"),
        duration_days: r.get("duration_days"),
        expires_at: r.get("expires_at"),
        used_by_user_id: r.get("used_by_user_id"),
        used_at: r.get("used_at"),
        bound_user_id: r.get("bound_user_id"),
        generated_by_operator: r.get("generated_by_operator"),
        note: r.get("note"),
        created_at: r.get("created_at"),
        bound_machine_id: r.try_get("bound_machine_id").ok().flatten(),
    }
}

pub struct ActivationCodesRepository;
impl ActivationCodesRepository {
    pub async fn insert(
        pool: &SqlitePool,
        code: &str,
        tier: &str,
        duration_days: i64,
        expires_at: &str,
        operator: Option<&str>,
        note: Option<&str>,
    ) -> Result<i64, SqlxError> {
        let r = sqlx::query(
            "INSERT INTO activation_codes (code, tier, duration_days, expires_at, generated_by_operator, note)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(code)
        .bind(tier)
        .bind(duration_days)
        .bind(expires_at)
        .bind(operator)
        .bind(note)
        .execute(pool)
        .await?;
        Ok(r.last_insert_rowid())
    }

    pub async fn find_by_code(
        pool: &SqlitePool,
        code: &str,
    ) -> Result<Option<ActivationCodeRow>, SqlxError> {
        let row_opt = sqlx::query("SELECT * FROM activation_codes WHERE code = ?")
            .bind(code)
            .fetch_optional(pool)
            .await?;
        Ok(row_opt.map(map_activation_code_row))
    }

    pub async fn list_paginated(
        pool: &SqlitePool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ActivationCodeRow>, SqlxError> {
        let rows = sqlx::query(
            "SELECT * FROM activation_codes ORDER BY id DESC LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(map_activation_code_row).collect())
    }

    pub async fn count(pool: &SqlitePool) -> Result<i64, SqlxError> {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM activation_codes")
            .fetch_one(pool).await
    }

    /// 标记已用, 防并发. WHERE used_by_user_id IS NULL 保证原子.
    pub async fn mark_used(
        pool: &SqlitePool,
        code: &str,
        user_id: i64,
        used_at_iso: &str,
    ) -> Result<u64, SqlxError> {
        let r = sqlx::query(
            "UPDATE activation_codes SET used_by_user_id = ?, used_at = ?
             WHERE code = ? AND used_by_user_id IS NULL"
        )
        .bind(user_id)
        .bind(used_at_iso)
        .bind(code)
        .execute(pool)
        .await?;
        Ok(r.rows_affected())
    }

    pub async fn revoke_unused(
        pool: &SqlitePool,
        code: &str,
    ) -> Result<u64, SqlxError> {
        let r = sqlx::query(
            "DELETE FROM activation_codes WHERE code = ? AND used_by_user_id IS NULL"
        )
        .bind(code)
        .execute(pool)
        .await?;
        Ok(r.rows_affected())
    }
}

// v0.6.10+: 升级意向 (用户点 "我想升级" 时记录)
pub struct UpgradeLeadsRepository;
impl UpgradeLeadsRepository {
    pub async fn create(
        pool: &SqlitePool,
        email: &str,
        contact: Option<&str>,
        note: Option<&str>,
    ) -> Result<i64, SqlxError> {
        let now = chrono::Utc::now().to_rfc3339();
        let r = sqlx::query(
            "INSERT INTO upgrade_leads (email, contact, note, created_at, status) VALUES (?1, ?2, ?3, ?4, 'new')"
        )
        .bind(email)
        .bind(contact)
        .bind(note)
        .bind(&now)
        .execute(pool)
        .await?;
        Ok(r.last_insert_rowid())
    }

    pub async fn list_recent(
        pool: &SqlitePool,
        limit: i64,
    ) -> Result<Vec<(i64, String, Option<String>, String)>, SqlxError> {
        let rows = sqlx::query_as::<_, (i64, String, Option<String>, String)>(
            "SELECT id, email, contact, created_at FROM upgrade_leads ORDER BY id DESC LIMIT ?1"
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}
