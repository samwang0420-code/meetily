---
title: §92 代码漏根因审计 — AGENTS.md / outputs 描述与代码实际状态脱节
date: 2026-08-07
author: 助理 (WorkBuddy)
type: audit-report
trigger: 用户指控"每次大变动后代码都会漏"
session: 当前会话
---

# §92 — 代码漏根因审计 (2026-08-07)

## 触发 (用户原话)

> "我现在发现每次大的变动之后代码都会漏，功能会恢复到以前的几个版本之前的功能上，这不是我想要的"

## 当前真实状态 (一句话)

**HEAD caad987，§91 文档 + 主代码 commit ba7ee13 真实存在且功能落地。但 §62 §63 §90 等历史"已完成"优化在 git 历史里完全不存在 commit。guard 脚本 64/64 PASS 是假象 — 它覆盖不到这些盲点。**

---

## §91 描述 vs 代码实际对照表 (全部已逐项验证)

| §91 描述 | 实际验证 | 状态 |
|---|---|---|
| Bug 1 i18n fileURLToPath | scripts/verify_i18n.mjs 含 fileURLToPath | ✅ 真 |
| Bug 2 MCP SQLITE_OPEN_READ_WRITE | meetily-mcp/src/main.rs:132 | ✅ 真 |
| Bug 3 obsidian duration | obsidian_export/mod.rs:393 注释 + 393-403 逻辑正确 | ✅ 真 |
| Bug 4 recording_devices emit meeting_id | recording_commands.rs:230-293 meeting_id 完整 | ✅ 真 |
| Bug 5 action_items markdown 解析 | action_items/mod.rs:30, 41 parse_markdown_action_items | ✅ 真 |
| P0-A topic_graph | lib.rs:65, 523, 592-595 注册 + scheduler.rs | ✅ 真 |
| P0-B obsidian_export | lib.rs:63, 598-601 注册 | ✅ 真 |
| P1-A MCP Server | meetily-mcp + READ_WRITE 修复 | ✅ 真 |
| P1-B Speaker UI | transcript LEFT JOIN + view 渲染 | ✅ 真 |
| P2-A action_items | parser + 4 测试 | ✅ 真 |
| P2-B scheduler | scheduler.rs + lib.rs setup | ✅ 真 |
| P2-C LiveQA | live_qa/mod.rs + lib.rs:66 + Overlay | ✅ 真 |
| 热词 6 pack | hotwords_data/ + sherpa_hotwords.py PACK_FILES | ✅ 真 |

**§91 14 项核心功能，全部真实。** 这部分是好的。

---

## 系统性盲点：guard 64/64 PASS 但代码实际回退

### guard 脚本的本质 (致命缺陷)

`scripts/check_historical_fixes.py` 只做一件事：
1. 文件存在
2. 文件内容匹配一个正则

**它不验证**：
- 注释里"§X 已修"是否对应真实生效的功能
- grep 匹配到一段代码是否真的在做那件事
- 修复是否"完整"还是"半截"

**64 个 anchor 覆盖的 §**：§23 §24 §27 §29 §32 §33 §34 §36 §38 §31P0 §56 §78 §79 §81 §83 §P1-B §P2-B §P2-C §90 §91

### guard 漏掉的关键 § (系统性盲点)

| § | 内容 | guard 是否有 anchor | 当前代码状态 |
|---|---|---|---|
| **§62 A** | 多 daemon (3 个 Python 子进程并行) | ❌ 无 | ❌ sherpa_daemon.rs:113 仍 `Mutex<Option<SherpaHandle>>` 单 daemon |
| **§62 B.1** | import.rs hardlink | ❌ 无 | ❌ import.rs `hard_link` 0 命中 |
| **§62 B.2** | decode_cache.rs (SHA1 缓存) | ❌ 无 | ❌ 文件不存在 |
| **§62 B.3** | decoder.rs /tmp wav | ❌ 无 | ❌ decoder.rs:296 仍 `tempfile_in(parent_dir)` |
| **§62 C** | max_tokens 1200 → 800 | ❌ 无 | ❌ processor.rs:12 仍 `pub const DEFAULT_SUMMARY_MAX_TOKENS: u32 = 1200;` |
| **§63** | provider sensevoice-zh → funasr-nano-zh | ❌ 无 | ❌ retranscription.rs:474 `if effective_provider == "sherpa_funasr_nano" { Some("sensevoice-zh".to_string()) }` |
| **§90** | friendlyImportTitle | ❌ 无 | ✅ **真在代码** `frontend/src/components/ImportAudio/ImportAudioDialog.tsx:190-201` (我之前 grep 路径错了) |
| **§86** | memory_watcher 阈值 + 自动降级 | ⚠️ 部分 | ✅ 接入了 (recording_commands.rs:297, 504, 525)，但阈值是否合理未验证 |

