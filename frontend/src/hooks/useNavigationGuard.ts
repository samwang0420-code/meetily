'use client';

import { useEffect, useRef, useState, useCallback } from 'react';

// ===================================================================
// §137.1 Module-level singleton: 跨 component lifecycle 保持 pushState wrapper
// 修复 React 异步 setPendingNav 跟 useEffect cleanup race,导致 dialog 不弹
// (用户实测: 摘要生成中点 Sidebar 链接, dialog 永久不弹)
// ===================================================================

type PendingNav = {
  /** 目标 url (string), 或 'beforeunload' 浏览器关闭, 或 'popstate' 后退 */
  to: string;
  type: 'push' | 'replace' | 'popstate' | 'beforeunload';
};

type Subscriber = (nav: PendingNav | null) => void;

let wrapperInstalled = false;
let originalPush: typeof window.history.pushState | null = null;
let originalReplace: typeof window.history.replaceState | null = null;
let refCount = 0;
let currentPendingNav: PendingNav | null = null;
let pendingArgs: Parameters<typeof window.history.pushState> | null = null;
const subscribers = new Set<Subscriber>();

const getCurrentKey = (): string => {
  if (typeof window === 'undefined') return '';
  return window.location.pathname + window.location.search;
};

const notify = (nav: PendingNav | null) => {
  currentPendingNav = nav;
  for (const sub of subscribers) {
    try { sub(nav); } catch { /* ignore subscriber errors */ }
  }
};

const handlePopState = () => {
  pendingArgs = null;
  notify({ to: 'popstate', type: 'popstate' });
};

const handleBeforeUnload = (event: BeforeUnloadEvent) => {
  if (currentPendingNav) return;
  event.preventDefault();
  event.returnValue = '';
  notify({ to: 'beforeunload', type: 'beforeunload' });
};

const installWrapper = (): boolean => {
  if (wrapperInstalled || typeof window === 'undefined') return wrapperInstalled;
  wrapperInstalled = true;
  originalPush = window.history.pushState.bind(window.history);
  originalReplace = window.history.replaceState.bind(window.history);

  window.history.pushState = function (...args: Parameters<typeof window.history.pushState>) {
    const targetUrl = args[2] as string | undefined;
    const targetKey = targetUrl
      ? new URL(targetUrl, window.location.origin).pathname + new URL(targetUrl, window.location.origin).search
      : getCurrentKey();
    if (targetKey === getCurrentKey()) {
      return originalPush!.apply(window.history, args);
    }
    pendingArgs = args;
    notify({ to: targetUrl ?? '', type: 'push' });
  };

  window.history.replaceState = function (...args: Parameters<typeof window.history.replaceState>) {
    const targetUrl = args[2] as string | undefined;
    const targetKey = targetUrl
      ? new URL(targetUrl, window.location.origin).pathname + new URL(targetUrl, window.location.origin).search
      : getCurrentKey();
    if (targetKey === getCurrentKey()) {
      return originalReplace!.apply(window.history, args);
    }
    pendingArgs = args as unknown as Parameters<typeof window.history.pushState>;
    notify({ to: targetUrl ?? '', type: 'replace' });
  };

  window.addEventListener('popstate', handlePopState);
  window.addEventListener('beforeunload', handleBeforeUnload);
  return true;
};

const uninstallWrapper = () => {
  if (!wrapperInstalled) return;
  window.history.pushState = originalPush!;
  window.history.replaceState = originalReplace!;
  window.removeEventListener('popstate', handlePopState);
  window.removeEventListener('beforeunload', handleBeforeUnload);
  wrapperInstalled = false;
  originalPush = null;
  originalReplace = null;
  pendingArgs = null;
  currentPendingNav = null;
  notify(null);
};

const acquireWrapper = () => {
  refCount += 1;
  if (refCount === 1) {
    installWrapper();
  }
};

const releaseWrapper = () => {
  refCount = Math.max(0, refCount - 1);
  if (refCount === 0 && currentPendingNav === null) {
    uninstallWrapper();
  }
};

const moduleConfirm = () => {
  const nav = currentPendingNav;
  if (!nav) return;
  if (nav.type === 'beforeunload' || nav.type === 'popstate') {
    pendingArgs = null;
    notify(null);
    return;
  }
  if ((nav.type === 'push' || nav.type === 'replace') && pendingArgs) {
    const args = pendingArgs;
    if (nav.type === 'push' && originalPush) {
      originalPush.apply(window.history, args);
    } else if (nav.type === 'replace' && originalReplace) {
      originalReplace.apply(window.history, args as unknown as Parameters<typeof window.history.replaceState>);
    }
    pendingArgs = null;
  }
  notify(null);
  if (refCount === 0) {
    uninstallWrapper();
  }
};

const moduleCancel = () => {
  const nav = currentPendingNav;
  if (nav?.type === 'popstate' && originalPush) {
    originalPush.call(
      window.history,
      null,
      '',
      window.location.pathname + window.location.search + window.location.hash,
    );
  }
  pendingArgs = null;
  notify(null);
  if (refCount === 0) {
    uninstallWrapper();
  }
};

export type { PendingNav };

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
 * §137.1: 摘要生成时拦截所有页面跳转
 * - Module-level singleton: pushState wrapper 跨 component lifecycle 保持
 * - 修复 React 异步 setPendingNav 跟 useEffect cleanup race 导致 dialog 不弹
 * - 引用计数管理 wrapper 生命周期, 支持多 hook 实例共存
 * - 有 pendingNav 时不卸载 wrapper, 防止 dialog 在 React unmount 后无法响应
 */
export function useNavigationGuard({
  when,
  title,
  description,
  confirmText,
  cancelText: _cancelText,
}: UseNavigationGuardOptions): UseNavigationGuardReturn {
  const [pendingNav, setPendingNav] = useState<PendingNav | null>(currentPendingNav);
  const subscriptionRef = useRef<Subscriber | null>(null);

  void title; void description; void confirmText;

  useEffect(() => {
    if (!when) {
      if (subscriptionRef.current) {
        subscribers.delete(subscriptionRef.current);
        subscriptionRef.current = null;
      }
      releaseWrapper();
      setPendingNav(null);
      return;
    }

    const sub: Subscriber = (nav) => {
      setPendingNav(nav);
    };
    subscribers.add(sub);
    subscriptionRef.current = sub;
    acquireWrapper();

    setPendingNav(currentPendingNav);

    return () => {
      if (subscriptionRef.current) {
        subscribers.delete(subscriptionRef.current);
        subscriptionRef.current = null;
      }
      releaseWrapper();
    };
  }, [when]);

  const cancel = useCallback(() => {
    moduleCancel();
    setPendingNav(null);
  }, []);

  const confirm = useCallback(() => {
    moduleConfirm();
    setPendingNav(null);
  }, []);

  return { pendingNav, confirm, cancel };
}
