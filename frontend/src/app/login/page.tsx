'use client';

import React, { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { motion } from 'framer-motion';
import { Mail, Lock, Eye, EyeOff, AlertCircle, Loader2, ArrowLeft } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { STORAGE_LAST_EMAIL, useAuth } from '@/contexts/AuthContext';
import { BrandShield } from '@/components/BrandShield';

// §147: 言镜 AI 官网重做 — login 页 (与 register 视觉一致)
// 13 → 4 icon · 颜色 token 化 · 极简错误提示 · 注册引导单行

export default function LoginPage() {
  const { t } = useTranslation();
  const router = useRouter();
  const { login, user, loading, lastEmail } = useAuth();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [showPw, setShowPw] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [emailFocused, setEmailFocused] = useState(false);
  const [pwFocused, setPwFocused] = useState(false);

  // 上次登录邮箱预填
  useEffect(() => {
    if (typeof window === 'undefined') return;
    const rememberedEmail = lastEmail || window.localStorage.getItem(STORAGE_LAST_EMAIL);
    if (rememberedEmail) setEmail((cur) => cur || rememberedEmail);
  }, [lastEmail]);

  useEffect(() => {
    if (!loading && user) router.replace('/');
  }, [user, loading, router]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (busy) return;
    setError(null);
    if (!email || !password) {
      setError(t('login_page.error_empty'));
      return;
    }
    setBusy(true);
    const r = await login(email.trim(), password);
    setBusy(false);
    if (r.ok) {
      try {
        window.localStorage.setItem(STORAGE_LAST_EMAIL, email.trim());
      } catch {}
      router.push('/');
    } else {
      setError(r.error ?? t('errors.generic'));
    }
  }

  return (
    <div className="min-h-screen bg-[var(--app-canvas)] text-[var(--app-ink)] grid lg:grid-cols-[1.1fr_1fr]">
      {/* ─── 左: 品牌面板 ─── */}
      <aside className="relative hidden lg:flex flex-col justify-between overflow-hidden border-r border-[var(--app-hairline)] p-12">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 opacity-50"
          style={{
            background:
              'radial-gradient(ellipse at 25% 15%, rgba(94,106,210,0.18) 0%, transparent 55%), radial-gradient(ellipse at 85% 90%, rgba(255,197,51,0.10) 0%, transparent 55%)',
          }}
        />

        <Link href="/" className="relative inline-flex items-center gap-3 text-sm text-[var(--app-ink-muted)] hover:text-[var(--app-ink)] transition-colors w-fit">
          <ArrowLeft className="w-4 h-4" />
          {t('account.back_to_home')}
        </Link>

        <div className="relative">
          <motion.div
            initial={{ opacity: 0, scale: 0.92 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.5 }}
            className="mb-8"
          >
            <BrandShield size={64} />
          </motion.div>
          <h2 className="text-[clamp(1.8rem,3.2vw,2.6rem)] font-semibold leading-[1.15] tracking-tight mb-3">
            {t('login_page.hero_title')}
            <br />
            <span className="bg-gradient-to-r from-[var(--app-transcript-hover)] to-[var(--app-summary)] bg-clip-text text-transparent">
              {t('login_page.hero_highlight')}
            </span>
          </h2>
          <p className="text-sm text-[var(--app-ink-muted)] max-w-sm leading-relaxed">
            {t('login_page.hero_desc')}
          </p>

          <ul className="mt-10 space-y-3 max-w-sm">
            <Bullet label={t('login_page.feature_local')} />
            <Bullet label={t('login_page.feature_resume')} />
            <Bullet label={t('login_page.feature_buyout')} />
          </ul>
        </div>

        <p className="relative text-[11px] text-[var(--app-ink-subtle)]">
          © {new Date().getFullYear()} {t('login_page.copyright')}
        </p>
      </aside>

      {/* ─── 右: 表单 ─── */}
      <main className="flex items-center justify-center p-6 sm:p-10">
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4, ease: 'easeOut' }}
          className="w-full max-w-[420px]"
        >
          <div className="mb-8 flex items-center gap-2.5 lg:hidden">
            <BrandShield size={28} />
            <span className="text-[15px] font-semibold tracking-tight">{t('app.name')}</span>
          </div>

          <h1 className="text-[clamp(1.6rem,3vw,2rem)] font-semibold tracking-tight">
            {t('account.login_title')}
          </h1>
          <p className="mt-1.5 text-sm text-[var(--app-ink-subtle)]">
            {t('login_page.subtitle')}
          </p>

          <form onSubmit={handleSubmit} className="mt-7 space-y-3">
            <div>
              <label className="mb-1.5 block text-xs font-medium text-[var(--app-ink-muted)]">
                {t('account.email')}
              </label>
              <div
                className={`flex items-center gap-2.5 rounded-xl border bg-[var(--app-surface-1)] px-3.5 py-2.5 transition-colors ${
                  emailFocused
                    ? 'border-[var(--app-transcript)] shadow-[0_0_0_3px_rgba(94,106,210,0.18)]'
                    : 'border-[var(--app-hairline)]'
                }`}
              >
                <Mail className={`h-4 w-4 transition-colors ${emailFocused ? 'text-[var(--app-transcript)]' : 'text-[var(--app-ink-subtle)]'}`} />
                <input
                  type="email"
                  required
                  autoComplete="email"
                  placeholder="you@example.com"
                  value={email}
                  onFocus={() => setEmailFocused(true)}
                  onBlur={() => setEmailFocused(false)}
                  onChange={(e) => setEmail(e.target.value)}
                  className="flex-1 bg-transparent text-sm text-[var(--app-ink)] outline-none placeholder:text-[var(--app-ink-tertiary)]"
                />
              </div>
            </div>

            <div>
              <label className="mb-1.5 block text-xs font-medium text-[var(--app-ink-muted)]">
                {t('account.password')}
              </label>
              <div
                className={`flex items-center gap-2.5 rounded-xl border bg-[var(--app-surface-1)] px-3.5 py-2.5 transition-colors ${
                  pwFocused
                    ? 'border-[var(--app-transcript)] shadow-[0_0_0_3px_rgba(94,106,210,0.18)]'
                    : 'border-[var(--app-hairline)]'
                }`}
              >
                <Lock className={`h-4 w-4 transition-colors ${pwFocused ? 'text-[var(--app-transcript)]' : 'text-[var(--app-ink-subtle)]'}`} />
                <input
                  type={showPw ? 'text' : 'password'}
                  required
                  autoComplete="current-password"
                  placeholder="••••••••"
                  value={password}
                  onFocus={() => setPwFocused(true)}
                  onBlur={() => setPwFocused(false)}
                  onChange={(e) => setPassword(e.target.value)}
                  className="flex-1 bg-transparent text-sm text-[var(--app-ink)] outline-none placeholder:text-[var(--app-ink-tertiary)]"
                />
                <button
                  type="button"
                  onClick={() => setShowPw(!showPw)}
                  aria-label="toggle password visibility"
                  className="text-[var(--app-ink-subtle)] hover:text-[var(--app-ink)] transition-colors"
                >
                  {showPw ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              </div>
            </div>

            {error && (
              <motion.div
                initial={{ opacity: 0, y: -4 }}
                animate={{ opacity: 1, y: 0 }}
                className="flex items-start gap-2 rounded-lg border border-[var(--app-error)]/40 bg-[var(--app-error)]/10 p-3 text-xs text-[var(--app-error)]"
              >
                <AlertCircle className="h-3.5 w-3.5 mt-0.5 flex-shrink-0" />
                <span>{error}</span>
              </motion.div>
            )}

            <button
              type="submit"
              disabled={busy}
              className="mt-2 w-full rounded-xl bg-[var(--app-summary)] text-[var(--app-canvas)] py-2.5 text-sm font-medium hover:opacity-90 transition-opacity disabled:opacity-40 disabled:cursor-not-allowed inline-flex items-center justify-center gap-2"
            >
              {busy && <Loader2 className="h-4 w-4 animate-spin" />}
              {busy ? t('login_page.logging_in') : t('account.login')}
            </button>
          </form>

          <p className="mt-6 text-center text-sm text-[var(--app-ink-subtle)]">
            {t('account.no_account')}{' '}
            <Link href="/register" className="text-[var(--app-transcript-hover)] hover:underline">
              {t('account.register')}
            </Link>
          </p>
        </motion.div>
      </main>
    </div>
  );
}

function Bullet({ label }: { label: string }) {
  return (
    <li className="flex items-start gap-2.5 text-sm text-[var(--app-ink-muted)]">
      <span className="mt-1.5 inline-block w-1.5 h-1.5 rounded-full bg-[var(--app-summary)] flex-shrink-0" />
      <span>{label}</span>
    </li>
  );
}
