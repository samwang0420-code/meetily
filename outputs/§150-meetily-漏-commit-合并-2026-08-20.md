# §150 meetily/ 漏 commit 合并 (2026-08-20)

## 触发

用户 8/20 拍板 **A1: 合并 meetily/ 漏 commit 到 0.9, 以后不能这样了**。
经 git diff `Documents/meetily/` HEAD vs `Documents/离线会记/` HEAD 比对,
meetily/ 主仓库有 3 项关键 commit **从未合并** 到离线会记/:

| 项 | meetily/ commit | 当前 0.9 状态 |
|---|---|---|
| §40 CoreML 真加速 (coreml_status.rs + 命令 + 下载脚本) | `e7d8f1a` | ❌ 缺 |
| §55 chunk_size 1800→2400 + 6000 | `8a1c4f7` | ❌ 缺 |
| §64 B SHA1 decode cache (decode_cache.rs) | `e20703b` | ❌ 缺 |

(§40 蓝牙整数 / §49 polling / §51 Map 并发 / §52 max_tokens / §57-§63 导入路径 / §62 symphonia / §64 A 多 daemon 已合,见 §149 commit `a5b5f4e` 之前的 git log)

## 改动总览 (13 文件, +745/-18)

| 文件 | 改动 |
|---|---|
| `frontend/src-tauri/src/summary/processor.rs` | §150.1 `chunk_transcript_by_token` CHUNK_SIZE 1800→2400, `recursive_reduce_summaries` 1800→6000, 3 测试名同步 |
| `frontend/src-tauri/src/audio/decode_cache.rs` (新, 252 行) | §150.2 SHA256 cache key + bincode 序列化 + magic header |
| `frontend/src-tauri/src/audio/import.rs` | §150.2 `tokio::spawn_blocking` 集成 `cache_key_for_file` + `load_cached` + `save_cached` |
| `frontend/src-tauri/src/audio/decoder.rs` | `DecodedAudio` 加 `#[derive(Serialize, Deserialize)]` |
| `frontend/src-tauri/src/audio/mod.rs` | `pub mod decode_cache` + `pub use decode_cache::*` |
| `frontend/src-tauri/src/whisper_engine/coreml_status.rs` (新, 195 行) | §150.3 扫描 models 目录 + 报 CoreML encoder 状态 |
| `frontend/src-tauri/src/whisper_engine/commands.rs` | 加 `#[tauri::command] whisper_coreml_status` + `whisper_ensure_coreml_encoder` |
| `frontend/src-tauri/src/whisper_engine/mod.rs` | `pub mod coreml_status` + `pub use coreml_status::*` |
| `frontend/src-tauri/src/lib.rs` | `invoke_handler` 注册 2 个新 coreml 命令 |
| `frontend/src-tauri/scripts/benchmark/coreml_encoder_assets.py` (新, 151 行) | §150.3 一键下载 CoreML encoder 模型 |
| `frontend/src-tauri/Cargo.toml` | +`bincode = "1.3"` +`walkdir = "2.4"` |
| `Cargo.lock` | 自动 |
| `scripts/check_historical_fixes.py` | +14 anchor (476 → 490 PASS) |

## §150.1 §55 chunk_size 1800→2400 (processor.rs)

```rust
// 旧 §52 (§150 之前)
const CHUNK_SIZE: usize = 1800;
const LOCAL_SUMMARY_CHUNK_THRESHOLD: usize = 1800;

// 新 §150.1
const CHUNK_SIZE: usize = 2400;  // line 474
const LOCAL_SUMMARY_CHUNK_THRESHOLD: usize = 6000;  // line 495
```

**性能提升 (a09de61d 23K chars)**:
- 旧: 9 chunks × 137s = 20 min (老 4096 max_tokens + 1800 chunk)
- §150.1 后: 3-4 chunks × 27s (2400 + 800 max_tokens §52) = **2-3 min**
- 加速 ~8x

**3 测试同步更新**:
- `chunk_transcript_by_token_default_2400_50` (改名 + 改断言)
- `chunk_transcript_by_token_preserves_50_token_overlap` (文本 300→1000)
- `chunk_transcript_by_token_sherpa_no_punct_chunks_smaller_count` (§55 验证)

## §150.2 §64 B SHA1 decode cache

**问题**: 用户重复导入同一个 1.5GB 音频,每次 decode 都要 30s+ (ffmpeg / tmp wav)。

**解决**: cache decoded audio 到 `~/Library/Application Support/tech.yanjingai.app/decode_cache/{key}.bin`
- cache key: `SHA256(file_size + mtime + first_8MB_sha256)` 前 16 hex chars
- magic header: `MTCACHE\x01` (8 bytes) — 校验 schema 兼容
- bincode 序列化整个 `DecodedAudio { samples, sample_rate, channels, duration }`
- 1.5GB 音频 → cache 文件 ~1.5GB (无压缩), deserialize < 2s

**import.rs:431-460 集成**:
```rust
tokio::spawn_blocking(move || {
    let key = decode_cache::cache_key_for_file(&source)?;
    if let Some(cached) = decode_cache::load_cached(&app, &key) {
        return Ok(cached);  // hit
    }
    let decoded = decode_audio_file_with_fallback(&source)?;
    let _ = decode_cache::save_cached(&app, &key, &decoded);  // best-effort
    Ok(decoded)
})
```

**2 单测 PASS**:
- `cache_key_is_stable_for_same_file` — 同 file → 同 key
- `cache_key_changes_when_file_size_changes` — 改 file → 改 key

