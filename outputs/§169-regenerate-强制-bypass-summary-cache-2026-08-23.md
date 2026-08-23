# §169 regenerate 强制 bypass summary cache (2026-08-23)

**触发**: 用户报告 "连续点了重新生成按钮生成了几条摘要, 没调用模型"。

## 根因

3 跳链路:
1. `useSummaryGeneration.ts::processSummary` invoke `api_process_transcript` 没传 `isRegeneration` 到后端
2. `service.rs::process_transcript_background` 不区分 generate vs regenerate, 都走 cache lookup
3. **中文 transcript + 中文 target**: `resolve_cached_english` 返 cached → Pass 1 LLM 跳过 → 直接复用上次英文 markdown 当中文返回

## 结果

- 用户点 "重新生成" 按钮
- DB 每次创建新 summary_processes 行 (process_id 唯一)
- **LLM Pass 1 不调** (cache 命中)
- 内容相同 → 用户感觉 "没调用模型"

## 修复

```rust
// commands.rs api_process_transcript
force_fresh: Option<bool>,

// service.rs process_transcript_background
let cached_english = if force_fresh {
    info!("§169 force_fresh=true, bypassing summary cache for meeting_id={}", meeting_id);
    None
} else {
    // 原 cache lookup 逻辑
};

// 前端 useSummaryGeneration.ts
invokeWithTimeout('api_process_transcript', {
    ...,
    forceFresh: isRegeneration,  // §169
}, ...);
```

## §37 6 步硬闸门

- ✅ tsc --noEmit: 0 errors
- ✅ cargo test --lib: **434 passed / 0 failed**
- ✅ check_historical_fixes.py: **562/562 PASS** (557 → 562, +5 §169 锚点)
- ✅ cargo build --release: 5m25s, binary 55M
- ✅ sync_app_bundle.sh: 3 binary 全 sync
- ⏳ GUI 端到端 (§15 强制, 用户必做)

## 铁律

1. 用户主动 "重新生成" 按钮 → 必须真调 LLM, 不允许 cache 复用
2. cache 优化只用于 "被动命中" (polling 重入 / 同输入再次生成)
3. `force_fresh` 是用户意图的明确信号, 不允许隐式缓存
4. 任何后续字段加入 cache_source 时, 必须评估 regenerate 路径是否需要 invalidate

## 关联

- §160 (in-flight guard, 防重复 invoke)
- §18 (云端 API 永不接入 — cache 复用仍然本地 LLM, 不算 cloud)
- §37 / §56 / §92

## §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

1. 打开任一会议 → 点 "重新生成"
2. 后端日志含 `§169 force_fresh=true, bypassing summary cache for meeting_id=...`
3. 进度条正常跑 (而非秒级完成)
4. DB 验证 chunk_count ≥ 1, processing_time > 5s
5. 连续点 2 次 → 应该有 2 个不同 content 的 summary_processes 行

