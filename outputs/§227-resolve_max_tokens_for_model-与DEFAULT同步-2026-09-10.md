# §227 — resolve_max_tokens_for_model 与 DEFAULT_MAX_TOKENS 同步 (2026-09-10)

## 触发 (背景)

用户跑 §226 commit 后实际跑 c1299582 (77 min / 10112 chars 庭审录像) 摘要,日志显示:
```
🔄 Starting generation (max_tokens: 1200)     ← 用户期望 1000, 实际 1200
```
修了 §226 DEFAULT_MAX_TOKENS=1000,但日志仍是 1200。

## 根因

`frontend/src-tauri/src/summary/service.rs::resolve_max_tokens_for_model` 函数 (service.rs:2-21) per-branch 硬编码:
```rust
} else if model_name.contains(":3b") || model_name.contains(":3B") {
    Some(1200)   // ← qwen2.5:3b 走这个分支, 永远 1200
} else if model_name.contains(":4b") || ... {
    Some(1500)
} else {
    Some(1200)
}
```

`DEFAULT_MAX_TOKENS` 是 `models.rs:430` 的常量 (默认 §226 = 1000),但 `resolve_max_tokens_for_model` 是**独立** hardcoded 分支表:
- `:1b/:1.5b` → 800
- `:2b` → 800
- `:3b` → 1200  ← qwen2.5:3b 默认
- `:4b/gemma3` → 1500
- 其他 → 1200

`service.rs:737` 调用 `resolve_max_tokens_for_model(&model_name, custom_openai_max_tokens)` 拿 hardcoded 值,**完全不走 DEFAULT_MAX_TOKENS**。所以 §226 改 DEFAULT=1000 对这个函数没影响。

**用户 invoke log 第 48 条** (14:28:24):
- bundled binary 13:58 = §226 built,但 resolve_max_tokens 仍是 hardcoded 1200
- llama-helper 接收到 max_tokens=1200 (= 1200 字面常量从 binary 中读出)
- 用户跑的 max_tokens 一直都是 1200,这是预期 (修了代码没修复这个函数)

## 修复 (§227)

**single source of truth**: 所有 per-branch 都改成 `Some(DEFAULT_MAX_TOKENS as u32)`,改 §226 DEFAULT 是 1 处改全部生效。

```rust
fn resolve_max_tokens_for_model(model_name: &str, user_override: Option<u32>) -> Option<u32> {
    if let Some(t) = user_override {
        if t > 0 {
            return Some(t);  // 用户显式永远优先
        }
    }
    // §227: 全部 fallthrough 到 DEFAULT_MAX_TOKENS (single source of truth)
    let default = crate::summary::summary_engine::models::DEFAULT_MAX_TOKENS as u32;
    if model_name.contains("1.5b") || ... || model_name.ends_with(":1b") {
        Some(default)
    } else if model_name.contains(":2b") || ... {
        Some(default)
    } else if model_name.contains(":3b") || ... {
        Some(default) // §191 was 1200, §226 → 1000 via DEFAULT
    } else if model_name.contains(":4b") || ... {
        Some(default)
    } else {
        Some(default)
    }
}
```

## 单测更新

§191 测试 (4 个) 全部 `qwen2.5:3b None 期望 1200` 改成 `期望 DEFAULT_MAX_TOKENS (= 1000 from §226)`。

新增 `section_227_no_hardcoded_max_tokens_remain` 检测 — 用 brace-matching 提取 `resolve_max_tokens_for_model` 函数体,断言不含 `Some(800)/Some(1200)/Some(1500)` 字面常量。

## §37 6 步硬闸门

- ✅ cargo check --lib: 0 errors (17 §18 warnings 不动)
- ✅ cargo test --lib --release: **566 passed** (1 fail §18 fixture `transcript_709b.txt` 缺 + 3 ignored)
- ✅ check_historical_fixes.py: **796/796 PASS** (+4 §227 anchor)
- ✅ cargo build --release: 5m 06s, binary 59M mtime 14:54:42
- ✅ sync_app_bundle.sh: 3 binary 全 sync (sha 915668c... meetily, eeb3556... llama-helper, 87955227... ffmpeg)
- ✅ bundle binary 含 `mov w*, #0x3e8` (1000 literal) — §227 真生效
- ⏳ GUI 端到端 (§15 强制, 用户必做)

## 用户必做 (§15 GUI 验收)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```
进 c1299582 → 点重新生成摘要。期望:
1. llama-helper log 显示 `🔄 Starting generation (max_tokens: 1000)` (不再 1200)
2. 跑约 3 min/chunk × 3-4 chunks = 10-12 min 完成
3. processing_time 应 < 720s (12 min), 不超时
4. chunk_count ≥ 3, status=completed

## 铁律 (未来 v0.X 演进适用)

1. **per-branch 数值表必须跟 single source of truth 同步** — 不要 hardcoded 多个 800/1200/1500, 改 1 处不动另外 5 处
2. **新增 :5b / :7b / :13b 模型时** — 直接 `Some(DEFAULT_MAX_TOKENS as u32)`, 不要新加 hardcoded
3. **guard 锚点必须测函数体不出现 magic number**, 不只测常量正确 — 因为 hardcoded literal 也算 magic
4. **§191 §225 §226 系列 commit 必须跑 cargo test --lib 全套** — 单测改动 + 函数改动同步 (否则漏 sync)
5. **macOS bundle sync 后必须断当前 llama-helper** — 重 build sync 让 OS 重启, 旧 PID 31382 (16:48) 还在拿老 binary 跑

## 关联

- §226 (DEFAULT 4096→1000, 但漏改 resolve_max_tokens 独立硬编码)
- §225 (链路传透 + DEFAULT 4096→1200)
- §191 (per-model max_tokens resolution, 2026-08-28 立)
- §52 (qwen3.5:2b 800 token cap 历史)
- §37 (硬闸门) / §92 (决策迁移铁律) / §18 (不主动改无关 bug)
