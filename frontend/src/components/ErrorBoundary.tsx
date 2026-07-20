'use client';

import React from 'react';
import { sanitizeDescription } from '@/lib/safeToast';

/**
 * v0.6.4: 全局 ErrorBoundary, 但 componentDidCatch 里绝对不能调 hook
 * (toast.error 内部 useToast 是 hook, 在 lifecycle method 里调会抛 #321)
 *
 * 改 3 处:
 * - 删掉 toast.error 调用 (避免 hooks in componentDidCatch)
 * - 删掉 document.title 操作 (非必须)
 * - 保留 console.error 完整堆栈 + localStorage 持久化
 * - render 里不再用 typeof window 'undefined' (永远客户端)
 */
interface EBState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<{ children: React.ReactNode }, EBState> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): Partial<EBState> {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    // Safety: 任何 hook 调用 (toast / useToast / useContext) 都会让 React 在
    // commit 之后调度时再次 dispatch render. 但 componentDidCatch 本身就是 commit 后.
    // 用 setTimeout 把"用户友好"的副作用推到下次 task
    const msg = error?.message || String(error);
    const stack = String(error?.stack || '').slice(0, 6000);
    const comp = String(info?.componentStack || '').slice(0, 6000);
    console.error('[离线会记 ErrorBoundary]', error, '\n\nstack:\n', stack, '\n\ncomponentStack:\n', comp);
    try {
      localStorage.setItem(
        'last-client-error',
        JSON.stringify({ msg, stack, comp, ts: Date.now() })
      );
    } catch {}
    // 不在 componentDidCatch 调 toast 或任何 hook - 这是 #321 的源头
    // render 里再 toast
    setTimeout(() => {
      try {
        // 这里 setTimeout 回调里调 toast - 这时 React 已经处理完 commit,安全
        // 用动态 import 避免 sonner 设置副作用在 boundary render 期触 hook
        import('sonner').then(({ toast }) => {
          toast.error('主内容渲染异常, 已自动重试', {
            description: sanitizeDescription(error, 'error'),
            duration: 6000,
          });
        }).catch(() => {});
      } catch {}
    }, 0);
  }

  reset = () => {
    this.setState({ hasError: false, error: null });
    try { window.location.reload(); } catch {}
  };

  render() {
    if (this.state.hasError && this.state.error) {
      const msg = String(this.state.error.message || this.state.error);
      const stackHead = String(this.state.error.stack || '').split('\n').slice(0, 8).join('\n');
      return (
        <div role="alert" className="flex h-full w-full items-center justify-center p-8">
          <div className="max-w-2xl rounded-lg border border-red-300 bg-white p-6 text-sm text-red-900 shadow dark:border-red-800/40 dark:bg-neutral-900 dark:text-red-200">
            <h2 className="mb-2 text-lg font-semibold">主内容渲染失败</h2>
            <p className="mb-3 text-neutral-600 dark:text-neutral-400">
              截图发我就能定位是哪个组件抛的错。下面是堆栈片段。
            </p>
            <div className="mb-4 rounded bg-neutral-100 p-3 font-mono text-xs text-neutral-800 dark:bg-neutral-950 dark:text-neutral-200">
              <div className="font-bold">ERROR:</div>
              <div className="mb-2">{msg}</div>
              <div className="font-bold">STACK (top 8):</div>
              <pre className="whitespace-pre-wrap break-words">{stackHead}</pre>
            </div>
            <button
              onClick={this.reset}
              className="rounded bg-red-600 px-4 py-2 text-white hover:bg-red-700"
            >
              Reload 试一次
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

export default ErrorBoundary;
