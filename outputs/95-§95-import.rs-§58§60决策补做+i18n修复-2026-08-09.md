# §95 — import.rs §58/§60 决策补做 + i18n 完整化

**触发**: 2026-08-09 13:07 哥 2 张截图:
1. 导入失败对话框 "Import Failed / An error occurred during import / Whisper engine not initialized / Close / Try Again" — **全部英文**, 应翻译
2. 工作台内存从 547MB 掉到 66MB, 提示 binary 重启过

## 根因 (双 bug)

### Bug 1: Whisper engine not initialized (决策回退)
**Obsidian 决策日志 §58/§60 写得很清楚**:
- §58: "导入路径新增 sherpa 后端分支, 按 provider 派发到 sherpa daemon / parakeet / whisper"
- §60: **"audio/import.rs provider=None 时显式走 pick_default_sherpa_model (与 worker.rs / 录音一致), 永不 fallback Whisper"**

但当前 `import.rs:339`:
```rust
let use_parakeet = provider.as_deref() == Some("parakeet");
// 之后 else 分支默认走 whisper ❌
```

**git log import.rs** 没有任何 §58/§60 决策的 commit 落地 — 典型的"AGENTS.md § 章节 ≠ 代码 commit"。

### Bug 2: ImportAudioDialog i18n 遗漏
- §82 P1-D commit `040f20e` 之前修过 3 处: `'Loading models...'` / `'Select model'` / `'Continue'`
- **但 dialog 标题/描述/按钮/toast 完全没动** — 11 处 hardcoded 英文残留

## §95 修复

### 1. 后端 `frontend/src-tauri/src/audio/import.rs`

按 `worker.rs:120-135` 模式 + `retranscription.rs:524-535` 的 base64 模式:

```rust
// §95 fix: §58/§60 决策补做 — effective_provider 按 worker.rs:120-135 模式, 永不 fallback Whisper
let effective_provider: String = match provider.as_deref() {
    Some("parakeet") => "parakeet".to_string(),
    Some("sherpa_funasr_nano") => "sherpa_funasr_nano".to_string(),
    Some("sherpa_paraformer") => "sherpa_paraformer".to_string(),
    Some(other) => {
        warn!("§95: unknown import provider '{}', fallback to pick_default_sherpa_model", other);
        crate::config::pick_default_sherpa_model()
    }
    None => crate::config::pick_default_sherpa_model(),
};
let use_parakeet = effective_provider == "parakeet";
let use_sherpa = effective_provider == "sherpa_funasr_nano"
    || effective_provider == "sherpa_paraformer";
```

**三分支转写**:
```rust
} else if use_sherpa {
    // §95 fix: 按 retranscription.rs:524-535 模式 f32 LE bytes → base64, block_in_place 调 sync daemon
    let pcm_bytes: Vec<u8> = segment.samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let pcm_b64 = base64::engine::general_purpose::STANDARD.encode(&pcm_bytes);
    let model_for_call = effective_model_name.clone();
    let hw_pack = crate::audio::hotwords_globals::current_pack();
    let hw_custom = crate::audio::hotwords_globals::current_custom_with_product_terms();
    let resp: crate::audio::sherpa_daemon::SherpaResponse = tokio::task::block_in_place(|| {
        let daemon = crate::audio::sherpa_daemon::global();
        daemon.transcribe_blocking(
            &model_for_call, &pcm_b64, 16000, false, hw_pack, &hw_custom, None, None,
        )
    })?;
    (resp.text, 0.9f32)
} else {
    // §60 决策: 不允许 fallback whisper
    return Err(anyhow!("§60 决策: import 不允许走到 fallback whisper (effective_provider={})", effective_provider));
}
```

**§60 永不 fallback 检查**:
```rust
if !use_parakeet && !use_sherpa && total_segments > 0 {
    return Err(anyhow!(
        "§60 决策: import 不支持 provider={:?}. 仅支持 parakeet / sherpa_funasr_nano / sherpa_paraformer, 不 fallback whisper.",
        effective_provider
    ));
}
```

### 2. 前端 `frontend/src/components/ImportAudio/ImportAudioDialog.tsx`

11 处 hardcoded 英文 → `t()`:

