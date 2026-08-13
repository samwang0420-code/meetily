# W1-ABC 合稿:商业化工具补完 v0.6.14(2026-07-16)

> 配套代码:`/Users/wangwei/Documents/meetily/`
> binary:`/Users/wangwei/Documents/meetily/target/release/meetily`(66MB arm64, **14:35**)
> 配套记录:[[46-W1-A-Markdown-TXT导出]]

## 一句话总结

**W1 原计划 4 项功能,实际只需做 1.5 项**。**导入音频、热词早就默认开启**(我之前判断过期),Sidebar 搜索接好了只缺空状态。真正的新功能只有 Markdown/TXT 导出 + 搜索空状态。

## 改动明细

### A: Markdown / TXT 导出(✅ 真新做)

7 文件,**纯前端**:

| 文件 | 改动 |
|---|---|
| `src/hooks/meeting-details/useCopyOperations.ts` | 新增 `handleExportTranscript(format)` — Blob + `<a download>`,复用 `fetchAllTranscripts` |
| `src/components/MeetingDetails/TranscriptButtonGroup.tsx` | + 2 按钮 `导出 MD` / `导出 TXT`(icon: FileCode / FileText) |
| `src/components/MeetingDetails/TranscriptPanel.tsx` | 接 prop + 转发 |
| `src/app/meeting-details/page-content.tsx` | 传 handler |
| `src/i18n/locales/zh.ts` | + 6 key: `export_md` / `export_txt` / `export_*_title` / `export_success` / `export_failed` |
| `src/i18n/locales/en.ts` | 同上 |
| `src/lib/analytics.ts` | `trackCopy` 类型放宽 |

### B: Sidebar 搜索空状态(✅ 真新做)

2 文件:

| 文件 | 改动 |
|---|---|
| `src/components/Sidebar/index.tsx` | `meetings` 文件夹 children=[] 且有 searchQuery 时显示"没有匹配的会议"+ 清空按钮 |
| `src/i18n/locales/zh.ts` / `en.ts` | + `common.clear` |

### C: 导入音频 + 热词从 Beta 默认开启(✅ 核实已完成,**零改动**)

| 项 | 实际状态 |
|---|---|
| `importAndRetranscribe` 默认值 | **`true`**(`src/types/betaFeatures.ts:24`) |
| Sidebar "导入音频" 按钮 | **已默认渲染**(蓝色背景显眼) |
| Settings > 热词 tab | **已默认渲染**(在 SettingTabs.tsx 第 39 行) |
| 热词独立页面 | `/settings/hotwords`(134 行,可正常访问) |

**结论**:W1 阶段 C 项是空操作,**不需要任何代码改动**。之前的"藏在 Beta 后"判断是基于 v0.4.x 老信息,v0.6.x 已全部转正。

## 验证 (按 AGENTS.md §15 铁律)

### 步骤 1: 编译干净 ✅

| 命令 | 结果 |
|---|---|
| `./node_modules/.bin/tsc --noEmit` | **0 errors**(无关的 bun:test 警告) |
| `npx next build` | **15/15 pages**, meeting-details 933 kB, settings/hotwords 7.96 kB |
| `cargo build --release` (Rust 无改动,但跑增量) | **0 errors, 1m 36s, 13 warnings(无关 dead code)** |
| Binary | 66 MB, Mach-O arm64, **时间戳 14:35** |

### 步骤 2: GUI 端到端验证 ⏳ 待用户

按 [[46-W1-A-Markdown-TXT导出]] 第 2 步:

```bash
# 1. GUI 启动(必须双击 Finder 或 'open',CLI 会 silent abort)
open /Users/wangwei/Documents/meetily/target/release/meetily

# 2. 录音 30s → 进会议详情 → 看顶部按钮组:
#    [复制] [导出 MD] [导出 TXT] [录音] [优化*]
#    * "优化" 按钮(betaFeatures 控制) 默认显示

# 3. 点 "导出 MD" → 应触发浏览器下载 ~/Downloads/{title}_2026-07-16.md

# 4. 点 "导出 TXT" → 同上 .txt

# 5. Sidebar 顶部搜索框输入不存在的关键词 → 应显示 "没有匹配的会议 [清空]"

# 6. DB 段数 ≥ 1 验证(确认录音功能没被破坏)
sqlite3 "/Users/wangwei/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite" \
  "SELECT COUNT(*) FROM transcripts ORDER BY id DESC LIMIT 1"
```

## 关键设计权衡

### 1. 为什么用 Blob + `<a download>` 而不是 tauri-plugin-dialog?

| 方案 | 优点 | 缺点 |
|---|---|---|
| **Blob + `<a download>`**(本轮选) | 零依赖、零权限改动、立即能用、跨平台一致 | 走浏览器下载 UI,不是 macOS 原生 save dialog |
| tauri-plugin-dialog | 原生 save dialog、可选路径 | 要装插件 + 改 capabilities + 改 Rust(本次编译时间 +30%) |

**取舍**:WebView 触发下载在 macOS 上体验可接受(默认进 ~/Downloads),未来如果用户反馈再加原生 dialog。

### 2. 为什么导出按钮 icon 用 FileCode / FileText?

- FileCode(.md) 视觉上暗示"代码/结构化"
- FileText(.txt) 暗示"纯文本"
- 都是 lucide-react 内置,零依赖

### 3. 搜索空状态为什么只针对 meetings 文件夹?

Sidebar 只有 meetings 文件夹是动态内容(笔记/设置都是静态),其他文件夹 children 永远是稳定的,不需要"空匹配"。

## 改动文件清单

```
M  frontend/src/hooks/meeting-details/useCopyOperations.ts
M  frontend/src/components/MeetingDetails/TranscriptButtonGroup.tsx
M  frontend/src/components/MeetingDetails/TranscriptPanel.tsx
M  frontend/src/app/meeting-details/page-content.tsx
M  frontend/src/components/Sidebar/index.tsx
M  frontend/src/i18n/locales/zh.ts
M  frontend/src/i18n/locales/en.ts
M  frontend/src/lib/analytics.ts

8 files changed, +97/-3 lines (estimate)
```

## 边界 / 已知

- **导出按钮组在小屏幕(< lg)只显示 icon,不显示文字**(`hidden lg:inline`),与现有按钮一致
- **导出文件名长度上限 60 字符**(清洗非法字符 + slice),防止 OS 文件系统限制
- **URL.revokeObjectURL 延迟 1 秒释放**,避免某些浏览器下载未启动就回收
- **trackCopy 类型已放宽**,新加的 export_md / export_txt 算独立埋点事件

## W2 计划(下一轮)

按 [[42-商业化方案-会议纪要外包+工具订阅]] §6 路线图:

1. **D8-10**:接通本地 Ollama LLM(CSP 已允许 localhost:11434)
   - 已存在 `OllamaDownloadContext.tsx` / `BuiltinAi`
   - 接入"转录 → 摘要 → 待办"流水线
2. **D11-12**:一键下载 Ollama + 启动本地 LLM
3. **D13-14**:binary build + GUI 验证

## W3 计划

启动工具订阅版:
- LemonSqueezy 收款集成(0 代码,iframe 嵌入)
- 落地页(用 [[43-工具订阅落地页文案]])
- 工具订阅引导"7 天免费试用 → Pro ¥39/月"
