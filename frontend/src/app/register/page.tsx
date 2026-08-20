'use client';

import React, { useState, useEffect, useMemo } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { motion } from 'framer-motion';
import { Mail, Lock, Eye, EyeOff, User as UserIcon, AlertCircle, Loader2, ArrowLeft } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { useAuth } from '@/contexts/AuthContext';
import { BrandShield } from '@/components/BrandShield';

// §146: 言镜 AI 官网重做 — register 页
// 视觉与 pricing 一致: --app-canvas/surface/transcript/summary token
// 表单逻辑保留 (useAuth.register),减 lucide icon 12→3,聚焦态 --app-transcript 蓝紫

function pwStrength(pw: string): { score: number; labelKey: string; color: string } {
  let score = 0;
  if (pw.length >= 6) score++;
  if (pw.length >= 10) score++;
  if (/[A-Z]/.test(pw)) score++;
  if (/[0-9]/.test(pw)) score++;
  if (/[^A-Za-z0-9]/.test(pw)) score++;
  const map = [
    { labelKey: 'register_page.strength_too_short', color: 'bg-[var(--app-error)]' },
    { labelKey: 'register_page.strength_weak', color: 'bg-[var(--app-warning)]' },
    { labelKey: 'register_page.strength_fair', color: 'bg-[var(--app-summary)]' },
    { labelKey: 'register_page.strength_good', color: 'bg-[var(--app-transcript)]' },
    { labelKey: 'register_page.strength_strong', color: 'bg-[var(--app-success)]' },
    { labelKey: 'register_page.strength_very_strong', color: 'bg-[var(--app-success)]' },
  ];
  const safe = Math.min(score, map.length - 1);
  return { score: safe, labelKey: map[safe].labelKey, color: map[safe].color };
}

