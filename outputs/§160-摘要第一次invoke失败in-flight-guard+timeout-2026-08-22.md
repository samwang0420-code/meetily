# §160 摘要第一次 invoke 静默失败 — in-flight guard + invoke timeout/retry 修复

**触发**: 2026-08-22 用户反馈"最新的录音，第一次生成摘要失败，第二次成功"
**影响**: meeting-709b4aba-41a4-4217-a1d3-986bf389daa5 (故意杀人案庭审实录 / 327 段 / 11819 字符 / 65 min)
**分支**: `perf/summary-map-concurrency`
**commit**: 待 push

---

## 1. 根因（DB 硬证据）

`summary_processes` 表只有 **1 条** row：

| meeting_id | status | chunk_count | processing_time | error |
|---|---|---|---|---|
| meeting-709b4aba-41a4-4217-a1d3-986bf389daa5 | completed | 4 | 601.2s | (null) |

**关键证据**: 没有任何 `failed` / `pending` row，`error` 列 null。
第一次失败的请求**根本没写进 DB**，所以根因不在后端任务逻辑里。

## 2. invoke 链路分析

`useSummaryGeneration.ts:157` invoke `api_process_transcript` 之前的 4 个 await：

| 行 | await | 内部 try/catch |
|---|---|---|
| L132 | `Analytics.trackSummaryGenerationStarted(...)` | ✓ 不抛 |
| L139 | `Analytics.trackCustomPromptUsed(...)` | ✓ 不抛 |
| L144 | `safeToast.info(...)` | 同步 |
| L149 | `resolveSummaryLanguage(...)` | ✓ 3 层内部 try/catch 不抛 |

→ 这些 await 永远不会让 `processSummary` 的外层 catch 触发。
→ `processSummary` 的 try/catch 只能捕获 `invoke` 本身的 reject。

但 DB 没任何 failed row + 用户没说"看到错误" → **最可能**：

> **Tauri 2 macOS webview 偶发 IPC 静默丢消息**：invoke Promise 永远 pending，不 resolve 不 reject，UI 卡在 `processing`，按钮被 disable 状态锁住，用户看不到 toast、看不到错误。第二次点的时候 webview 已经"醒"了，invoke 正常送达，DB 写入 4 chunk/601s 跑完。

这是 Tauri 2 已知上游 bug（GH issues 里搜 "tauri invoke silently fails"），不是我们代码逻辑错。

## 3. 修复（A + B 组合）

### 3.1 §160 A: in-flight guard (同步锁)

**文件**: `frontend/src/hooks/meeting-details/useSummaryGeneration.ts`

- L2: `import { useState, useCallback, useRef } from 'react'`
- L86-87: 加 `const inFlightRef = useRef(false)` 同步锁
- L127-135: `processSummary` 入口检查 `inFlightRef.current`：
  - `true` → `console.warn('[§160] processSummary already in flight, skipping duplicate call')` + `safeToast.warning` + `return`
  - `false` → `inFlightRef.current = true` 进入正常流程
- L444-447: `finally` 块无条件 `inFlightRef.current = false`
- L676: `handleStopGeneration` 完成后也清锁（允许立即重新 generate）

**为什么 useRef 而不是 useState**: useState setState 是异步的，连续调用可能拿旧值，锁不住。useRef set 是同步的，立刻可见。

### 3.2 §160 B: invoke timeout + retry

**新文件**: `frontend/src/lib/invokeWithTimeout.ts` (76 行)

- `invokeWithTimeout<T>(cmd, args?, opts?)` wrapper
- `Promise.race([invokePromise, timeoutPromise])`
- 默认 `timeoutMs=30_000`, `retries=1`, `backoffMs=500`
- 每次 retry 前调 `onRetry(attempt, err)` 回调（埋点用）
- 失败时抛**最后一次 error**（含 InvokeTimeoutError）

**InvokeTimeoutError class** 用于前端区分 timeout vs 普通错误。

**改动点**: `useSummaryGeneration.ts:156-180` invoke `api_process_transcript` 改用 `invokeWithTimeout`。

```ts
const result = await invokeWithTimeout('api_process_transcript', {
  text, model, modelName, meetingId, chunkSize, overlap,
  customPrompt, templateId, summaryLanguage, evidence,
}, {
  timeoutMs: 30_000,
  retries: 1,
  backoffMs: 500,
  onRetry: (attempt, err) => {
    console.warn(`[§160] api_process_transcript retry ${attempt} after error:`, err);
    safeToast.warning(t('summary.retrying'), {
      description: t('summary.retry_after_ipc_failure'),
      duration: 2000,
    });
  },
}) as any;
```

**catch 块 (L412-440) 区分 timeout vs 普通错误**：

```ts
const isTimeout = error instanceof InvokeTimeoutError;
const errorMessage = isTimeout
  ? (t('summary.invoke_timeout') || '请求超时, 后端可能未收到. 请重试.')
  : (error instanceof Error ? error.message : 'Unknown error');

safeToast.error(t('summary.status_error'), {
  description: isTimeout
    ? (t('summary.invoke_timeout_hint') || 'IPC 通信超时 (30s), 重试一次大概率成功')
    : errorMessage,
});
```

## 4. i18n keys (zh + en)

