'use client';
import React, { createContext, useCallback, useContext, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

const STORAGE_SESSION = 'lixianhuiji.session';
export const STORAGE_LAST_EMAIL = 'lixianhuiji.last_email';

export type UserPublic = {
  id: number;
  email: string;
  display_name?: string | null;
  membership: 'free' | 'member';
  member_since?: string | null;
  machine_id?: string | null;
};

type AuthState = {
  session: string | null;
  user: UserPublic | null;
  machineId: string | null;
  loading: boolean;
  lastEmail: string | null;
};

type AuthApi = AuthState & {
  register: (email: string, password: string, displayName?: string) => Promise<{ ok: boolean; error?: string }>;
  login: (email: string, password: string) => Promise<{ ok: boolean; error?: string }>;
  logout: () => Promise<void>;
  refresh: () => Promise<void>;
  activateMember: () => Promise<boolean>;
};

const AuthContext = createContext<AuthApi | null>(null);

const ERR_ZH: Record<string, string> = {
  invalid_email: '邮箱格式不正确',
  weak_password: '密码至少 6 位',
  password_too_long: '密码过长',
  email_exists: '该邮箱已被注册',
  bad_credential: '邮箱或密码错误',
  banned: '账户已停用',
  db_error: '数据库错误',
  registration_failed: '注册失败',
  login_failed: '登录失败',
  not_logged_in: '请先登录',
  activation_failed: '激活失败',
  server_misconfigured: '数据库未初始化,请重启应用',
};
function errToMessage(code: string | undefined): string {
  if (!code) return '未知错误';
  return ERR_ZH[code] ?? code;
}

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [session, _setSession] = useState<string | null>(null);
  const [user, setUser] = useState<UserPublic | null>(null);
  const [machineId, setMachineId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [lastEmail, setLastEmail] = useState<string | null>(null);

  // mount: 从 localStorage 拿 session + 系统 machine_id + 自动 get_current_user
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const mid = await invoke<string>('system_machine_id');
        if (!cancelled) setMachineId(mid);
      } catch (e) {
        console.error('get machine id failed', e);
      }
      const saved = window.localStorage?.getItem(STORAGE_SESSION);
      if (saved) {
        try {
          const u = await invoke<UserPublic | null>('user_get_current', { session: saved });
          if (!cancelled) {
            _setSession(saved);
            setUser(u);
          }
        } catch (e) {
          console.error('get current user failed', e);
          window.localStorage?.removeItem(STORAGE_SESSION);
        }
      }
      if (!cancelled && !saved) {
        try {
          const restored = await invoke<{
            session?: string | null;
            user?: UserPublic | null;
            last_email?: string | null;
          }>('user_bootstrap');
          if (restored.last_email) {
            setLastEmail(restored.last_email);
            window.localStorage?.setItem(STORAGE_LAST_EMAIL, restored.last_email);
          }
          if (restored.session && restored.user) {
            window.localStorage?.setItem(STORAGE_SESSION, restored.session);
            _setSession(restored.session);
            setUser(restored.user);
          }
        } catch (e) {
          console.error('bootstrap local auth failed', e);
        }
      } else if (!cancelled) {
        setLastEmail(window.localStorage?.getItem(STORAGE_LAST_EMAIL));
      }
      if (!cancelled) setLoading(false);
    })();
    return () => { cancelled = true; };
  }, []);

  const persist = (s: string | null) => {
    if (s) window.localStorage?.setItem(STORAGE_SESSION, s);
    else window.localStorage?.removeItem(STORAGE_SESSION);
    _setSession(s);
  };

  const login = useCallback(async (email: string, password: string) => {
    console.log('[auth] login called email=', email);
    try {
      const res = await invoke<{ ok: boolean; session?: string; user?: UserPublic; error?: string }>('user_login', { email, password });
      console.log('[auth] login response', res);
      if (res.ok && res.session && res.user) {
        persist(res.session);
        setUser(res.user);
        setLastEmail(res.user.email);
        window.localStorage?.setItem(STORAGE_LAST_EMAIL, res.user.email);
        return { ok: true };
      }
      return { ok: false, error: errToMessage(res.error) };
    } catch (e) {
      console.error('[auth] login exception', e);
      return { ok: false, error: errToMessage('login_failed') };
    }
  }, []);

  const register = useCallback(async (email: string, password: string, displayName?: string) => {
    console.log('[auth] register called email=', email, 'displayName=', displayName);
    try {
      const res = await invoke<{ ok: boolean; session?: string; user?: UserPublic; error?: string }>(
        'user_register',
        { email, password, displayName: displayName ?? null }
      );
      console.log('[auth] register response', res);
      if (res.ok && res.session && res.user) {
        persist(res.session);
        setUser(res.user);
        setLastEmail(res.user.email);
        window.localStorage?.setItem(STORAGE_LAST_EMAIL, res.user.email);
        return { ok: true };
      }
      return { ok: false, error: errToMessage(res.error) };
    } catch (e) {
      console.error('[auth] register exception', e);
      return { ok: false, error: errToMessage('registration_failed') };
    }
  }, []);

  const logout = useCallback(async () => {
    if (session) {
      try { await invoke('user_logout', { session }); } catch {}
    }
    persist(null);
    setUser(null);
  }, [session]);

  const refresh = useCallback(async () => {
    if (!session) return;
    try {
      const u = await invoke<UserPublic | null>('user_get_current', { session });
      setUser(u);
    } catch {}
  }, [session]);

  const activateMember = useCallback(async () => {
    if (!session) return false;
    try {
      const u = await invoke<UserPublic>('user_activate_member', { session });
      setUser(u);
      return true;
    } catch (e) {
      console.error('activate failed', e);
      return false;
    }
  }, [session]);

  const value: AuthApi = { session, user, machineId, loading, lastEmail, login, register, logout, refresh, activateMember };
  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthApi {
  const c = useContext(AuthContext);
  if (!c) {
    return {
      session: null, user: null, machineId: null, loading: false, lastEmail: null,
      login: async () => ({ ok: false, error: 'no provider' }),
      register: async () => ({ ok: false, error: 'no provider' }),
      logout: async () => {},
      refresh: async () => {},
      activateMember: async () => false,
    };
  }
  return c;
}