export default function RegisterPage() {
  const { t } = useTranslation();
  const router = useRouter();
  const { register, user, loading } = useAuth();
  const [email, setEmail] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [showPw, setShowPw] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [focusedField, setFocusedField] = useState<string | null>(null);

  useEffect(() => {
    if (!loading && user) router.replace('/');
  }, [user, loading, router]);

  const strength = useMemo(() => pwStrength(password), [password]);
  const pwTooShort = password.length > 0 && password.length < 6;
  const matchError = confirm.length > 0 && password !== confirm;

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (busy) return;
    setError(null);
    if (!email || !password || !confirm) {
      setError(t('register_page.error_empty'));
      return;
    }
    if (pwTooShort) {
      setError(t('register_page.error_pw_too_short'));
      return;
    }
    if (password !== confirm) {
      setError(t('register_page.error_pw_mismatch'));
      return;
    }
    setBusy(true);
    try {
      const ok = await register(email, password, displayName || undefined);
      if (ok) {
        router.replace('/');
      } else {
        setError(t('register_page.error_generic'));
      }
    } catch (e: any) {
      setError(e?.message || t('register_page.error_generic'));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="min-h-screen bg-[var(--app-canvas)] text-[var(--app-ink)] grid lg:grid-cols-[1.1fr_1fr]">
      {/* ────── 左: 品牌面板 ────── */}
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
            {t('register_page.hero_title')}
            <br />
            <span className="bg-gradient-to-r from-[var(--app-transcript-hover)] to-[var(--app-summary)] bg-clip-text text-transparent">
              {t('register_page.hero_highlight')}
            </span>
          </h2>
          <p className="text-sm text-[var(--app-ink-muted)] max-w-sm leading-relaxed">
            {t('register_page.hero_desc')}
          </p>

          {/* 3 bullet 而非 2x2 grid */}
          <ul className="mt-10 space-y-3 max-w-sm">
            <Bullet label={t('register_page.feature_local')} />
            <Bullet label={t('register_page.feature_models')} />
            <Bullet label={t('register_page.feature_buyout')} />
          </ul>
        </div>

        <p className="relative text-[11px] text-[var(--app-ink-subtle)]">
          © {new Date().getFullYear()} {t('register_page.copyright')}
        </p>
      </aside>

      {/* ────── 右: 表单 ────── */}
      <main className="flex items-center justify-center p-6 sm:p-10">
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4, ease: 'easeOut' }}
          className="w-full max-w-[420px]"
        >
          {/* mobile-only logo */}
          <div className="mb-8 flex items-center gap-2.5 lg:hidden">
            <BrandShield size={28} />
            <span className="text-[15px] font-semibold tracking-tight">{t('app.name')}</span>
          </div>

          <h1 className="text-[clamp(1.6rem,3vw,2rem)] font-semibold tracking-tight">
            {t('account.register_title')}
          </h1>
          <p className="mt-1.5 text-sm text-[var(--app-ink-subtle)]">
            {t('register_page.subtitle')}
          </p>

          <form onSubmit={handleSubmit} className="mt-7 space-y-3">
            {/* Email */}
            <Field label={t('account.email')} focused={focusedField === 'email'} icon={<Mail className="h-4 w-4" />}>
              <input
                type="email"
                required
                autoComplete="email"
                placeholder="you@example.com"
                value={email}
                onFocus={() => setFocusedField('email')}
                onBlur={() => setFocusedField(null)}
                onChange={(e) => setEmail(e.target.value)}
                className="flex-1 bg-transparent text-sm text-[var(--app-ink)] outline-none placeholder:text-[var(--app-ink-tertiary)]"
              />
            </Field>

            {/* Display name (optional) */}
            <Field
              label={
                <span>
                  {t('account.display_name')}{' '}
                  <span className="text-[var(--app-ink-tertiary)] text-xs">{t('register_page.optional')}</span>
                </span>
              }
              focused={focusedField === 'name'}
              icon={<UserIcon className="h-4 w-4" />}
            >
              <input
                type="text"
                placeholder={t('register_page.nickname')}
                value={displayName}
                onFocus={() => setFocusedField('name')}
                onBlur={() => setFocusedField(null)}
                onChange={(e) => setDisplayName(e.target.value)}
                className="flex-1 bg-transparent text-sm text-[var(--app-ink)] outline-none placeholder:text-[var(--app-ink-tertiary)]"
              />
            </Field>

            {/* Password */}
            <Field label={t('account.password')} focused={focusedField === 'password'} icon={<Lock className="h-4 w-4" />}>
              <input
                type={showPw ? 'text' : 'password'}
                required
                autoComplete="new-password"
                placeholder="••••••••"
                value={password}
                onFocus={() => setFocusedField('password')}
                onBlur={() => setFocusedField(null)}
                onChange={(e) => setPassword(e.target.value)}
                className="flex-1 bg-transparent text-sm text-[var(--app-ink)] outline-none placeholder:text-[var(--app-ink-tertiary)]"
              />
              <button
                type="button"
                onClick={() => setShowPw(!showPw)}
                className="text-[var(--app-ink-subtle)] hover:text-[var(--app-ink)] transition-colors"
                aria-label="toggle password visibility"
              >
                {showPw ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
              </button>
            </Field>

            {/* Password strength meter */}
            {password.length > 0 && (
              <div className="flex items-center gap-2 pt-1">
                <div className="flex flex-1 gap-1">
                  {[0, 1, 2, 3, 4].map((i) => (
                    <div
                      key={i}
                      className={`h-1 flex-1 rounded-full transition-colors ${
                        i <= strength.score ? strength.color : 'bg-[var(--app-surface-3)]'
                      }`}
                    />
                  ))}
                </div>
                <span className="text-[11px] text-[var(--app-ink-subtle)]">{t(strength.labelKey)}</span>
              </div>
            )}

            {/* Confirm password */}
            <Field
              label={t('register_page.confirm_password')}
              focused={focusedField === 'confirm'}
              icon={<Lock className="h-4 w-4" />}
              error={matchError ? t('register_page.error_pw_mismatch') : null}
            >
              <input
                type={showPw ? 'text' : 'password'}
                required
                autoComplete="new-password"
                placeholder="••••••••"
                value={confirm}
                onFocus={() => setFocusedField('confirm')}
                onBlur={() => setFocusedField(null)}
                onChange={(e) => setConfirm(e.target.value)}
                className="flex-1 bg-transparent text-sm text-[var(--app-ink)] outline-none placeholder:text-[var(--app-ink-tertiary)]"
              />
            </Field>

            {error && (
              <div className="flex items-start gap-2 rounded-lg border border-[var(--app-error)]/40 bg-[var(--app-error)]/10 p-3 text-xs text-[var(--app-error)]">
                <AlertCircle className="h-3.5 w-3.5 mt-0.5 flex-shrink-0" />
                <span>{error}</span>
              </div>
            )}

            <button
              type="submit"
              disabled={busy || pwTooShort || matchError}
              className="mt-2 w-full rounded-xl bg-[var(--app-summary)] text-[var(--app-canvas)] py-2.5 text-sm font-medium hover:opacity-90 transition-opacity disabled:opacity-40 disabled:cursor-not-allowed inline-flex items-center justify-center gap-2"
            >
              {busy && <Loader2 className="h-4 w-4 animate-spin" />}
              {busy ? t('account.registering') : t('account.register_button')}
            </button>
          </form>

          <p className="mt-6 text-center text-sm text-[var(--app-ink-subtle)]">
            {t('register_page.have_account')}{' '}
            <Link href="/login" className="text-[var(--app-transcript-hover)] hover:underline">
              {t('account.login')}
            </Link>
          </p>
        </motion.div>
      </main>
    </div>
  );
}

function Field({
  label,
  icon,
  focused,
  error,
  children,
}: {
  label: React.ReactNode;
  icon?: React.ReactNode;
  focused?: boolean;
  error?: string | null;
  children: React.ReactNode;
}) {
  return (
    <div>
      <label className="mb-1.5 block text-xs font-medium text-[var(--app-ink-muted)]">{label}</label>
      <div
        className={`flex items-center gap-2.5 rounded-xl border bg-[var(--app-surface-1)] px-3.5 py-2.5 transition-colors ${
          focused
            ? 'border-[var(--app-transcript)] shadow-[0_0_0_3px_rgba(94,106,210,0.18)]'
            : error
            ? 'border-[var(--app-error)]/60'
            : 'border-[var(--app-hairline)]'
        }`}
      >
        {icon && <span className="text-[var(--app-ink-subtle)]">{icon}</span>}
        {children}
      </div>
      {error && <p className="mt-1 text-[11px] text-[var(--app-error)]">{error}</p>}
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
