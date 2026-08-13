# W1 P0 D3 — 热词设置持久化 + 后端登录后自动加载 globals (2026-07-17)

## 修了什么 (用户截图暴露的真问题)

### 1. `/settings/hotwords` 选择/勾选不持久化
**根因**: 原代码 `setCfg()` 只是更新 state, 必须点底部"保存"按钮才调 `invoke('hotwords_save')`。用户点击内置词库按钮 → 视觉上高亮 → 但 state 变更从不落库 → 刷新页面回到默认。

**修复** (`frontend/src/app/settings/hotwords/page.tsx` 全部重写, 192 行):
- 选词 / 勾选 / 输入 → 自动 600ms debounce 后调 `hotwords_save` (DB) + `hotwords_set_globals` (in-memory)
- React Hooks 顺序违规: 旧版 `React.useEffect` 写在 `if (!user) return null` 之后 → 删掉, 改成在组件顶层用 `loading` 状态控制渲染
- 选内置词库时自动 `enabled=true` (除非选 "none")
- "✓ 热词已保存" 绿色提示 (1.5s 自动消失)
- 全程 loading 用 `useAuth().loading` + 本地 `loadingCfg` 双层保护, 不闪烁
- 等 auth context 恢复完再决定渲染, 避免 hooks 调用次数不一致

### 2. 后端不依赖 settings 页: 登录/注册成功后自动加载该用户的热词到 globals
**根因**: 用户从未打开 settings 页 → globals 永远是默认 ("none", "") → 第一条录音无 L1 纠错.

**修复** (`frontend/src-tauri/src/user/commands.rs`):
```rust
async fn load_user_hotwords_into_globals<R: Runtime>(
    app: &AppHandle<R>, user_id: i64,
) -> Result<(), String> {
    let pool = db_pool(app)?;
    let (builtin, custom, _enabled) = HotwordsRepository::get(&pool, user_id).await?;
    crate::audio::hotwords_globals::set(builtin, custom);
    info!("[hotwords] loaded into globals for user_id={}", user_id);
    Ok(())
}
```
- `user_register` 成功后: `let _ = load_user_hotwords_into_globals(&app, id).await;`
- `user_login` 成功后: 同样调用

## 验证铁律 (AGENTS.md)
- ✅ `cd frontend/src-tauri && cargo check` → 0 errors
- ✅ `cd frontend && ./node_modules/.bin/tsc --noEmit` → 0 errors (排除无关 bun test)
- ✅ `cargo test sherpa_daemon::tests` → 4/4 pass
- ✅ `cargo test summary::` → 97/97 pass
- ✅ `python3 -c "import ast; ast.parse(...)"` 对所有 daemon 脚本通过
- ⚠️ **Tauri macOS app 在 CLI shell 启会被 launchd silent abort**, 必须 GUI 重启实测

## 用户 GUI 验证 checklist (重启 app 后照做)

### 步骤 1: 验证自动持久化
1. 重启 app (Dock 退出再点图标)
2. 进入 `/settings/hotwords`
3. 点 "技术 · 研发 · 工程"
4. 等 ~1 秒 → UI 右下应该闪过绿色 "✓ 热词已保存"
5. Cmd+R 刷新页面 → 应该看到 "技术" 仍处于高亮状态 (蓝色边框 + 蓝底浅色)
6. 点下方"启用热词"开关, 切到开启
7. 等 ~1 秒
8. Cmd+R 刷新 → 两个状态都应该保持

### 步骤 2: 验证后端启动加载 (全局)
1. 让上面保存的是 "技术" 域
2. 完全退出 app (Cmd+Q)
3. 重新启动 app
4. **不打开 /settings/hotwords** 直接到首页
5. 录 30 秒: "前段和后端的数据库设计, black note 编辑器"
6. 结束时查 DB:
   ```bash
   sqlite3 "/Users/wangwei/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite" \
     "SELECT COUNT(*) FROM transcripts ORDER BY id DESC LIMIT 1"
   ```
7. **预期**: 转写文本中 "前段" → "前端", "black note" → "BlockNote"

### 步骤 3: stderr 日志验证
- app stderr (终端里 `./target/release/meetily` 启动的话能看到; 否则用 Console.app 过滤 `cn.lixianhuiji.app`)
- 应该看到 `[sherpa_asr] hotword_bias: words=XX l0_hits=X` 字样 (XX ≈ 50, X ≥ 1)

### 步骤 4: 跨境域
1. /settings/hotwords 选 "跨境 · 独立站 · 海外仓"
2. 录 30 秒: "广告投放后转化率提升, 独立站海外仓运营"
3. 预期: 4 个专名应保持正确 (ASR 大概率本来就能识别对, 关键验证 "✓ 热词已保存" + globals 已更新)

## 边界 / 已知问题

- **D 之前录制过的会议无法回溯**: 旧 transcript 是修改前的 raw text (DB 里)
  - 解法: 如果需要, 用 "重新转录" 按钮 (SettingsModal 那个回放图标) 触发 retranscription.rs 重新跑, 此时会带新 hotwords
- **如果用户登录后从来没保存过 hotwords, globals 是 ("none", "")**: L1 不触发, 但 L0 (产品名硬规则) 永远触发
- **React strict mode 双跑**: dev mode 下 useEffect 跑两遍, 但 persist fn 用 timer, 第二次 setCfg 是同 value 不触发新 timer; 已有 loadedRef 保护避免 mount 时空跑

## 后续 (A 阶段完成, 进入 D4-D5)
- D4: FunASR-Nano 设为默认 ASR 引擎; 速度优化 (intra_op_num_threads / 全 INT8 / CoreML provider)
- D4 同步: cam++ 下载容错 (断点续传) + 8G 设备自动降级
- D5: 法律/研发/跨境/金融各 30 分钟真实录音验收; 通过后 A 阶段封板

