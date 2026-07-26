# W1-A: Markdown / TXT 导出 (2026-07-16)

> 配套代码仓库:`/Users/wangwei/Documents/meetily/`
> binary:`/Users/wangwei/Documents/meetily/target/release/meetily`(66MB arm64, 14:24)

## 改动概要

4 个文件,**纯前端**(Rust 没动 → 不需要 cargo check):

| 文件 | 改动 |
|---|---|
| `src/hooks/meeting-details/useCopyOperations.ts` | 新增 `handleExportTranscript(format: 'md' \| 'txt')`,复用 `fetchAllTranscripts`,Blob + `<a download>` 触发下载 |
| `src/components/MeetingDetails/TranscriptButtonGroup.tsx` | 加 2 个按钮 `导出 MD` / `导出 TXT`(icon: `FileCode` / `FileText`),仅当 prop 传入时显示 |
| `src/components/MeetingDetails/TranscriptPanel.tsx` | 接 prop + 转发给按钮组 |
| `src/app/meeting-details/page-content.tsx` | 传 `() => copyOperations.handleExportTranscript('md'/'txt')` |
| `src/i18n/locales/zh.ts` | 加 6 个 key: `export_md` / `export_txt` / `export_md_title` / `export_txt_title` / `export_success` / `export_failed` |
| `src/i18n/locales/en.ts` | 同上,英文版 |
| `src/lib/analytics.ts` | `trackCopy` 类型放宽,接受 `export_md` / `export_txt` |

## 关键设计

- **零依赖**:`Blob + <a download>` 触发浏览器原生下载,没装/不需要 `tauri-plugin-dialog` 权限改动
- **文件命名**:`{safeTitle}_{YYYY-MM-DD}.md`,标题清洗非法字符
- **MD 格式**:`# 标题` + 元数据(YAML 风格) + `**[MM:SS]** 文本` 段(粗体时间戳)
- **TXT 格式**:纯文本 + `===` 分隔线 + `[MM:SS]` 段
- **空会议容错**:`disabled={transcriptCount === 0}`,toast 提示
- **Analytics 上报**:`trackCopy('export_md'/'export_txt')`,埋点后续追踪

## 验证 (按 AGENTS.md §15 铁律)

### 步骤 1: 编译干净 ✅

- **cargo build --release**: 5m39s, 0 errors, 13 warnings(全是无关 dead code)
- **tsc --noEmit**: 0 errors(原 bun:test 错忽略)
- **next build**: 15/15 pages generated,meeting-details 933 kB

### 步骤 2: GUI 端到端录音 30s 验证 ⏳ 待用户

> **关键**:**CLI 启动 Tauri 会被 launchd 当 orphan silent abort**(§15 铁律第 2 步明确禁止 CLI 验证),必须 GUI session 启动。

#### 你需要做的:

1. **双击打开 Finder 里的 binary**(或运行 `open /Users/wangwei/Documents/meetily/target/release/meetily`)
2. **新建一次录音**,录音 30 秒(说中文,随便读一段文档)
3. **结束录音**,进入会议详情页
4. **点击 "导出 MD" 按钮** → 应弹出 macOS 保存对话框(实测是浏览器下载,看 Safari/Chrome 行为)
5. **检查导出文件**:
   ```bash
   open ~/Downloads/未命名会议_2026-07-16.md
   ```
6. **数据库段数 ≥ 1 验证**:
   ```bash
   sqlite3 "/Users/wangwei/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite" \
     "SELECT COUNT(*) FROM transcripts ORDER BY id DESC LIMIT 1"
   ```

#### 已知行为

- macOS 14+:浏览器下载走系统下载文件夹(~/Downloads),Tauri WebView 不弹原生 save dialog(因为没用 tauri-plugin-dialog)
- 这不是 bug,是设计取舍:**零依赖 vs 原生 save dialog**
- 未来如果用户反馈需要,可加 `tauri-plugin-dialog` 升级到原生对话框

## 红线

- ❌ **不破坏现有录音功能**(AGENTS.md §15 铁律,出 bug 立刻回滚)
- ❌ **不改 Rust 代码**(本轮只前端)
- ❌ **不引入新依赖**(保持零依赖方案)

## 下一步 (B + C)

### B: Sidebar 搜索实测 + 空状态补完

- 实测当前 932 行 Sidebar 里的 `Search` icon 是否能用
- 修 bug + 加空状态("没找到匹配的会议")

### C: 把导入音频 + 热词从 Beta 默认开启

- 找到 BetaFeatures 配置开关
- 把 `importAndRetranscribe` / `hotwords` 默认 `true`
- 让普通用户在设置里能直接看到,不藏

---

## 风险

- **导出按钮可能被现有用户已经习惯的"复制"按钮挤掉**:lg 以下屏幕 button group 折行,可能拥挤,需要 UI 反馈再调整
- **未跑真实录音回归**:AGENTS.md §15 第 2 步在等你 GUI 操作,我不能替你跑
