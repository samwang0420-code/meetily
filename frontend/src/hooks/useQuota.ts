// 离线会记 v0.6.10+: 商业化配额 hook
// 每次挂载/登录/录制后刷新配额状态

'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface QuotaState {
  tier: 'anonymous' | 'free' | 'member';
  month_meetings_used: number;
  month_meetings_limit: number;     // -1 = 无限制
  segments_per_transcript_limit: number; // -1 = 无限制
  can_record: boolean;
  reason: string | null;
}

const DEFAULT_QUOTA: QuotaState = {
  tier: 'anonymous',
  month_meetings_used: 0,
  month_meetings_limit: 1,  // anonymous 默认 1 次 (兜底, 实际后端为准)
  segments_per_transcript_limit: 100,
  can_record: true,
  reason: null,
};

// localStorage key for anonymous (未登录) trial 用过的次数
// C1: 防止未登录用户清缓存反复刷试用次数
const ANON_KEY = 'lixianhuiji.anonymous_trial_meetings';

export function getAnonymousUsedCount(): number {
  if (typeof window === 'undefined') return 0;
  try {
    const v = window.localStorage.getItem(ANON_KEY);
    return v ? Math.max(0, parseInt(v, 10) || 0) : 0;
  } catch { return 0; }
}

export function incrementAnonymousUsedCount(): number {
  if (typeof window === 'undefined') return 0;
  const cur = getAnonymousUsedCount();
  const next = cur + 1;
  try {
    window.localStorage.setItem(ANON_KEY, String(next));
  } catch {}
  return next;
}

function applyAnonymous(state: QuotaState): QuotaState {
  if (state.tier !== 'anonymous') return state;
  const used = getAnonymousUsedCount();
  const limit = state.month_meetings_limit;
  return {
    ...state,
    month_meetings_used: used,
    can_record: used < limit,
    reason: used >= limit ? '试用已达上限, 请注册账号继续使用' : null,
  };
}

export function useQuota(session: string | null): {
  quota: QuotaState;
  loading: boolean;
  refresh: () => Promise<void>;
  recordAfterSave: () => Promise<void>;
} {
  const [quota, setQuota] = useState<QuotaState>(DEFAULT_QUOTA);
  const [loading, setLoading] = useState(true);
  const lastSession = useRef<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setLoading(true);
      const r = await invoke<QuotaState>('quota_get_status', { session });
      // anon 用户配额跟 localStorage 同步
      setQuota(applyAnonymous(r));
    } catch (e) {
      console.warn('[useQuota] refresh failed', e);
      setQuota(applyAnonymous(DEFAULT_QUOTA));
    } finally {
      setLoading(false);
    }
  }, [session]);

  const recordAfterSave = useCallback(async () => {
    if (session) {
      try {
        const r = await invoke<QuotaState>('quota_increment_after_record', { session });
        setQuota(applyAnonymous(r));
      } catch (e) {
        console.warn('[useQuota] increment failed', e);
      }
    } else {
      // anonymous: 更新本地计数
      const next = incrementAnonymousUsedCount();
      setQuota((cur) => applyAnonymous({ ...cur, month_meetings_used: next }));
    }
  }, [session]);

  // 每次 session 变化刷新 (登录后/登出后)
  useEffect(() => {
    if (lastSession.current !== session) {
      lastSession.current = session;
      refresh();
    }
  }, [session, refresh]);

  // 监听全局 quota 变化事件 (来自 useRecordingStop save 完成)
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail && typeof detail === 'object') {
        // detail 可能是后端 QuotaStatus 或 { used: number } 匿名
        if ('tier' in detail) {
          setQuota(applyAnonymous(detail as QuotaState));
        } else if ('used' in detail) {
          setQuota((cur) => applyAnonymous({ ...cur, month_meetings_used: (detail as { used: number }).used }));
        }
      }
    };
    window.addEventListener('lixianhuiji:quota-changed', handler);
    return () => window.removeEventListener('lixianhuiji:quota-changed', handler);
  }, []);

  return { quota, loading, refresh, recordAfterSave };
}