| 位置 | Before | After |
|---|---|---|
| `:111` | `'Import failed'` (toast) | `t('import_dialog.toast_failed')` |
| `:221` | `'Import cancelled'` (toast) | `t('import_dialog.toast_cancelled')` |
| `:258` | `Importing Audio...` | `t('import_dialog.title_processing')` |
| `:263` | `Import Failed` | `t('import_dialog.title_failed')` |
| `:268` | `Import Complete` | `t('import_dialog.title_complete')` |
| `:273` | `导入音频文件` (混搭) | `t('import_dialog.title_default')` |
| `:281` | `'An error occurred during import'` | `t('import_dialog.description_failed')` |
| `:282` | `'Import an audio file...'` | `t('import_dialog.description_default')` |
| `:279` | `'Processing audio...'` (fallback) | `t('import_dialog.description_processing')` |
| `:462, 477` | `Cancel` (2 处) | `t('common.cancel')` |
| `:483` | `Close` | `t('common.close')` |
| `:486` | `Try Again` | `t('import_dialog.try_again')` |

### 3. i18n keys 加全

`zh.ts` + `en.ts` 加 `import_dialog` 区段 (9 keys):
```
title_processing / title_failed / title_complete / title_default
description_processing / description_failed / description_default
toast_failed / toast_cancelled / try_again
```

i18n 776 → 785 keys (+9).

### 4. 单测 `audio::import::tests::test_provider_dispatch_default_to_sherpa`

测 §60 决策逻辑: None / Some("") / Some("unknown") / Some("whisper") / Some("localWhisper") 全部都映射到 sherpa_funasr_nano, **永不返回 whisper**.

### 5. audit + guard 升级

`scripts/audit_codebase.py` 加 `check_import_whisper_fallback()`:
- 检测 import.rs 仍调用 `transcribe_audio_with_confidence` (Whisper fallback) → error
- 检测 import.rs 缺 `use_sherpa` 分支 → error

`scripts/check_historical_fixes.py` 加 3 §95 anchor:
- `95_import_provider_dispatch` (effective_provider match)
- `95_import_use_sherpa_branch` (use_sherpa = ...)
- `95_import_no_whisper_fallback` (§60 注释)

## 验证

| 项 | 结果 |
|---|---|
| `cargo check --lib` | 0 errors (27 §18 warnings, +1 §95) |
| `cargo test --lib` | **327 passed**; 0 failed; 2 ignored; 3 filtered (system_audio SCK) |
| `cargo test test_provider_dispatch_default_to_sherpa` | 1 passed |
| `pnpm run build` | ✅ |
| `cargo build --release` | 1m 28s, binary 72.6MB, mtime 14:32:41 |
| `sync_app_bundle.sh` | hash `9115d3868e59` 一致 |
| `audit_codebase.py` | **0 errors / 0 warns / 60 info** |
| `check_historical_fixes.py` | **87/87 PASS** (84 + 3 §95) |
| `node scripts/verify_i18n.mjs` | i18n OK: 785 keys aligned |
| `strings binary` | 含 `fallback whisper (effective_provider=` (§95 改动编译进) |
| .app bundle 启动 | PID 49272 |

## 哥重启后能看到的

### 1. 导入对话框（中文环境）
- 标题: **导入失败** / 导入中... / 导入完成 / 导入音频文件
- 描述: **导入过程中发生错误** / 正在处理音频... / 导入音频文件以创建新会议并生成转录
- 按钮: **关闭 / 重试 / 取消**
- Toast: **导入失败** / 导入已取消

### 2. 实际行为
- provider=None (默认) → 走 `pick_default_sherpa_model` (= funasr-nano-zh 947MB)
- provider="sherpa_funasr_nano" 或 "sherpa_paraformer" → 走对应 sherpa daemon
- provider="parakeet" → 走 parakeet (老用户)
- **不再 fallback Whisper** (§60 决策)

## 关联

- `outputs/94-全面代码审计-代码漏系统性问题-2026-08-07.md` (§94 主报告)
- `outputs/95-§94.1-TranscriptSettings硬编码修复-2026-08-07.md` (§94.1 上一版本)
- `outputs/94-§62-v0.8.5-Section-64-三联优化.md` (§62 决策补做历史)
- Obsidian 决策日志 §58/§60 (BaseLine 2026-08-04 决策原文)
- commit `xxx` (待落地)
- AGENTS.md §3.2 §94 铁律
