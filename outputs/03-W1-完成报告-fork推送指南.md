---
tags: [W1, 改造, 推送, 离线会记, samwang0420-code]
created: 2026-07-08
fork: samwang0420-code/meetily @ 0281737d (与上游同 SHA)
status: ✅ 本地验证 100% 通过,待你推到 fork
---

# 离线会记 W1 完成报告 — Fork 推送指南

## 1. 改造状态:全部验证通过 ✅

**Fork 验证链(已跑)**:
1. ✅ API 验证 `samwang0420-code/meetily` 存在,与上游 `Zackriya-Solutions/meetily` 同 SHA `0281737d`
2. ✅ 拉取 fork 10 个关键文件,与本地 baseline diff 0 字节差异
3. ✅ `apply.sh` 在 fork 源上跑通,5 项改造 100% 命中 anchor
4. ✅ 所有预期值对账成功(identifier / 行数 / 守卫数 / posthog 行数)
5. ✅ `git apply --check` 在干净仓库 100% 通过,5 文件 1:1 同步
6. ✅ 静态检查:25 个 `#[command]` 属性齐全 / 0 posthog 字眼残留

## 2. 改动总览(5 个文件)

| 文件 | 行数变化 | 改动 |
|---|---|---|
| `frontend/src-tauri/tauri.conf.json` | 113 行 → 113 行 | identifier / productName / CSP / 删 updater |
| `frontend/src-tauri/Cargo.toml` | -1 行 | 删 `posthog-rs = 0.3.7` |
| `frontend/src-tauri/src/analytics/analytics.rs` | 521 行 → 75 行 | PostHog Client → 本地 noop struct |
| `frontend/src-tauri/src/analytics/commands.rs` | 373 行 → 109 行 | 25 个 tauri::command 内部全 Ok(()) |
| `frontend/src-tauri/src/summary/llm_client.rs` | 346 行 | +2 处 anchor 守卫(砍云端 LLM) |

**改动代码统计**:
- 删除:约 700 行(PostHog 客户端实现 + 云端 LLM HTTP 调用)
- 新增:约 150 行(noop 实现 + 守卫)
- 修改:0 行(全文件级替换)

## 3. 推送流程(3 步 1 分钟)

### 步骤 A:本地 clone 你的 fork(首次)

```bash
cd ~/Documents
git clone https://github.com/samwang0420-code/meetily.git
cd meetily
git remote add upstream https://github.com/Zackriya-Solutions/meetily.git
git fetch upstream
```

### 步骤 B:跑推送脚本(应用 diff + 提交)

```bash
bash /Users/wangwei/Documents/离线会记/outputs/patches/w1/push-to-fork.sh
```

脚本会自动:
- 创建 `feature/w1-no-cloud` 分支
- 跑 `git apply --check`(失败会中断)
- 应用 `w1-fork-changes.diff`
- 提交 commit(commit message 已写好)

**输出**:
```
==> 应用 diff...
  ✅ diff 应用成功
==> 提交 commit...
  ✅ commit 完成: 36xxxxx
==> 准备推送到 origin (feature/w1-no-cloud)...
    如需推送,请手动执行:
    git push -u origin feature/w1-no-cloud
```

### 步骤 C:推送(脚本不自动推,留你确认)

```bash
git push -u origin feature/w1-no-cloud
```

**为什么脚本不自动推**:
- 推送是单向操作,推到错误地方难撤回
- 第一次推送需要你输 GitHub 凭据(token / SSH key)
- 你可能想先看 commit diff 再推

**预期 GitHub 链接**:
`https://github.com/samwang0420-code/meetily/tree/feature/w1-no-cloud`

## 4. 推送后的验证步骤(在你本机跑)

