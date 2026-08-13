# §74 P0-B Obsidian vault 写入 Phase 1 (2026-08-06)

## 触发
71 报告 P0-B: **接 Obsidian vault 写入**（Charoite 真做了），用户已在 Obsidian，零教育成本，工作量 3 天，ROI 极高。

## Phase 1 交付（本 turn）
- **新表** `obsidian_export_settings(user_id, enabled, vault_path, subdir, template_id, last_exported_*)` + 索引（migration `20260806000000`）
- **新模块** `frontend/src-tauri/src/obsidian_export/{mod,markdown}.rs` (~580 行)
- **9 个单测** 全过：模板渲染 / slugify 中文 / 文件名 fallback / home 展开 / yaml escape / 重复 minutes 省略 / 默认 settings / Context fields
- **4 个 Tauri commands** 在 lib.rs 注册：
  - `api_obsidian_get_settings(user_id) -> Settings`
  - `api_obsidian_set_settings(settings) -> Result<()>`
  - `api_obsidian_export_meeting(user_id, meeting_id) -> Result<{path, bytes, duration_ms}>`
  - `api_obsidian_preview_markdown(meeting_id) -> Result<String>`
- **i18n** zh 16 + en 16 个 key 全部加在 `settings.obsidian.*`
- **新组件** `frontend/src/components/ObsidianSettings.tsx` + 挂到 `frontend/src/app/settings/page.tsx` general tab

## Phase 1 限制（已知，Phase 2 修）
1. `api_obsidian_export_meeting` 暂返 "Phase 2 尚未实现" — 接 summary/service.rs trigger 完整 meeting context 查询 (meetings + transcripts + summary_processes 三表 join)
2. user_id 暂硬编码 2 (machine owner fallback) — 接 auth.session.user_id
3. last_exported_meeting_id 暂未写路径列 — 后续 migration 加 `last_export_path`
4. related_links 暂空数组 — Phase 3 接 P0-A 知识图谱 (回查同 title 旧会议 [[wikilink]])

## Markdown 模板示例
```markdown
---
created: "2026-08-06T15:30:00+08:00"
meeting_id: "meeting-8bffd804-..."
title: "周会复盘: Q3 目标 + OKR 对齐"
duration_minutes: 77
transcript_count: 234
asr_provider: "sherpa_funasr_nano"
asr_model: "funasr-nano-zh"
tags:
  - meeting
  - 言镜AI
summary_preview: "## 关键决议\n..."
---

# 周会复盘: Q3 目标 + OKR 对齐

> 📅 2026-08-06T15:30:00+08:00 · ⏱ 77 分钟 · 🎤 234 段 · ASR `sherpa_funasr_nano / funasr-nano-zh`

## 📋 摘要
...

## 🎤 完整转录 (234 段)
<details>
<summary>点击展开转录</summary>

- [00:00:05] 王伟: 大家好...
- [00:00:10] 张三: 接着说...
</details>
```

## §37 闸门（本 turn 状态）
- ✅ cargo check 0 errors
- ✅ cargo test obsidian_export 9/9 PASS
- ✅ tsc 0 errors (1 §18 bun:test 不动)
- ⏳ next build (待跑)
- ⏳ cargo build --release (待跑)
- ⏳ §15 GUI 验收 (待用户)

## 文件清单
- `frontend/src-tauri/migrations/20260806000000_obsidian_export_settings.sql` (新, 21 行)
- `frontend/src-tauri/src/obsidian_export/markdown.rs` (新, 304 行 + 7 单测)
- `frontend/src-tauri/src/obsidian_export/mod.rs` (新, 281 行 + 2 单测)
- `frontend/src-tauri/src/lib.rs` (改, +2 行: pub mod + 4 commands)
- `frontend/src-tauri/src/audio/vad.rs` (改, rename 2nd `mod tests` → `mod timestamp_tests_v33` 修 pre-existing E0428)
- `frontend/src/i18n/locales/zh.ts` (改, +16 keys)
- `frontend/src/i18n/locales/en.ts` (改, +16 keys)
- `frontend/src/components/ObsidianSettings.tsx` (新, 196 行)
- `frontend/src/app/settings/page.tsx` (改, +1 import + 1 组件挂载)

## 关联
- 71 报告 P0-B
- §28 决策迁移铁律 (3 处同日落)
- §37 闸门 5 步
- §15 GUI 验收 (用户必做)
