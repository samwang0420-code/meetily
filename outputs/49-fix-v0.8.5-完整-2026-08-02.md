# v0.8.5 完整修复 — commit 1b52011 (2026-08-02)

> 用户截图 V0.8.2 + 问"就这么简单吗" 后, 我深挖 6 个真问题并 4 件 commit 一次到位.
> 31f60e2 (polling 30min) + 1b52011 (4 fixes) 共同组成 v0.8.5.

## 1. 两个 commit 协同

### commit 31f60e2 (polling 30min)
- SidebarProvider.tsx:208 MAX_POLLS 300 → 1800 (10 分钟 → 30 分钟)
- i18n zh.ts:543 / en.ts:653 timeout_15min → timeout_30min 文案
- polling 超时前兜底查后端 status='completed'

### commit 1b52011 (本批 4 件)
1. **§34 enforcement** (P0 上线必要条件)
2. **folder_path 直写** (P1 用户体验)
3. **free 配额 block** (P1 商业化承诺)
4. **version.ts v0.8.5 双常量** (防 §37.1 教训)

## 2. §34 enforcement 详解

### 历史 bug
- meeting-02c7f2d9 / a2851054 三段文本几乎完全相同 (silero 8s 强切后 SpeechEnd.samples 又 emit 一次完整 utterance)
- meeting-5479ed7e 单段 287.9s (SpeechEnd 没触发)
- meeting-cc26ddc3 单段 104.3s

### 旧 binary 状态
- import.rs 末尾有 const MAX_REALTIME_SEGMENT_DURATION_SECONDS=12 (用户工作树待提交)
- 但 save 路径**没 check**, const bypass

### 新 binary 状态
```rust
// import.rs create_meeting_with_transcripts 内
for segment in segments {
    if let Some(d) = segment.duration {
        if d > MAX_REALTIME_SEGMENT_DURATION_SECONDS {  // 12s
            anyhow::bail!("§34 reject: segment too long ({:.1}s > 12s). ...")
        }
    }
}
for win in segments.windows(2) {
    // 相邻段文本前 30 字符相同 → 拒绝
    // 相邻段时间重叠 > 0.5s → 拒绝
}
```
- api_save_transcript 路径同步加 check (record 实时录音也覆盖)

## 3. folder_path 直写详解

### 历史 bug
- 51 个会议 folder_path 50 NULL, 重转录找不到音频
- 实际音频在 ~/Movies/meetily-recordings/ 73 个文件夹

### 旧链路 (有断点)
```
start_recording → placeholder INSERT folder_path=NULL
录音中 IndexedDB 存 folderPath
stop_recording → recording-stopped 事件 → sessionStorage
useRecordingStop → storageService.saveMeeting(folderPath)
后端 save_transcript → UPDATE folder_path (仅在 NULL 时)
```
早期 binary 任意环节断就丢失.

### 新链路 (一锤定音)
```rust
// recording_commands.rs start_recording 内, placeholder 时直接生成
let folder_path = match get_default_recordings_folder_path().await {
    Ok(base) => {
        let ts = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        let name = meeting_name.as_deref().unwrap_or("Untitled");
        Some(format!("{}/{}_{}", base, name.replace('/', "_"), ts))
    }
    Err(_) => None,  // 兜底 None 不破坏录音
};
sqlx::query("INSERT OR IGNORE INTO meetings (..., folder_path) VALUES (?, ?, ?, ?, ?)")
    .bind(folder_path.as_deref())
```
不依赖前端, 录音开始那一刻 folder_path 就写进 DB.

## 4. free 配额 block 详解

### 历史 bug
- sam@wang.com (uid=3, free) 7 月用 9 场, FREE_MONTHLY_MEETING_LIMIT=5
- §29 商业化承诺 "free 用户每月 ≤ 5 场" 违反
- 配额只在 segments 截断生效, 月度会议数没拦截

### 新增拦截
```rust
// api.rs api_save_transcript 入口
if membership == "free" {
    let used = query month_meetings_used;
    let status = compute_quota("free", used);
    if !status.can_record {
        return Err("[§29 quota] 免费用户本月已用 5 场, 达到上限...");
    }
}
```

### UI 影响
- free 用户超额时 api_save_transcript 返错
- 前端 polling 接到 error, 显示错误 toast + 升级 CTA (前端现有 toast 框架复用)

## 5. version.ts v0.8.5

### §37.1 教训加固
```ts
// §37.1: 双常量必须同步 (APP_VERSION + APP_VERSION_SHORT)
export const APP_VERSION = "v0.8.5";
export const APP_VERSION_SHORT = "v0.8.5";
```
注释里写明铁律, 防止下次只改一个.

## 6. §37 硬闸门全过

| 步骤 | 结果 |
|---|---|
| npx tsc --noEmit | 0 errors (1 个 §18 bun:test 不动) |
| npx next build | 14.9s ✓ |
| cargo build --release | 1m31s ✓ (25 个 §18 warning 不动) |
| check_historical_fixes.py | **34/34 PASS** (29 → 34, 加 5 个新锚点) |

## 7. binary

- `/Users/wangwei/Documents/meetily/target/release/meetily` 67.78 MB
- mtime 2026-08-02 13:18
- tag v0.8.5 (覆盖指向新 commit 1b52011)

## 8. §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open /Users/wangwei/Documents/meetily/target/release/meetily
```

GUI 上看:
1. **左下角版本**应显示 "V0.8.5" (不再 V0.8.2)
2. **开会 a09de61d** → 摘要直接显示 (polling 修复)
3. **录 30s 新会议** → 录完 DB 验证:
   ```bash
   sqlite3 "$HOME/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite" \
     "SELECT folder_path FROM meetings WHERE id=(SELECT meeting_id FROM transcripts ORDER BY id DESC LIMIT 1)"
   ```
   应有 folder_path (不再是 NULL)
4. **录完后打开新会议, 看 ASR 段是否正常 8s 切分** → 不应有大段 (>12s)
5. **sam@wang.com 登录** (如果还能切) → 录会议应被配额 block + 升级 CTA

如果 1/2/3/4/5 任一不满足, 立刻截图 + `git reset --hard 31f60e2 && cargo build --release`.

## 9. 关联

- [[49-v0.8.4-77min-录音+超时诊断-2026-08-01]]
- [[49-fix-v0.8.5-落地-2026-08-01]] (commit 31f60e2)
- [[49-v0.8.5-全面体检-2026-08-01]] (6 类问题诊断)
- [[49-fix-v0.8.5-完整-2026-08-02]] (本文件)
- §15 §25 §29 §34 §37 §49 §50
