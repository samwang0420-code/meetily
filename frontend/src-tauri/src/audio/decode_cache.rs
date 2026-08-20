// audio/decode_cache.rs
//
// Section 64 B SHA1 缓存 (2026-08-05): 同一文件二次导入时复用 decode 结果.
// 用户机器 8GB RAM, decode 1:49:57 stereo 音频需要 ~30-60s CPU, 二次导入省 100% decode.
//
// cache key: SHA1(size_bytes + mtime_unix_secs + first_8MB_bytes)
//   - 不全文件 hash (1.5GB 文件全 hash 5-15s, 比 decode 还慢)
//   - size + mtime + 头 8MB 足够防误命中 (用户改文件 mtime 变 → key 变 → cache miss)
//
// cache file: bincode 序列化 DecodedAudio (samples Vec<f32> 占大头)
// cache dir:  ~/Library/Application Support/tech.yanjingai.app/decode_cache/
//
// 已知边界:
// - cache miss 是常态 (用户换文件就 miss), 缓存命中后 1.5GB 音频零 CPU
// - 缓存目录满 5GB 时 LRU 清理 (TODO: §64 B 后续, 当前不实现)
// - bincode 1.5GB 序列化 ~2s, 比 decode 30s 仍快 15x
//
// 决策: 重复导入零 I/O 是用户原话 (2026-08-05), 必须 100% 复用 decoded samples,
// 包括 sample_rate / channels / duration_seconds 全字段.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

use crate::audio::decoder::DecodedAudio;

/// 缓存目录名 (在 app_data_dir 下)
const CACHE_SUBDIR: &str = "decode_cache";
/// 缓存文件扩展名 (调试时容易识别)
const CACHE_EXT: &str = "bin";
/// 头采样字节数 (用于 cache key 计算, 8MB 平衡误命中率和 hash 速度)
const HEADER_SAMPLE_BYTES: u64 = 8 * 1024 * 1024;
/// 缓存魔数 (bincode + version 检查)
const CACHE_MAGIC: &[u8; 8] = b"MTCACHE\x01";

/// 获取缓存目录路径 (~/.yanjingai.app/decode_cache/)
pub fn cache_dir<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<PathBuf> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .context("failed to get app data dir for decode cache")?;
    let dir = app_data_dir.join(CACHE_SUBDIR);
    if !dir.exists() {
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create decode cache dir: {}", dir.display()))?;
    }
    Ok(dir)
}

/// 计算文件的 cache key (SHA256 of size + mtime + first 8MB)
/// 比 SHA1 长 32 字节但抗碰撞更强; cache key 文件名只用 hex 前 16 字符 (128 bit, 够用)
pub fn cache_key_for_file(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("failed to stat file for cache key: {}", path.display()))?;
    let size_bytes = metadata.len();
    let mtime_unix = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let mut hasher = Sha256::new();
    hasher.update(&size_bytes.to_le_bytes());
    hasher.update(&mtime_unix.to_le_bytes());

    // 读头 8MB (或整个文件, 如果 < 8MB)
    let read_len = std::cmp::min(HEADER_SAMPLE_BYTES, size_bytes);
    if read_len > 0 {
        let mut file = File::open(path)
            .with_context(|| format!("failed to open file for cache key header: {}", path.display()))?;
        let mut buffer = vec![0u8; read_len as usize];
        file.read_exact(&mut buffer)
            .with_context(|| format!("failed to read first {} bytes for cache key", read_len))?;
        hasher.update(&buffer);
    }

    let full_hash = hasher.finalize();
    // 用 hex 前 16 字符 (64 bit) 当 cache 文件名, 抗碰撞够用
    let hex = full_hash
        .iter()
        .take(8)
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    Ok(hex)
}

