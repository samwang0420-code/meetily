//! Pro 激活码生成 / 兑换 / 校验 (纯逻辑, 不依赖 DB / sidecar)
//!
//! 码格式: `PROMO-XXXXXXXX-YYYY`
//! - prefix: PROMO (5 字符)
//! - secret: XXXXXXXX (8 字符, Crockford base32, ≈ 40 bit 熵, 1T 空间)
//! - checksum: YYYY (4 字符, FNV-1a + base32, 检测拼写错)
//!
//! 鉴权: 不依赖登录态, code 本身就是权限 (gift card 模式).

use base32::{Alphabet, encode as b32_encode};

const PREFIX: &str = "PROMO";
const SECRET_LEN: usize = 8;
const CHECKSUM_LEN: usize = 4;
const ALPHABET: Alphabet = Alphabet::Crockford;
const CROCKFORD_BYTES: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn is_crockford_char(c: char) -> bool {
    CROCKFORD_BYTES.iter().any(|&b| b == c as u8)
}

/// 生成一个新激活码 (admin 后台用)
pub fn generate_code() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 6]; // 48 bit 随机源
    rand::thread_rng().fill_bytes(&mut bytes);
    let encoded = b32_encode(ALPHABET, &bytes);
    let secret: String = encoded.chars().filter(|c| *c != '=').take(SECRET_LEN).collect();
    // secret 必须是 8 字符, 不够补 base32 头部几个字符
    let padding_needed = SECRET_LEN.saturating_sub(secret.chars().count());
    let secret = if padding_needed > 0 {
        format!("{}{}", &"0".repeat(padding_needed), secret)
    } else {
        secret
    };
    let secret = secret.chars().take(SECRET_LEN).collect::<String>();
    assert_eq!(secret.chars().count(), SECRET_LEN);

    let checksum = compute_checksum_4(&secret);
    format!("{PREFIX}-{}-{}", secret.to_ascii_uppercase(), checksum.to_ascii_uppercase())
}

/// 校验激活码格式 + checksum. 大小写无关 + 自动 trim 空格
pub fn validate_code(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_uppercase().replace(' ', "");

    let parts: Vec<&str> = normalized.split('-').collect();
    if parts.len() != 3 {
        return Err(format!(
            "格式错误, 期望 PROMO-XXXXXXXX-YYYY (3 段), 实际 {} 段",
            parts.len()
        ));
    }
    if parts[0] != PREFIX {
        return Err(format!("前缀错误, 期望 {PREFIX}, 实际 {}", parts[0]));
    }
    if parts[1].len() != SECRET_LEN {
        return Err(format!(
            "secret 段长度错误, 期望 {SECRET_LEN}, 实际 {}",
            parts[1].len()
        ));
    }
    if !parts[1].chars().all(is_crockford_char) {
        return Err(format!("secret 段含非 Crockford 字符: {}", parts[1]));
    }
    if parts[2].len() != CHECKSUM_LEN {
        return Err(format!(
            "checksum 段长度错误, 期望 {CHECKSUM_LEN}, 实际 {}",
            parts[2].len()
        ));
    }

    let secret = parts[1];
    let expected = compute_checksum_4(secret);
    let provided = parts[2];
    if expected != provided {
        return Err(format!(
            "checksum 不匹配, 期望 {expected}, 实际 {provided}"
        ));
    }
    Ok(normalized)
}

/// 4 字符 checksum (FNV-1a 64-bit + base32)
fn compute_checksum_4(core: &str) -> String {
    let mut acc: u64 = 0xcbf29ce484222325;
    for b in core.as_bytes() {
        acc = (acc ^ (*b as u64)).wrapping_mul(0x100000001b3);
    }
    let bytes = acc.to_le_bytes();
    let encoded = b32_encode(ALPHABET, &bytes);
    encoded
        .chars()
        .filter(|c| *c != '=')
        .take(CHECKSUM_LEN)
        .collect::<String>()
        .to_ascii_uppercase()
}

