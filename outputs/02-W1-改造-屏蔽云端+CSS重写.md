---
tags: [W1, 改造, 里程碑, 离线会记]
created: 2026-07-08
base: Meetily v0.4.0 (commit 0281737d)
status: ✅ 应用成功,待本地 build 验证
---

# 离线会记 W1 改造 — 屏蔽云端 + CSP 重写

## TL;DR

W1 改造 4 项已全部应用,本地基线测试 100% 成功:
- ✅ **CSP 严格化**:`https://api.ollama.ai` + Claude/Groq/OpenRouter/OpenAI 全部白名单删除
- ✅ **PostHog 埋点**:`posthog-rs = 0.3.7` 删依赖 + analytics 25 个 command 改 noop
- ✅ **云端 LLM 全砍**:`LLMProvider` 枚举保留(避免改 lib.rs 200+ 处),但 `from_str` 与 `generate_summary` 入口加双重守卫
- ✅ **品牌/identifier 重塑**:`com.meetily.ai` → `cn.lixianhuiji.app`,`meetily` → `离线会记`

## 改动总览

| 改动 | 文件 | 性质 | 备份位置 |
|---|---|---|---|
| identifier / productName / CSP / 删 updater | `frontend/src-tauri/tauri.conf.json` | 整体替换 | `outputs/patches/w1/backup-*/` |
| 25 个 analytics 事件改 noop | `frontend/src-tauri/src/analytics/commands.rs` | 整体替换 | 同上 |
| AnalyticsClient 简化(保留类型签名) | `frontend/src-tauri/src/analytics/analytics.rs` | 整体替换 | 同上 |
| 砍 OpenAI/Claude/Groq/OpenRouter/CustomOpenAI | `frontend/src-tauri/src/summary/llm_client.rs` | 2 处 anchor 替换 | 同上 |
| 删 `posthog-rs = 0.3.7` | `frontend/src-tauri/Cargo.toml` | 1 行删除 | 同上 |

**改动代码行数**:
- tauri.conf.json: 全量替换(88 行)
- analytics.rs: 1374 行 → 75 行(精简 94%)
- commands.rs: 412 行 → 109 行(精简 74%)
- llm_client.rs: 在 11279 字节文件上做 2 处 anchor 替换
- Cargo.toml: -1 行

## 设计原则

### 1. 保留所有 lib.rs 的 invoke_handler 引用(25 个 analytics command)
- 改 commands.rs 函数实现,不改签名 → 编译零影响
- 这是最关键的兼容性保证,否则 lib.rs 会有 25 处 `undefined command` 错误

### 2. 保留 `LLMProvider` 枚举的 5 个云端变体
- 不改 enum,改 `from_str` + `generate_summary` 入口守卫
- 未来若恢复某个云端 provider(如企业版可配 B 端 LLM 代理),只删守卫即可

### 3. PostHog API key + host 已完全删除
- 原代码:`api_key: "phc_Aa9PqeCkDkVbtbRsYjtmHANBfcscjCVupxZwrtL5vZ77"`, `host: "https://us.i.posthog.com"`
- 替换后:无任何 PostHog 字面量,`AnalyticsConfig::default()` = `enabled: false`

### 4. identifier 用 `cn.lixianhuiji.app`(国内 Apple 开发者命名规范)
- `cn.*` 明确国内分发
- `lixianhuiji` = 离线会记的拼音(可改,但要 Apple 开发者账号前缀一致)

## 验证记录

### 本地基线测试(已通过)
在 `/tmp/lxhj_apply_test` 跑 apply.sh:

```
==> 仓库根: /tmp/lxhj_apply_test
  [REPLACE] frontend/src-tauri/tauri.conf.json
  [REPLACE] frontend/src-tauri/src/analytics/analytics.rs
  [REPLACE] frontend/src-tauri/src/analytics/commands.rs
  PATCHED  /tmp/lxhj_apply_test/frontend/src-tauri/src/summary/llm_client.rs
  [EDIT] Cargo.toml: 删 posthog-rs = 0.3.7
```

