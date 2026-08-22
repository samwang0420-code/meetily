// §160: Tauri invoke wrapper — adds timeout + retry-with-backoff.
//
// 背景 (2026-08-22): 用户反馈"最新的录音, 第一次生成摘要失败, 第二次成功".
// 调查 meeting-709b4aba-41a4-4217-a1d3-986bf389daa5 发现 DB summary_processes
// 只有 1 条 completed 行 (4 chunks / 601.2s), 没有任何 failed/pending row,
// 证明第一次 invoke 在 Tauri IPC 层被静默丢掉 (Promise 永远 pending, 不 resolve
// 也不 reject, UI 卡在 'processing' 状态, 用户看到的是"失败").
//
// 这是 Tauri 2 macOS webview 已知上游 bug, 我们代码逻辑没问题. 客户端能做的就是
// 给 invoke 加 timeout 边界 + retry 兜底, 让用户感知到失败 + 自动恢复.
//
// §160 默认参数: timeoutMs=30s (用户耐心阈值), retries=1 (二次大概率成功),
// backoffMs=500ms (避开 webview 状态切换瞬间).
//
// 只对长链路 / 关键 invoke 用这个 wrapper, 不要全局替换, 避免引入额外 risk surface.

import { invoke as invokeTauri } from '@tauri-apps/api/core';

export interface InvokeTimeoutOptions {
  timeoutMs?: number;
  retries?: number;
  backoffMs?: number;
  /** 每次 retry 前触发, 用于埋点 / 日志 */
  onRetry?: (attempt: number, err: unknown) => void;
}

export class InvokeTimeoutError extends Error {
  readonly cmd: string;
  readonly timeoutMs: number;
  constructor(cmd: string, timeoutMs: number) {
    super(`[invokeWithTimeout] ${cmd} timed out after ${timeoutMs}ms`);
    this.name = 'InvokeTimeoutError';
    this.cmd = cmd;
    this.timeoutMs = timeoutMs;
  }
}

export async function invokeWithTimeout<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
  opts: InvokeTimeoutOptions = {}
): Promise<T> {
  const timeoutMs = opts.timeoutMs ?? 30_000;
  const retries = Math.max(0, opts.retries ?? 1);
  const backoffMs = opts.backoffMs ?? 500;

  let lastError: unknown;
  for (let attempt = 0; attempt <= retries; attempt++) {
    try {
      const invokePromise = invokeTauri<T>(cmd, args);
      let timer: ReturnType<typeof setTimeout> | null = null;
      const timeoutPromise = new Promise<never>((_, reject) => {
        timer = setTimeout(
          () => reject(new InvokeTimeoutError(cmd, timeoutMs)),
          timeoutMs
        );
      });
      try {
        return await Promise.race([invokePromise, timeoutPromise]);
      } finally {
        if (timer) clearTimeout(timer);
      }
    } catch (err) {
      lastError = err;
      if (attempt < retries) {
        try {
          opts.onRetry?.(attempt + 1, err);
        } catch {
          // onRetry 回调自身抛错不阻塞主流程
        }
        await new Promise((r) => setTimeout(r, backoffMs));
      }
    }
  }
  throw lastError;
}