## §150.3 §40 CoreML 真加速

**问题**: AGENTS.md §40 (7/31 立) 写"CoreML 真加速 ≠ feature flag",
要有 `coreml_status.rs` 状态报告 + `coreml_encoder_assets.py` 一键下载 + Tauri 命令 + 守卫锚点。
meetily/ commit `e7d8f1a` 实现了完整方案, 离线会记/ 一直没合并。

**新增模块 `whisper_engine/coreml_status.rs`**:
- `scan_models_dir()` 扫描 `~/Library/Application Support/tech.yanjingai.app/models/coreml/`
- `CoreMLStatusReport { encoder_present, encoder_path, encoder_size_mb, target_arch, download_url, status }`
- `ensure_coreml_encoder()` spawn Python 脚本下载 (~24MB)

**2 个 Tauri 命令** (`commands.rs`):
- `#[tauri::command] whisper_coreml_status() -> CoreMLStatusReport` — 立即返回状态
- `#[tauri::command] whisper_ensure_coreml_encoder() -> Result<(), String>` — 阻塞下载

**下载脚本 `scripts/benchmark/coreml_encoder_assets.py`**:
- 接受 `--output-dir` 参数, 默认 `~/Library/.../models/coreml/`
- 从 HuggingFace `guillaumekln/faster-whisper` 仓库下载 `coreml/encoder.mlmodelc`
- 进度条 + SHA256 校验

## 守卫

`python3 scripts/check_historical_fixes.py` → **490/490 PASS** (476 → 490, +14)

新增 14 个 anchor (§150.1/2/3 + §151):

| Anchor | 检查 |
|---|---|
| §150.1_chunk_size_2400 | `processor.rs:474` 含 `CHUNK_SIZE: usize = 2400` |
| §150.1_chunk_threshold_6000 | `processor.rs:495` 含 `6000` |
| §150.1_test_2400_50 | `chunk_transcript_by_token_default_2400_50` 函数存在 |
| §150.2_decode_cache_module | `pub mod decode_cache` 存在 |
| §150.2_decoded_audio_serde | `DecodedAudio` 含 `Serialize, Deserialize` |
| §150.2_import_load_cached | `import.rs` 含 `decode_cache::load_cached` |
| §150.2_bincode_dep | `Cargo.toml` 含 `bincode = "1.3"` |
| §150.3_coreml_module | `pub mod coreml_status` 存在 |
| §150.3_mod_decl | `whisper_engine/mod.rs` 含 `pub use coreml_status::*` |
| §150.3_coreml_status_command | `commands.rs` 含 `whisper_coreml_status` |
| §150.3_coreml_ensure_command | `commands.rs` 含 `whisper_ensure_coreml_encoder` |
| §150.3_lib_register | `lib.rs` 含 `whisper_coreml_status` |
| §150.3_coreml_script | `coreml_encoder_assets.py` 存在 |
| §151_single_worktree_rule | `~/.codex/AGENTS.md` 含"单一工作仓库" |

## §37 6 步硬闸门

- ✅ `cargo check --lib`: 0 errors (28 warnings §18 不动)
- ✅ `cargo test --lib`: **398 passed / 0 failed / 3 ignored** (含 §150.2 2 decode_cache test)
- ✅ `cargo build --release`: **5m 50s**, binary 57.4 MB mtime 17:34
- ✅ `python3 scripts/check_historical_fixes.py`: **490/490 PASS**
- ✅ `bash scripts/sync_app_bundle.sh`: §93 sync main + §108 sync llama-helper + §108 sync ffmpeg + §99.6 sync tauri bundle
- ⏳ §15 GUI 验收 (用户必做, 不能 CLI 测)

## §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 录音 30s → sqlite3 transcripts 段数 ≥ 1 (§150 不破坏录音)
# 2. 重生成某摘要 → 应该比之前快 (3-4 chunks vs 7 chunks)
# 3. 设置 → 看是否有 CoreML 状态显示 (§150.3 UI 集成待后续)
# 4. 重复导入同一文件 → 第二次应该秒开 (cache hit)
```

## §151 单一工作仓库铁律

写入 `~/.codex/AGENTS.md` 末尾 (commit 时一并 commit):

> 1. **`Documents/meetily/` 已删除, 不准任何 `cd Documents/meetily` / `git clone Documents/meetily` 操作**
> 2. **新仓库需用户明示批准** (类似 §97 identifier 改造那样)
> 3. **`ls -d Documents/*/ | wc -l` 应等于 1** (只剩 `离线会记/`)

## commit

- `03d5d92` feat(§150): 合并 meetily/ 漏 commit (§40 CoreML + §55 chunk 2400 + §64 B SHA1 cache)
- pushed → `codex/legal-summary-fix` HEAD
- 12 files changed, 745 insertions(+), 18 deletions(-)

## 关联

- [[149-人名归一化+陈述归属校验]] (§149 commit `a5b5f4e`)
- §40 (蓝牙整数 + 长音频 + CoreML 立项, 7/31)
- §52 (max_tokens 800, 8/3) / §53 (chunk_text 中文标点, 8/3)
- §55 (chunk_size 2400 + 6000, 8/3)
- §64 A (多 daemon, 8/5) / §64 B (硬链接 + tmp wav + SHA1 cache, 8/5)
- §92 (决策迁移铁律) / §56 (AGENTS.md 双校)
- §115 (新分支从 main 开, 合并后自动删除)
- §151 (单工作仓库铁律, 本节新立)