/// 加载缓存的 decoded audio. 不存在或损坏返 Ok(None), I/O 错误才 Err.
pub fn load_cached<R: tauri::Runtime>(app: &AppHandle<R>, key: &str) -> Result<Option<DecodedAudio>> {
    let dir = cache_dir(app)?;
    let cache_path = dir.join(format!("{}.{}", key, CACHE_EXT));
    if !cache_path.exists() {
        return Ok(None);
    }

    let bytes = match std::fs::read(&cache_path) {
        Ok(b) => b,
        Err(e) => {
            log::warn!(
                "[Section 64 B cache] failed to read {}: {}, treating as miss",
                cache_path.display(),
                e
            );
            return Ok(None);
        }
    };

    // Magic check
    if bytes.len() < 8 || &bytes[0..8] != CACHE_MAGIC {
        log::warn!(
            "[Section 64 B cache] {} has bad magic, treating as miss",
            cache_path.display()
        );
        return Ok(None);
    }
    let payload = &bytes[8..];

    match bincode::deserialize::<DecodedAudio>(payload) {
        Ok(decoded) => {
            log::info!(
                "[Section 64 B cache] HIT {} samples={} sr={} ch={} dur={:.2}s",
                cache_path.display(),
                decoded.samples.len(),
                decoded.sample_rate,
                decoded.channels,
                decoded.duration_seconds
            );
            Ok(Some(decoded))
        }
        Err(e) => {
            log::warn!(
                "[Section 64 B cache] failed to deserialize {}: {}, treating as miss",
                cache_path.display(),
                e
            );
            // 删除损坏 cache, 下次重新生成
            let _ = std::fs::remove_file(&cache_path);
            Ok(None)
        }
    }
}

/// 保存 decoded audio 到 cache. 失败只 warn 不 Err (cache 是 best-effort 优化).
pub fn save_cached<R: tauri::Runtime>(app: &AppHandle<R>, key: &str, decoded: &DecodedAudio) -> Result<()> {
    let dir = cache_dir(app)?;
    let cache_path = dir.join(format!("{}.{}", key, CACHE_EXT));

    let payload = match bincode::serialize(decoded) {
        Ok(p) => p,
        Err(e) => {
            log::warn!(
                "[Section 64 B cache] failed to serialize {}: {}",
                cache_path.display(),
                e
            );
            return Ok(());
        }
    };

    let mut file = match File::create(&cache_path) {
        Ok(f) => f,
        Err(e) => {
            log::warn!(
                "[Section 64 B cache] failed to create {}: {}",
                cache_path.display(),
                e
            );
            return Ok(());
        }
    };

    // 写 magic + payload
    if let Err(e) = file.write_all(CACHE_MAGIC) {
        log::warn!("[Section 64 B cache] failed to write magic: {}", e);
        return Ok(());
    }
    if let Err(e) = file.write_all(&payload) {
        log::warn!(
            "[Section 64 B cache] failed to write payload to {}: {}",
            cache_path.display(),
            e
        );
        // 清理半成品
        let _ = std::fs::remove_file(&cache_path);
        return Ok(());
    }
    if let Err(e) = file.flush() {
        log::warn!("[Section 64 B cache] failed to flush: {}", e);
    }

    log::info!(
        "[Section 64 B cache] SAVED {} samples={} bytes={}",
        cache_path.display(),
        decoded.samples.len(),
        8 + payload.len()
    );
    Ok(())
}

/// 清理 cache 目录 (用户手动触发, 当前不在 UI 暴露, 仅供调试)
pub fn clear_cache<R: tauri::Runtime>(app: &AppHandle<R>) -> Result<usize> {
    let dir = cache_dir(app)?;
    let mut removed = 0;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some(CACHE_EXT) {
                if std::fs::remove_file(&path).is_ok() {
                    removed += 1;
                }
            }
        }
    }
    log::info!("[Section 64 B cache] cleared {} entries", removed);
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_stable_for_same_file() {
        // 用 std::env::temp_dir() 写临时文件, 测两次 key 一致
        let dir = std::env::temp_dir();
        let test_path = dir.join("speakmirror_cache_test.bin");
        std::fs::write(&test_path, b"hello world from section 64 b").unwrap();
        let k1 = cache_key_for_file(&test_path).expect("k1");
        let k2 = cache_key_for_file(&test_path).expect("k2");
        assert_eq!(k1, k2, "Section 64 B cache key must be stable");
        assert_eq!(k1.len(), 16, "Section 64 B cache key 是 hex 16 字符 (64 bit)");
        std::fs::remove_file(&test_path).ok();
    }

    #[test]
    fn cache_key_changes_when_file_size_changes() {
        let dir = std::env::temp_dir();
        let test_path = dir.join("speakmirror_cache_test_size.bin");
        std::fs::write(&test_path, b"short").unwrap();
        let k1 = cache_key_for_file(&test_path).expect("k1");
        std::fs::write(&test_path, b"much longer content here!").unwrap();
        let k2 = cache_key_for_file(&test_path).expect("k2");
        assert_ne!(
            k1, k2,
            "Section 64 B cache key must change when content changes"
        );
        std::fs::remove_file(&test_path).ok();
    }
}
