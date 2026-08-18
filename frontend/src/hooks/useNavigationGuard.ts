'use client';

import { useEffect, useRef, useState, useCallback } from 'react';

export type PendingNav = {
  /** 目标 url (string), 或 'beforeunload' 浏览器关闭, 或 'popstate' 后退 */
  to: string;
  type: 'push' | 'replace' | 'popstate' | 'beforeunload';
};

export type UseNavigationGuardOptions = {
  /** 启用拦截 */
  when: boolean;
  /** Dialog 标题 (i18n key) */
  title: string;
  /** Dialog 描述 (i18n key) */
  description: string;
  /** "继续离开" 按钮文案 (i18n key) */
  confirmText: string;
  /** "继续生成" 按钮文案 (i18n key, 保留防未来扩展) */
  cancelText: string;
};

export type UseNavigationGuardReturn = {
  pendingNav: PendingNav | null;
  confirm: () => void;
  cancel: () => void;
};

/**
 * §137: 摘要生成时拦截所有页面跳转
 * - 拦截 window.history.pushState / replaceState (Next.js router.push / .replace)
 * - 拦截 popstate (浏览器后退)
 * - 拦截 beforeunload (浏览器刷新/关闭)
 * - 当 when=true 时, 检测到任何跳转行为 → 设 pendingNav, 弹 modal 让用户确认
 * - 确认 → 执行跳转, 取消 → 还原到当前 url (popstate 情况)
 *
 * 不破坏 Next.js: 还原时用原始 history.pushState (保存的引用)
 * 不重复拦截: pushState 检测到 url === 当前 pathname + 相同 query 时不拦
 */
export function useNavigationGuard({
  when,
  title,
  description,
  confirmText,
  cancelText: _cancelText,
}: UseNavigationGuardOptions): UseNavigationGuardReturn {
  const [pendingNav, setPendingNav] = useState<PendingNav | null>(null);

  const originalPushRef = useRef<typeof window.history.pushState | null>(null);
  const originalReplaceRef = useRef<typeof window.history.replaceState | null>(null);

  const pendingNavRef = useRef<PendingNav | null>(null);
  pendingNavRef.current = pendingNav;

  const pendingArgsRef = useRef<Parameters<typeof window.history.pushState> | null>(null);

  const confirm = useCallback(() => {
    const nav = pendingNavRef.current;
    if (!nav) return;

    if (nav.type === 'beforeunload' || nav.type === 'popstate') {
      setPendingNav(null);
      pendingNavRef.current = null;
      return;
    }

    if ((nav.type === 'push' || nav.type === 'replace') && pendingArgsRef.current) {
      const args = pendingArgsRef.current;
      if (nav.type === 'push' && originalPushRef.current) {
        originalPushRef.current.apply(window.history, args);
      } else if (nav.type === 'replace' && originalReplaceRef.current) {
        originalReplaceRef.current.apply(window.history, args);
      }
      pendingArgsRef.current = null;
    }

    setPendingNav(null);
    pendingNavRef.current = null;
  }, []);

  const cancel = useCallback(() => {
    const nav = pendingNavRef.current;

    if (nav?.type === 'popstate') {
      if (originalPushRef.current) {
        originalPushRef.current.call(
          window.history,
          null,
          '',
          window.location.pathname + window.location.search + window.location.hash
        );
      }
    }

    pendingArgsRef.current = null;
    setPendingNav(null);
    pendingNavRef.current = null;
  }, []);

  useEffect(() => {
    if (!when) {
      setPendingNav(null);
      pendingArgsRef.current = null;
      return;
    }

    const originalPush = window.history.pushState.bind(window.history);
    const originalReplace = window.history.replaceState.bind(window.history);
    originalPushRef.current = originalPush;
    originalReplaceRef.current = originalReplace;

    const getCurrentKey = () => window.location.pathname + window.location.search;

    window.history.pushState = function (
      ...args: Parameters<typeof window.history.pushState>
    ) {
      const targetUrl = args[2] as string | undefined;
      const targetKey = targetUrl
        ? new URL(targetUrl, window.location.origin).pathname +
          new URL(targetUrl, window.location.origin).search
        : getCurrentKey();

      if (targetKey === getCurrentKey()) {
        return originalPush(...args);
      }

      pendingArgsRef.current = args;
      setPendingNav({ to: targetUrl ?? '', type: 'push' });
    };

    window.history.replaceState = function (
      ...args: Parameters<typeof window.history.replaceState>
    ) {
      const targetUrl = args[2] as string | undefined;
      const targetKey = targetUrl
        ? new URL(targetUrl, window.location.origin).pathname +
          new URL(targetUrl, window.location.origin).search
        : getCurrentKey();

      if (targetKey === getCurrentKey()) {
        return originalReplace(...args);
      }

      pendingArgsRef.current = args as unknown as Parameters<typeof window.history.pushState>;
      setPendingNav({ to: targetUrl ?? '', type: 'replace' });
    };

    const handlePopState = (_event: PopStateEvent) => {
      setPendingNav({ to: 'popstate', type: 'popstate' });
    };

    const handleBeforeUnload = (event: BeforeUnloadEvent) => {
      if (pendingNavRef.current) {
        return;
      }
      event.preventDefault();
      event.returnValue = title || '';
      setPendingNav({ to: 'beforeunload', type: 'beforeunload' });
    };

    window.addEventListener('popstate', handlePopState);
    window.addEventListener('beforeunload', handleBeforeUnload);

    return () => {
      window.history.pushState = originalPush;
      window.history.replaceState = originalReplace;
      window.removeEventListener('popstate', handlePopState);
      window.removeEventListener('beforeunload', handleBeforeUnload);
    };
  }, [when, title]);

  return { pendingNav, confirm, cancel };
}
