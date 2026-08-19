# §110 Ollama banner i18n key 字面泄漏修复 (2026-08-19)

**commit**: `b8af7d0` (branch main, push OK)
**触发**: 用户截图 `/knowledge` 页 banner 显示 i18n key 字面文本 (`knowledge.ollama_offline_title` 等), 应该显示中文文案。
**binary**: target/release/meetily 73M mtime 2026-08-19

## 根因 (1 跳)

§132.1 commit `cf97a1f` 文档说加 `knowledge.ollama_offline_*` 8 个 key, 调用 `t('knowledge.ollama_offline_*')` 也没错。但**实际代码把 keys 加到了 `summary` 命名空间** (zh.ts:530-537, en.ts:527-534), 调用时 `t()` lookup `summary.*` 没有这些 key, fallback 返 path 字符串本身 (`t()` 函数 frontend/src/i18n/index.tsx:46)。

跟 §107 完全同一个模式 — 声明路径 ≠ 调用路径。`t()` 的 fallback 是返 path,不是空白,所以用户看到的是 `knowledge.ollama_offline_title` 字面。

## 修复 (2 文件, +26/-18)

1. **`frontend/src/i18n/locales/zh.ts`**:
   - 从 `summary` namespace 删 8 行 + 1 行注释
   - 新增顶级 `knowledge: { ... }` 块 (689 行),含 8 个 keys + 注释

2. **`frontend/src/i18n/locales/en.ts`**:
   - 同步英文版

## §37 硬闸门

- ✅ tsc --noEmit: 0 errors (除 §18 bun:test 已知)
- ✅ next build: 10s OK
- ✅ cargo build --release: 2m37s, binary 73M
- ✅ check_historical_fixes.py: **337/337 PASS**
- ✅ sync_app_bundle.sh: 3 binary 全 sync (main + llama-helper + ffmpeg)
- ✅ verify_i18n.mjs: 通过 (剩余 zh-only keys 是 §104 已遗留)

## §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 进 /knowledge 页 (会议脉络)
# 2. 如果 Ollama 没跑, banner 应显示完整中文文案:
#    "想跨会议追踪主题, 需要本地 AI 模型" + 选项 A/B 卡片 + 关闭按钮
# 3. **不应**看到 `knowledge.ollama_offline_title` 等 key 字面
```

## 教训 (§107 + §110 强化)

- §132.1 commit 时我看了 page.tsx + i18n, 但只验证 cargo build + tsc pass,**没真正进 GUI 渲染一次 banner**。
- §37 闸门没补"tsc/next build 不验证 i18n lookup 实际匹配"这层。
- 任何 `t()` / `localT()` 改动 → §15 GUI 验收必跑(用户真渲染一次才能发现 key 字面泄漏)。
- 写完 i18n key 后, 必须 grep 一遍确认**调用路径 == 声明路径**(可在 verify_i18n.mjs 加 symmetric check)。

## 关联

- §107 (录音通知 i18n 路径错位, 同一个模式)
- §132.1 (本次修的 banner 首次实现)
- §56 (AGENTS.md §X 描述 ≠ 代码 commit)
- §37 (硬闸门) / §92 (决策迁移铁律) / §18 (不主动改无关 bug)

