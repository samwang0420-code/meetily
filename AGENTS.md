# AGENTS.md — 离线会记 / 言镜 AI 项目规则

> 任何 LLM session (Claude / Codex / WorkBuddy) 操作本项目必读。

## 1. 项目身份

- **产品名**: 言镜 AI / SpeakMirror (旧 Meetily)
- **主仓库**: `/Users/wangwei/Documents/离线会记/`
- **Obsidian Vault**: `~/Documents/Obsidian Vault/项目/3-离线会记/`
- **当前版本**: v0.8.6 (commit ba7ee13, HEAD caad987)
- **当前分支**: `perf/summary-map-concurrency`

## 2. 双写规则 (§92 立, 2026-08-07)

### ✅ 必须双写
- **`outputs/§X-...md` ↔ Obsidian `项目/3-离线会记/§X-...md`**
- 每次写 outputs/§X 后必须 `cp` 到 Obsidian 同名文件
- 文件大小 + 行数必须完全一致 (`diff -q` 应无输出)
- §91 / §70 / §62 / §71-78 等历史 outputs 都已双写

### ❌ 不要双写
- 决策日志 `决策日志-当前.md` (仓库本地用,污染严重,不同步到 Obsidian `00-收件箱/决策日志.md`)
- 仓库根目录 `CLAUDE.md` / `README.md` (Obsidian 不需要)
- 代码文件 / Cargo.toml / src-tauri/* (Obsidian 是笔记,不是代码镜像)

### 双写模板
```bash
# 写完 outputs/§X.md 后:
cp /Users/wangwei/Documents/离线会记/outputs/§X-*.md \
   "$HOME/Documents/Obsidian Vault/项目/3-离线会记/§X-*.md"
diff -q /Users/wangwei/Documents/离线会记/outputs/§X-*.md \
        "$HOME/Documents/Obsidian Vault/项目/3-离线会记/§X-*.md"
# 期望: 无输出 = 一致
```

## 3. §92 铁律 (防代码漏)

1. **AGENTS.md § 章节 ≠ 代码 commit** — outputs 写"改了 X 文件"前必须 `git show` 验证 commit 真存在
2. **guard 脚本盲点** — `check_historical_fixes.py` 只 grep 文字锚点,不验证功能
   - §62 §63 §90 三个盲点必须手动验证
   - 升级版 guard 见 §92 P2 清单
3. **release 前必跑** — `python3 scripts/check_historical_fixes.py --strict` + `cargo test --lib` + `cargo build --release` 三件套
4. **commit 完整性 CI** — commit message 写"fix §X"前必须 `git log --oneline | grep §X` 看主线真在

## 4. 文件结构

```
/Users/wangwei/Documents/离线会记/
├── outputs/                          ← 双写到 Obsidian
│   ├── §91-...md
│   ├── §92-...md
│   └── ...
├── frontend/
│   ├── src/                          ← Next.js UI
│   └── src-tauri/                    ← Rust + Tauri 后端
├── meetily-mcp/                      ← MCP server
├── llama-helper/                     ← LLM helper
├── CLAUDE.md                         ← Claude 上下文
├── AGENTS.md                         ← 本文件 (任何 LLM 必读)
├── README.md                         ← 项目 README
├── Cargo.toml                        ← workspace root
└── 决策日志-当前.md                  ← 仓库本地决策日志 (不同步)
```

## 5. 历史教训

- §70 (2026-08-06): 11 个 § 修复未落地 (commit message 写了但代码没动)
- §92 (2026-08-07): §62 §63 §90 三组关键优化 commit 完全丢失 (git log --all 0 命中)
- 用户原话: "每次大变动后代码都会漏,这不是我想要的"
- 根因: outputs 文档堆积,AGENTS.md § 章节 ≠ 代码 commit,guard 只查文字不查功能
