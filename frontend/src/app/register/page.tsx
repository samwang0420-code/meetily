'use client';

import React, { useState, useEffect, useMemo } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { motion } from 'framer-motion';
import {
  ArrowLeft, Mail, Lock, Eye, EyeOff, User as UserIcon,
  Shield, Headphones, Mic, ChevronRight, AlertCircle,
  CheckCircle2, Loader2, Sparkles
} from 'lucide-react';
import { useTranslation } from '@/i18n';
import { useAuth } from '@/contexts/AuthContext';
import { BrandShield } from '@/components/BrandShield';

function pwStrength(pw: string): { score: number; labelKey: string; color: string } {
  let score = 0;
  if (pw.length >= 6) score++;
  if (pw.length >= 10) score++;
  if (/[A-Z]/.test(pw)) score++;
  if (/[0-9]/.test(pw)) score++;
  if (/[^A-Za-z0-9]/.test(pw)) score++;
  const map = [
    { labelKey: 'register_page.strength_too_short', color: 'bg-red-500' },
    { labelKey: 'register_page.strength_weak', color: 'bg-orange-500' },
    { labelKey: 'register_page.strength_fair', color: 'bg-yellow-500' },
    { labelKey: 'register_page.strength_good', color: 'bg-blue-500' },
    { labelKey: 'register_page.strength_strong', color: 'bg-emerald-500' },
    { labelKey: 'register_page.strength_very_strong', color: 'bg-emerald-600' },
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
      setError(t('register_page.required_error'));
      return;
    }
    if (password !== confirm) {
      setError(t('account.password_mismatch'));
      return;
    }
    if (password.length < 6) {
      setError(t('account.weak_password'));
      return;
    }
    setBusy(true);
    const r = await register(email.trim(), password, displayName.trim() || undefined);
    setBusy(false);
    if (r.ok) router.push('/');
    else setError(r.error ?? t('errors.generic'));
  }

  return (
    <div className="min-h-screen grid lg:grid-cols-2 bg-white dark:bg-neutral-950">
      <BrandPanel />

      <div className="relative flex flex-col p-6 sm:p-10">
        <div className="flex items-center justify-between">
          <Link
            href="/"
            className="group inline-flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-[13px] text-neutral-500 transition-colors hover:bg-neutral-100 hover:text-neutral-900 dark:text-neutral-400 dark:hover:bg-neutral-800 dark:hover:text-neutral-100"
          >
            <ArrowLeft className="h-3.5 w-3.5 transition-transform group-hover:-translate-x-0.5" />
            {t('account.back_to_home')}
          </Link>
        </div>

        <div className="flex flex-1 items-center justify-center">
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, ease: 'easeOut' }}
            className="w-full max-w-[440px]"
          >
            <div className="mb-8 flex items-center gap-2.5 lg:hidden">
              <BrandShield size={32} />
              <span className="text-[15px] font-semibold tracking-tight text-neutral-900 dark:text-neutral-50">{t('app.name')}</span>
            </div>

            <h1 className="text-[28px] font-semibold tracking-tight text-neutral-900 dark:text-neutral-50">
              {t('account.register_title')}
            </h1>
            <p className="mt-1.5 text-[13.5px] text-neutral-500 dark:text-neutral-400">
              {t('register_page.subtitle')}
            </p>

            <form onSubmit={handleSubmit} className="mt-7 space-y-3.5">
              {/* Email */}
              <Field
                label={t('account.email')}
                focused={focusedField === 'email'}
                icon={<Mail className="h-4 w-4" />}
              >
                <input
                  type="email"
                  required
                  autoComplete="email"
                  placeholder="you@example.com"
                  value={email}
                  onFocus={() => setFocusedField('email')}
                  onBlur={() => setFocusedField(null)}
                  onChange={(e) => setEmail(e.target.value)}
                  className="flex-1 bg-transparent text-[14px] text-neutral-900 outline-none placeholder:text-neutral-400 dark:text-neutral-100"
                />
              </Field>

              {/* Display Name (optional) */}
              <Field
                label={
                  <span>
                    {t('account.display_name')}{' '}
                    <span className="text-neutral-400">{t('register_page.optional')}</span>
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
                  className="flex-1 bg-transparent text-[14px] text-neutral-900 outline-none placeholder:text-neutral-400 dark:text-neutral-100"
                />
              </Field>

              {/* Password */}
              <div>
                <Field
                  label={t('account.password')}
                  focused={focusedField === 'password'}
                  icon={<Lock className="h-4 w-4" />}
                  right={
                    <button
                      type="button"
                      onClick={() => setShowPw(!showPw)}
                      aria-label={showPw ? t('register_page.hide_password') : t('register_page.show_password')}
                      className="text-neutral-400 hover:text-neutral-600"
                    >
                      {showPw ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                    </button>
                  }
                >
                  <input
                    type={showPw ? 'text' : 'password'}
                    required
                    minLength={6}
                    autoComplete="new-password"
                    placeholder={t('register_page.password_hint')}
                    value={password}
                    onFocus={() => setFocusedField('password')}
                    onBlur={() => setFocusedField(null)}
                    onChange={(e) => setPassword(e.target.value)}
                    className="flex-1 bg-transparent text-[14px] text-neutral-900 outline-none placeholder:text-neutral-400 dark:text-neutral-100"
                  />
                </Field>

                {/* Strength meter */}
                {password.length > 0 && (
                  <div className="mt-1.5 flex items-center gap-2">
                    <div className="flex flex-1 gap-1">
                      {[0, 1, 2, 3, 4].map((i) => (
                        <div
                          key={i}
                          className={`h-1 flex-1 rounded-full transition-colors ${
                            i <= strength.score ? strength.color : 'bg-neutral-200 dark:bg-neutral-800'
                          }`}
                        />
                      ))}
                    </div>
                    <span className={`text-[10.5px] font-medium ${
                      pwTooShort ? 'text-red-500' : 'text-neutral-500'
                    }`}>
                      {pwTooShort ? t('register_page.password_hint') : t(strength.labelKey)}
                    </span>
                  </div>
                )}
              </div>

              {/* Confirm password */}
              <Field
                label={t('account.confirm_password')}
                focused={focusedField === 'confirm'}
                icon={<Lock className="h-4 w-4" />}
                right={
                  confirm.length > 0 && !matchError ? (
                    <CheckCircle2 className="h-4 w-4 text-emerald-500" />
                  ) : matchError ? (
                    <AlertCircle className="h-4 w-4 text-red-500" />
                  ) : null
                }
                hasError={!!matchError}
              >
                <input
                  type={showPw ? 'text' : 'password'}
                  required
                  minLength={6}
                  autoComplete="new-password"
                  placeholder={t('register_page.confirm_hint')}
                  value={confirm}
                  onFocus={() => setFocusedField('confirm')}
                  onBlur={() => setFocusedField(null)}
                  onChange={(e) => setConfirm(e.target.value)}
                  className="flex-1 bg-transparent text-[14px] text-neutral-900 outline-none placeholder:text-neutral-400 dark:text-neutral-100"
                />
              </Field>

              {/* Error */}
              {error && (
                <motion.div
                  initial={{ opacity: 0, y: -4 }}
                  animate={{ opacity: 1, y: 0 }}
                  className="flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-[12.5px] text-red-700 dark:border-red-800/60 dark:bg-red-900/30 dark:text-red-300"
                >
                  <AlertCircle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                  <span>{error}</span>
                </motion.div>
              )}

              {/* Submit */}
              <button
                type="submit"
                disabled={busy}
                className="group mt-2 flex h-11 w-full items-center justify-center gap-2 rounded-lg bg-blue-600 text-[14px] font-medium text-white transition-all hover:bg-blue-700 hover:shadow-md hover:shadow-blue-600/20 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {busy ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {t('register_page.creating')}
                  </>
                ) : (
                  <>
                    {t('account.register')}
                    <ChevronRight className="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
                  </>
                )}
              </button>
            </form>

            {/* v0.6.10+: 注册即同意条款 (合规底线) */}
            <p className="mt-3 text-[11px] text-neutral-500 text-center">
              {t('register_page.consent_prefix')}{' '}
              <Link href="/legal/terms" target="_blank" className="text-blue-600 hover:underline">{t('register_page.terms')}</Link>
              {' '}{t('register_page.consent_and')}{' '}
              <Link href="/legal/privacy" target="_blank" className="text-blue-600 hover:underline">{t('register_page.privacy')}</Link>
            </p>

            <div className="mt-6 rounded-lg border border-neutral-200 bg-neutral-50 px-4 py-3 text-[13px] text-neutral-700 dark:border-neutral-800 dark:bg-neutral-900/50 dark:text-neutral-300">
              <div className="flex items-center justify-between gap-3">
                <span>{t('account.has_account')}</span>
                <Link
                  href="/login"
                  className="inline-flex items-center gap-1 rounded-md bg-white px-3 py-1 text-[12.5px] font-medium text-blue-600 shadow-sm transition-colors hover:bg-blue-50 dark:bg-neutral-800 dark:text-blue-400 dark:hover:bg-neutral-700"
                >
                  {t('account.login')}
                  <ChevronRight className="h-3 w-3" />
                </Link>
              </div>
            </div>

            <p className="mt-6 text-center text-[11px] text-neutral-400 dark:text-neutral-500">
              {t('register_page.local_consent')}
            </p>
          </motion.div>
        </div>
      </div>
    </div>
  );
}