/// 显示时遮蔽 secret 段, 防日志泄漏
pub fn mask_for_display(code: &str) -> String {
    match validate_code(code) {
        Ok(normalized) => {
            let parts: Vec<&str> = normalized.split('-').collect();
            if parts.len() != 3 {
                return "****-****-****".to_string();
            }
            format!("{PREFIX}-****-{}", parts[2])
        }
        Err(_) => "****-****-****".to_string(),
    }
}

/// 解码出 8 字符 secret (给 audit 用)
pub fn code_to_secret(code: &str) -> Result<String, String> {
    let normalized = validate_code(code)?;
    let parts: Vec<&str> = normalized.split('-').collect();
    Ok(parts[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn generate_code_format_is_valid() {
        for _ in 0..10 {
            let code = generate_code();
            assert!(validate_code(&code).is_ok(), "generated code should validate, got={code}");
            assert_eq!(code.split('-').count(), 3);
            assert!(code.starts_with("PROMO-"));
        }
    }

    #[test]
    fn validate_rejects_garbage() {
        assert!(validate_code("").is_err());
        assert!(validate_code("hello").is_err());
        assert!(validate_code("PROMO-").is_err());
        assert!(validate_code("PROMO-A").is_err());
        assert!(validate_code("XXXX-AAAAAAAA-BBBB").is_err()); // wrong prefix
        assert!(validate_code("PROMO-1234567-BCDE").is_err()); // secret too short
        assert!(validate_code("PROMO-123456789-BCDE").is_err()); // secret too long
        assert!(validate_code("PROMO-12345678-BC").is_err()); // checksum too short
    }

    #[test]
    fn validate_is_case_insensitive() {
        let code = generate_code();
        let lower: String = code.to_ascii_lowercase();
        assert!(validate_code(&lower).is_ok(), "lowercase form should work, got={}", code);
    }

    #[test]
    fn validate_accepts_with_spaces() {
        let code = generate_code();
        let spaced_with_inner = code.replace('-', " - ");
        assert!(validate_code(&spaced_with_inner).is_ok());
        assert!(validate_code(&format!(" {} ", code)).is_ok());
    }

    #[test]
    fn checksum_catches_bitflip() {
        let code = generate_code();
        // 翻转 secret 段某字符 (用合法的下一个 Crockford 字符)
        let c0 = code.chars().nth(6).unwrap(); // secret 第 1 个字符
        let next_idx = (CROCKFORD_BYTES.iter().position(|&b| b == c0 as u8).unwrap_or(0) + 1) % 32;
        let next_char = CROCKFORD_BYTES[next_idx] as char;
        let mut tampered: String = code.chars().collect();
        tampered.replace_range(6..7, &next_char.to_string());
        let r = validate_code(&tampered);
        assert!(r.is_err(), "tampered code should not validate: got {r:?}");
    }

    #[test]
    fn mask_for_display_hides_secret() {
        let code = generate_code();
        let masked = mask_for_display(&code);
        assert!(masked.starts_with("PROMO-****-"));
        assert!(!masked.contains(&code[6..14]), "secret should be hidden");
    }

    #[test]
    fn collision_rate_1000_gen() {
        let mut set = HashSet::new();
        for _ in 0..1000 {
            let c = generate_code();
            assert!(!set.contains(&c), "collision in 1k: {c}");
            set.insert(c);
        }
        assert_eq!(set.len(), 1000);
    }

    #[test]
    fn code_to_secret_extracts_8_chars() {
        let code = generate_code();
        let secret = code_to_secret(&code).unwrap();
        assert_eq!(secret.chars().count(), 8);
    }

    #[test]
    fn crockford_alphabet_no_io_confusion() {
        // 关键安全断言: O/0/I/1/L 排除歧义字符
        assert!(!is_crockford_char('O'));
        assert!(!is_crockford_char('I'));
        assert!(!is_crockford_char('L'));
        assert!(is_crockford_char('0'));
        assert!(is_crockford_char('1'));
    }
}
