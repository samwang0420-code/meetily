// 离线会记 v0.5.0: 机器识别码 (跨平台)
// 基于硬件指纹: hostname + 主板/机器 UUID
// macOS: IOPlatformUUID (ioreg)
// Linux: /etc/machine-id + DMI product_uuid
// Windows: wmic csproduct get uuid + hostname
// 用 SHA-256 截取, 形成稳定的 16-char ID

use sha2::{Digest, Sha256};
use std::process::Command;
use log::warn;

#[cfg(target_os = "macos")]
fn read_platform_uuid() -> Option<String> {
    let out = Command::new("ioreg")
        .arg("-rd1").arg("-c").arg("IOPlatformExpertDevice")
        .output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        if line.contains("IOPlatformUUID") {
            return line.split('"').nth(3).map(|s| s.to_string());
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn read_platform_uuid() -> Option<String> {
    // 优先 machine-id (systemd, 几乎所有现代 Linux 都有)
    if let Ok(s) = std::fs::read_to_string("/etc/machine-id") {
        let t = s.trim();
        if !t.is_empty() { return Some(t.to_string()); }
    }
    // fallback: DMI product_uuid (root only, 但尝试)
    if let Ok(out) = Command::new("sh")
        .arg("-c").arg("cat /sys/class/dmi/id/product_uuid 2>/dev/null")
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() && s.chars().any(|c| c != '\0' && c != ' ') {
            return Some(s);
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn read_platform_uuid() -> Option<String> {
    // wmic 在 Win10 1809+ 可能被移除, PowerShell 更可靠
    let ps = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-Command")
        .arg("(Get-CimInstance -Class Win32_ComputerSystemProduct).UUID")
        .output();
    if let Ok(out) = ps {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }
    // fallback: wmic (Win10 早期)
    let wmic = Command::new("wmic")
        .arg("csproduct").arg("get").arg("uuid")
        .output();
    if let Ok(out) = wmic {
        let s = String::from_utf8_lossy(&out.stdout);
        // 跳过 "UUID" 标题行
        for line in s.lines().skip(1) {
            let t = line.trim();
            if !t.is_empty() { return Some(t.to_string()); }
        }
    }
    None
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn read_platform_uuid() -> Option<String> { None }

fn read_hostname() -> Option<String> {
    Command::new("hostname").output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn get_machine_id() -> String {
    let mut inputs: Vec<String> = Vec::new();
    if let Some(u) = read_platform_uuid() { inputs.push(u); }
    if let Some(h) = read_hostname() { inputs.push(h); }
    if inputs.is_empty() {
        warn!("[machine_id] failed to read system identifiers, fallback to random per-session");
        return format!("fallback-{}", rand_short());
    }
    let joined = inputs.join("|");
    let mut h = Sha256::new();
    h.update(joined.as_bytes());
    let full = format!("{:x}", h.finalize());
    // 16-char human-readable: 8 + dash + 8
    format!("{}-{}", &full[0..8], &full[8..16])
}

fn rand_short() -> String {
    use rand::Rng;
    let r: [u8; 4] = rand::thread_rng().gen();
    format!("{:x}", u32::from_le_bytes(r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_machine_id_is_stable() {
        let a = get_machine_id();
        let b = get_machine_id();
        // 正常情况下两次调用应该完全一致 (除非真 fallback 随机)
        if !a.starts_with("fallback-") {
            assert_eq!(a, b, "machine_id 应该是稳定的");
        }
    }

    #[test]
    fn test_get_machine_id_format() {
        let id = get_machine_id();
        // 8-dash-8 = 17 字符, 或 fallback-XXXX (短随机)
        assert!(
            id.starts_with("fallback-") || id.len() == 17,
            "machine_id 格式不对: {}", id
        );
    }

    #[test]
    fn test_read_hostname_nonempty() {
        if let Some(h) = read_hostname() {
            assert!(!h.is_empty());
        }
    }
}