function Field({
  label, focused, icon, children, right, hasError
}: {
  label: React.ReactNode
  focused: boolean
  icon: React.ReactNode
  children: React.ReactNode
  right?: React.ReactNode
  hasError?: boolean
}) {
  return (
    <div>
      <label className="mb-1.5 block text-[12px] font-medium text-neutral-700 dark:text-neutral-300">
        {label}
      </label>
      <div
        className={`flex h-11 items-center gap-2.5 rounded-lg border bg-white px-3 transition-all dark:bg-neutral-900 ${
          hasError
            ? 'border-red-400 ring-2 ring-red-400/20'
            : focused
              ? 'border-blue-500 ring-2 ring-blue-500/20 dark:border-blue-400'
              : 'border-neutral-300 dark:border-neutral-700'
        }`}
      >
        <span className={`transition-colors ${
          hasError ? 'text-red-500' : focused ? 'text-blue-500' : 'text-neutral-400'
        }`}>
          {icon}
        </span>
        {children}
        {right}
      </div>
    </div>
  )
}

function BrandPanel() {
  const { t } = useTranslation();
  return (
    <div className="relative hidden flex-col justify-between overflow-hidden bg-neutral-950 p-10 text-white lg:flex">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_30%_20%,rgba(19,168,158,0.18),transparent_60%),radial-gradient(circle_at_80%_80%,rgba(11,37,69,0.6),transparent_70%)]" />
      <div
        className="pointer-events-none absolute inset-0 opacity-30"
        style={{
          backgroundImage:
            'linear-gradient(rgba(255,255,255,0.04) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,0.04) 1px, transparent 1px)',
          backgroundSize: '32px 32px',
        }}
      />

      <div className="relative flex items-center gap-3">
        <BrandShield size={36} />
        <div className="flex items-baseline gap-2">
          <span className="text-[17px] font-semibold tracking-tight text-white">{t('app.name')}</span>
          <span className="rounded border border-white/20 px-1.5 py-px font-mono text-[10px] uppercase tracking-wider text-white/60">
            v0.8.6
          </span>
        </div>
      </div>

      <div className="relative">
        <h2 className="text-[34px] font-semibold leading-tight tracking-tight">
          {t('register_page.hero_title')}
          <br />
          <span className="bg-gradient-to-r from-teal-300 via-cyan-300 to-emerald-300 bg-clip-text text-transparent">
            {t('register_page.hero_highlight')}
          </span>
        </h2>
        <p className="mt-4 max-w-md text-[14px] leading-relaxed text-white/70">
          {t('register_page.hero_desc')}
        </p>

        <div className="mt-8 grid grid-cols-2 gap-3 max-w-md">
          <FeatureItem icon={<Shield className="h-3.5 w-3.5" />} label={t('register_page.feature_local')} />
          <FeatureItem icon={<Mic className="h-3.5 w-3.5" />} label={t('register_page.feature_minutes')} />
          <FeatureItem icon={<Headphones className="h-3.5 w-3.5" />} label={t('register_page.feature_models')} />
          <FeatureItem icon={<Sparkles className="h-3.5 w-3.5" />} label={t('register_page.feature_buyout')} />
        </div>
      </div>

      <div className="relative text-[11px] text-white/40">
        © {new Date().getFullYear()} {t('register_page.copyright')}
      </div>
    </div>
  );
}

function FeatureItem({ icon, label }: { icon: React.ReactNode; label: string }) {
  return (
    <div className="flex items-center gap-2 rounded-md border border-white/10 bg-white/5 px-3 py-2 backdrop-blur-sm">
      <span className="text-teal-400">{icon}</span>
      <span className="text-[12px] text-white/80">{label}</span>
    </div>
  );
}