| 验证项 | 结果 |
|---|---|
| tauri.conf.json identifier 改 `cn.lixianhuiji.app` | ✅ |
| tauri.conf.json productName 改 `离线会记` | ✅ |
| tauri.conf.json CSP connect-src 删 `https://api.ollama.ai` | ✅ |
| tauri.conf.json 删 updater plugin block | ✅ |
| analytics.rs 行数 1374 → 75 | ✅ |
| commands.rs 行数 412 → 109 | ✅ |
| llm_client.rs 含 "离线会记仅支持本地 LLM" 文案 | ✅ |
| Cargo.toml 不再含 posthog | ✅ |
| lib.rs 中 25 个 `analytics::commands::` 引用完整 | ✅ |

### 待你做的验证(在真实仓库)

```bash
# 1. 浏览器 fork Meetily
#    https://github.com/Zackriya-Solutions/meetily
#    点 Fork → 改成自己的 repo(比如 lxhj-fork/meetily)

# 2. clone 到本地
git clone https://github.com/lxhj-fork/meetily.git
cd meetily
git remote add upstream https://github.com/Zackriya-Solutions/meetily.git

# 3. 跑 W1 改造
git checkout -b feature/w1-no-cloud
bash /Users/wangwei/Documents/离线会记/outputs/patches/w1/apply.sh

# 4. 跑 build
cd frontend
pnpm install              # 拉依赖
./clean_run.sh            # Mac dev 模式

# 5. 验证断网也能跑
#    关闭 WiFi / 拔网线 → 启动 app → 录音 → 转录 → 摘要
#    如果断网后还能完整工作 = 改造成功

# 6. 验证网络流量
sudo tcpdump -i lo0 -A | grep -E "posthog|ollama.ai|api\.anthropic|api\.openai"
# 或: Proxyman / Charles 抓包,确认零外网请求
```

## 上游同步策略(避免冲突)

- **只拉不推**:`git fetch upstream` + `git rebase upstream/main`,不主动 PR
- **锁定 base SHA**:patch 的 anchor 字符串对应当前 `0281737d`,上游更新时先 `git diff` 看 lib.rs 的 25 个 analytics::commands 引用 + llm_client.rs 的 from_str 块是否变
- **冲突解决**:
  - analytics 模块上游改了 → 重新跑 apply.sh(我们整模块替换,不受上游影响)
  - llm_client.rs 上游改了 → 手动对比 anchor 周围代码
  - tauri.conf.json 上游改了 → 同上,重点看 CSP + identifier 字段

## 已知边界 / 未做

- ❌ **未做前端 UI 改中文**(W1 范围外,优先级 W3)
- ❌ **未做 lib.rs 的 AnalyticsProvider.tsx 引用改 noop**(前端 Analytics 组件仍会调 init_analytics,得到 Ok(()) 假成功;不破坏功能,只是埋点面板的 "Disable" 按钮点击后无视觉变化)
- ❌ **未做 OllamaEndpoint 默认值改 `http://localhost:11434`**(已经是这个,无需改)
- ❌ **未做 entitlements.plist + signing**(W3 阶段,需要先拿到 Apple 开发者账号)

## 接下来 W1 剩余动作(本周内)

- [ ] **你**浏览器里 fork + clone meetily → 跑 apply.sh → 跑 `pnpm install` + `./clean_run.sh`
- [ ] **你**断网测试 + 抓包验证零外网
- [ ] **你**申请 Apple 开发者账号($99,1-3 周)
- [ ] **你**申请商标 9 类 + 42 类(¥540)
- [ ] **你**注册域名 `lixianhuiji.cn` + `.com`(¥160)
- [ ] **我**出 W2 改造:sherpa-onnx + SenseVoiceSmall 集成 patch(预计 3 个文件:新增 `audio/transcription/funasr_provider.rs` + `audio/transcription/mod.rs` 注册 + `Cargo.toml` 加依赖)