**§62 §63 §90 三组关键优化，guard 完全覆盖不到。**

### guard PASS ≠ 功能完整 (反例)

§62 三联优化：
- outputs/62-v0.8.5-Section-64-三联优化.md 文档完整
- commit message 写了 §64 A/B/C
- §37 闸门显示 cargo test 80/80 PASS + binary 65MB
- **但 commit `835a3c3`, `1f42580`, `d99b0b2` 在 git 历史里完全不存在** (git log --all 0 命中)
- 当前代码里 §62 三联优化完全回退

**§70 事故报告已经记录过一次** (2026-08-06)：当时 11 个 anchor 2/13 pass，被发现后补 commit 9346f69。
但 §62 三联优化的 commit 没在 §70 补救清单里 — **它从来没真正存在过**。

---

## "代码漏" 根因 (5 个)

### 根因 1: AGENTS.md § 章节 ≠ 代码 commit
`outputs/62` 文档说"改了 N 个文件，M 个测试 PASS"，但 git log --all 里找不到对应 commit。
可能性：commit 后被 rebase 丢了 / cherry-pick 失败 / 从未真提交。

### 根因 2: outputs 文档写在 commit 之前
AGENTS.md / outputs 决策文档描述了"功能设计"，但实现 commit 失败或半截。
docs commit (caad987, 1c38ecd, b48cea6) 经常先于 feat/fix commit (ba7ee13, fda59cd)。

### 根因 3: guard 脚本只检查文字锚点
grep "section 64" "max_tokens" 等关键词都能 PASS，但实际代码功能可能完全回退。
guard 应该检查**功能**（如：daemon count > 1，max_tokens 值 == 800），而不是文字注释。

### 根因 4: 用户验收只看 binary 不看代码
§15 GUI 验收 SOP 只让用户跑应用、看活动监视器、查 DB 字段，不验证 Rust 源码。
binary 是某个 commit 编译出来的，commit 丢了用户根本不知道。

### 根因 5: 分支策略混乱
- v0.8.5 tag 在 `fix/polling-30min` 分支 (f440ccd)
- 当前 HEAD 在 `perf/summary-map-concurrency` (caad987)
- §91 主代码 ba7ee13 只在 perf 分支
- cherry-pick 经常失败 (决策日志 §51 提过 "移植 cherry-pick 不行, 两个仓库 processor.rs 已分叉, 手写核心")

---

## 已完成的 (§91 内)

✅ §91 14 项核心功能全部真在代码里 (P0-A/B/P1-A/B/C/D/P2-A/B/C + 热词 6 pack + 5 bug)

---

## 已完成但被吹成 100% 但实际回退的 (§91 之外)

❌ **§62 三联优化**：多 daemon / hardlink / max_tokens 800 — **完全没在代码里**
❌ **§63 provider 映射**：sensevoice-zh 仍是默认 — **没改**
✅ **§90 friendlyImportTitle**：真在 `ImportAudioDialog.tsx:190-201`（截图 "导入音频 2026-08-05 13:45" 即 §90 生效）
❌ **import.rs 没走 sherpa_daemon**：§63 即使改对，import 流程仍走 whisper engine — **没接**
❌ **tauri.conf.json:5 identifier 仍是 cn.lixianhuiji.app**：根因没修 — **没改**
⚠️ **§86 memory_watcher 阈值**：接入了但 1.2GB 阈值合理性与 daemon 多进程 RSS 统计待验证
❌ **UI 版本号**：sidebar/HomeDashboard 显示 v0.8.5，§91 文档说 v0.8.6 但代码没改

