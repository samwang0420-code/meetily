// 离线会记 v0.5.0: 用户管理 + 会员模块
pub mod auth;
pub mod membership;
pub mod machine_id;
pub mod commands;
// v0.7.x: re-export for backend usage (api.rs 配额截断)
pub use commands::SessionStore;
// v0.6.10+: 商业化模块
pub mod quota;
pub mod activation_code;
