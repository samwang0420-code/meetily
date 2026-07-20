// 离线会记 v0.5.0: 认证 (注册/登录/密码)
// 简化: SHA-256(password + salt), 无 email 验证, 无密码重置
// 风险评估: 与本地单机账户一致 (用户数据全本地), 无云, 密码强度要求 ≥6

use sha2::{Digest, Sha256};

pub fn hash_password(plain: &str, salt: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update(plain.as_bytes());
    format!("{:x}", h.finalize())
}

pub fn verify_password(plain: &str, salt: &str, expected_hash: &str) -> bool {
    hash_password(plain, salt).as_str() == expected_hash
}

pub fn gen_salt() -> String {
    use rand::Rng;
    let r: [u8; 16] = rand::thread_rng().gen();
    let mut hex = String::with_capacity(32);
    for b in r.iter() { hex.push_str(&format!("{:02x}", b)); }
    hex
}

pub fn validate_email(s: &str) -> bool {
    // 简化校验: 必须有 @, @ 后必须有 ., 至少 5 字符
    if s.len() < 5 { return false; }
    let at_pos = s.find('@');
    if at_pos.is_none() { return false; }
    let at = at_pos.unwrap();
    if at == 0 || at == s.len() - 1 { return false; }
    s[at+1..].contains('.')
}

pub fn validate_password(p: &str) -> Result<(), &'static str> {
    if p.len() < 6 { return Err("weak_password"); }
    if p.len() > 128 { return Err("password_too_long"); }
    Ok(())
}