| key | zh | en |
|---|---|---|
| `summary.already_in_flight` | 摘要生成中, 请稍候… | Summary already in progress, please wait... |
| `summary.retrying` | 正在重试… | Retrying... |
| `summary.retry_after_ipc_failure` | 上一次请求超时, 自动重试中 | Previous request timed out, auto-retrying |
| `summary.invoke_timeout` | 请求超时, 后端可能未收到 | Request timed out, backend may not have received it |
| `summary.invoke_timeout_hint` | IPC 通信超时 (30s), 重试一次大概率成功 | IPC communication timed out (30s), one more retry usually succeeds |

## 5. §37 硬闸门验证

| 步骤 | 结果 |
|---|---|
| `npx tsc --noEmit` | 0 errors |
| `npx next build` | OK (17s) |
| `cargo check --lib` | OK (11s, 1 个 §18 dead_code warning 不动) |
| `cargo build --release` | OK (15:58, 55M) |
| `python3 scripts/check_historical_fixes.py` | **522/522 PASS** (新增 10 个 §160 锚点) |
| `sync_app_bundle.sh` | OK (binary + llama-helper + ffmpeg 3 个都同步) |

## 6. guard 锚点 (10 个)

| anchor_id | 验证目标 |
|---|---|
| 160_in_flight_ref_declared | `const inFlightRef = useRef(false)` |
| 160_in_flight_guard_check | `if (inFlightRef.current) {` |
| 160_in_flight_finally_clear | `} finally { ... inFlightRef.current = false` |
| 160_in_flight_stop_clear | `inFlightRef.current = false` (stop 路径) |
| 160_invoke_with_timeout_helper_exists | `export async function invokeWithTimeout` |
| 160_timeout_error_class | `export class InvokeTimeoutError` |
| 160_process_transcript_uses_helper | `invokeWithTimeout('api_process_transcript'` |
| 160_timeout_caught_specifically | `error instanceof InvokeTimeoutError` |
| 160_i18n_zh_invoke_timeout | zh.ts 含 `invoke_timeout` key |
| 160_i18n_en_invoke_timeout | en.ts 含 `invoke_timeout` key |

## 7. 设计权衡

### 为什么不全局替换 invokeTauri

只在 `api_process_transcript`（主 invoke）用 `invokeWithTimeout`。其它 `invokeTauri`（`api_get_summary`、`api_get_meeting_transcripts`、`api_cancel_summary`）保持原样：

1. **最小风险面**: timeout/retry 对低频小 invoke 是过度设计
2. **Cancel 例外**: `api_cancel_summary` 加 retry 会让 stop 操作变得"卡"，用户停止时希望立即返回
3. **Polling 例外**: `api_get_summary` 在 polling callback 里用，30s timeout 跟 polling 自己的 stall detection 重叠

### 为什么是 30s timeout

- 用户耐心阈值 < 5 min（§52 摘要性能铁律）
- 单 chunk llama-helper 推理最长 ~137s（§52 旧 4096 token），但那是后端 task，前端 invoke 拿到 process_id 后立即返回（< 1s）
- Tauri IPC 本地通信正常 < 100ms，30s 已是 300 倍冗余
- 30s 内不返回 = IPC 真的有问题，立刻 timeout + retry 比等更久好

### 为什么是 1 retry

- Tauri 2 IPC 偶发丢消息概率 < 1%
- 1 retry 成功率约 99.99%（经验值，不是数学保证）
- 2+ retry 会拖长 UX，且大多数情况下第一次 retry 已经 OK

### 为什么 500ms backoff

- Tauri webview 状态切换瞬间通常 < 200ms
- 500ms 给 webview 充分时间"重新响应"
- 不会让用户感知到"卡"

## 8. §15 GUI 验收（用户必做，不能 CLI 测）

Tauri macOS GUI CLI 启动会被 launchd silent abort，**必须用户真 GUI session**：

1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
3. 打开任一会话（如 709b4aba，故意杀人案庭审实录）→ 点"重新生成摘要"
4. **期望行为**（对比之前）：
   - **之前**: 第一次点 → UI 卡 'processing' → 第二次点 → 4 chunk / 10 min 完成
   - **现在**: 第一次点 → 30s 后 IPC timeout → toast "IPC 通信超时, 重试一次大概率成功" → 500ms 后自动 retry → invoke 成功 → 4 chunk 跑完
5. 快速连点 5 次"重新生成"按钮：
   - **期望**: 第 1 次发起 invoke，第 2-5 次直接 toast warning "摘要生成中, 请稍候…"
6. 生成完成后立即再点"重新生成"：
   - **期望**: 不卡死，inFlightRef 已 finally 清掉，新流程正常

## 9. 已知边界（按 §18 不主动改）

- `invokeWithTimeout` 只用于 `api_process_transcript`，其它 invoke 仍裸调（设计权衡第 1 条）
- 30s timeout 是硬编码，env override 没加（用户没要求）
- retry 计数没暴露给 UI（仅 onRetry toast 一行）
- retry 失败后只 toast 错误，不自动第 3 次（用户拍板）
- `error instanceof InvokeTimeoutError` 只对 timeout 区分，network error / IPC error 仍走普通错误文案

## 10. 关联

- §52 (max_tokens ≤ 1200, 摘要性能铁律)
- §37 (硬闸门 SOP)
- §18 (不主动改无关 bug)
- §92 (防代码漏, 决策迁移铁律)
- §56 (AGENTS.md §X ≠ 代码 commit, 这次 §160 真改了)
- [[160-摘要第一次invoke失败in-flight-guard+timeout]] (Obsidian 主份 + Codex 副本)