---

## 需要做什么 (P0-P2 排序)

### P0: 立即修（用户截图的 bug）
1. **§62 C max_tokens** — processor.rs:12 `1200` → `800` (1 行)
2. **§63 provider** — retranscription.rs:474 `sensevoice-zh` → `funasr-nano-zh` (1 行)
3. **§90 friendlyImportTitle** — import.rs 加 title 格式化函数 (10-20 行)

### P1: 1 周内（性能 + 验收）
4. **§62 A 多 daemon** — sherpa_daemon.rs 改 Vec<Mutex<Option<SherpaHandle>>> (核心改动，2-3h)
5. **§62 B.1 hardlink** — import.rs hard_link 优先 (10 行)
6. **§62 B.3 /tmp wav** — decoder.rs `parent_dir` → `std::env::temp_dir()` (1 行)
7. **import.rs 走 sherpa_daemon** — §63 即使改了，import 也得接 sherpa (2h)
8. **tauri.conf.json identifier 改 tech.yanjingai.app** — 但要保留旧路径迁移 (1h)

### P2: 1 个月内（系统性防漏）
9. **guard 脚本升级**：加 §62 §63 §90 anchor，加 §62 A 多 daemon 检查（grep `Vec<Mutex<Option<SherpaHandle>>>`），加 §63 provider 检查，加 max_tokens 值检查
10. **AGENTS.md 同步**：CLAUDE.md 分支名 / 当前版本号 / §62 §63 §90 状态
11. **决策日志归档**：补 §56-§91 进决策日志-当前.md
12. **commit 完整性 CI**：每次 commit 后跑 guard --strict，FAIL 阻断 push
13. **AGENTS.md vs 代码 diff 检查**：outputs 文档写"改了 X 文件"，CI 跑 git show 验证

---

## 关键警示

> §70 事故报告 (2026-08-06) 当时修的是另一批 §，§62 三联优化没被 §70 抓到。
> 现在 §91 §56 §28 铁律执行不严，又漏一批 (§62 §63 §90)。
> 用户问的"每次大变动后代码都会漏"是真实的 — 这是**流程问题，不是单次疏忽**。

---

## 关联

- [[71-7款AI会议工具深度调研]] §5 P0-P2
- [[62-v0.8.5-Section-64-三联优化]] §62 文档存在但 commit 不存在
- [[70-v0.8.5-§修复未落地事故]] §70 事故记录
- [[91-v0.8.6-§71-P0-P2完整收尾+热词词库]] §91 文档 + ba7ee13 commit
- AGENTS.md §28 §35 §56 (铁律但执行不严)
- scripts/check_historical_fixes.py (盲点见上)

---

## §92.1 双写规则 (2026-08-07 立)

**触发**: 用户原话: "我做这个项目过程也同步写 obsidian，你也保持这个规则，每次修改和决策都需要双写。目录在 Obsidian Vault"

### 双写路径映射
| 类型 | 仓库路径 | Obsidian 路径 |
|---|---|---|
| **决策文档** | `outputs/§X-...md` | `项目/3-离线会记/§X-...md` |
| **项目规则** | `AGENTS.md` | `项目/3-离线会记/AGENTS.md` |
| **项目 README** | `README.md` | `项目/3-离线会记/README.md` |

### 不双写
- 决策日志 `决策日志-当前.md` (仓库本地用，污染严重)
- 代码文件 / Cargo.toml / src-tauri/*

### 双写命令模板
```bash
SRC="/Users/wangwei/Documents/离线会记/outputs/§X-*.md"
DST="$HOME/Documents/Obsidian Vault/项目/3-离线会记/§X-*.md"
cp "$SRC" "$DST" && diff -q "$SRC" "$DST"
# 期望: 无输出 = 一致
```

### 当前双写状态 (2026-08-07 11:17)
- ✅ outputs/91 � Obsidian 91 (7949 bytes)
- ✅ outputs/92 ↔ Obsidian 92 (8373 bytes)
- ✅ AGENTS.md ↔ Obsidian AGENTS.md
- ✅ README.md ↔ Obsidian README.md (修正路径错误)
