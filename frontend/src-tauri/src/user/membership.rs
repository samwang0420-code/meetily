// 离线会记 v0.5.0: 会员管理
// ¥88 永久买断, 绑定当前机器, 写入 licensing 表 (永久有效)

use sqlx::SqlitePool;
use crate::database::repositories::user::UsersRepository;

pub const MEMBER_BUNDLE_KEY: &str = "member-bundle-2026-07-v1";

pub async fn activate_member_for_user(
    pool: &SqlitePool,
    user_id: i64,
    machine_id: &str,
) -> Result<(), String> {
    UsersRepository::set_machine_id(pool, user_id, machine_id)
        .await.map_err(|e| format!("db error: {}", e))?;
    UsersRepository::activate_membership(pool, user_id, MEMBER_BUNDLE_KEY)
        .await.map_err(|e| format!("db error: {}", e))?;
    Ok(())
}