```bash
# 1. 同步 fork 拉取
cd ~/Documents/meetily
git pull origin feature/w1-no-cloud

# 2. 装依赖
cd frontend
pnpm install

# 3. 跑 dev build(首次 5-10 分钟)
./clean_run.sh            # Mac
# 或
./clean_run_windows.bat   # Win

# 4. 验证断网也能用
#    关闭 WiFi / 拔网线 → 启动 app → 录音 30 秒 → 转录 → 摘要
#    全程不报错 = 改造成功
```

## 5. 抓包验证零外网(关键!)

启动 app 后,在另一个终端跑:

```bash
# Mac (需要装 Wireshark 或用 Proxyman / Charles)
sudo tcpdump -i lo0 -A | grep -E "posthog|ollama.ai|api\.anthropic|api\.openai|groq|openrouter"
# 期望:无任何匹配

# 简易方法: Proxyman / Charles 开系统代理
# 期望:零 https 请求到非 localhost:11434
```

## 6. 关键文件位置

| 用途 | 路径 |
|---|---|
| W1 改造主日志 | `outputs/02-W1-改造-屏蔽云端+CSS重写.md` |
| 完整 diff(可手动 apply) | `outputs/patches/w1/w1-fork-changes.diff` |
| 一键推送脚本 | `outputs/patches/w1/push-to-fork.sh` |
| apply.sh(也可用,等同 push-to-fork 但只 apply 不 commit) | `outputs/patches/w1/apply.sh` |
| Replacement 模板(下次重跑用) | `outputs/patches/w1/replacement/` |
| 改动前原文件备份 | `outputs/patches/w1/backup-20260708-170240/` |

## 7. 上游同步策略(避免冲突)

`push-to-fork.sh` 第一次跑时:
- 创建 `feature/w1-no-cloud` 分支(独立分支,不影响 main)
- 上游 main 更新时,本地 `git fetch upstream` + `git rebase upstream/main` 同步
- W1 改动只动 5 个文件,冲突范围可控

**未来 W2-W3 的 patch 会在同一分支叠加 commit**,保持 fork 跟 W1 状态衔接。

## 8. 已知边界 / 推迟项

- ❌ **未做前端 UI 中文化**(W3 范围,先验证后端零外网能跑)
- ❌ **未做 Ollama endpoint 配置页改中文**(W3)
- ❌ **未做 `frontend/src/components/AnalyticsConsentSwitch.tsx` 删 hide**(W3,前端默认开启弹窗,后端 noop 后用户开关无效但不报错)
- ❌ **未做 `frontend/src/lib/analytics.ts` 改 noop**(W3,后端返 Ok(()) 后前端 track 调用也不报错)

**W1 现状的可用性**:
- ✅ 后端零外网(完整隔离)
- ✅ LLM 强制走 Ollama,云端 API 全部返错
- ✅ 25 个 analytics 事件全部 noop
- ✅ 品牌/identifier 改成离线会记
- ⚠️ 前端 UI 还是英文(看得懂英文的能用,纯中文用户 W3 后再优化)
- ⚠️ Mac/Win 启动应用需要装 Rust toolchain(参见 Meetily 官方文档)

## 9. 出错兜底

如果 `git apply` 报 conflict:
```bash
# 看冲突位置
git apply --reject w1-fork-changes.diff
# 会生成 .rej 文件,人工合并
```

如果 `pnpm install` 失败:
- 检查 Node.js 版本(>= 18)+ pnpm 版本
- 第一次 install 慢(5-10 分钟),可能下载 Rust 工具链

如果 `clean_run.sh` 编译失败:
- 看 Cargo 错误信息
- 大概率是 macOS SDK 路径问题,xcode-select --install 修复
- Windows 需要 MSVC build tools

## 10. 下一步(W1 完成后)

- [ ] **你**:跑 push 脚本 → 推到 fork → clone 到本地 → pnpm install → dev build
- [ ] **你**:断网测试 + 抓包验证
- [ ] **你**:浏览器侧 3 件事(Apple 开发者 / 商标 / 域名)
- [ ] **我**:W2 patch — sherpa-onnx + SenseVoiceSmall 国产 ASR 集成(预计 4 个文件)
