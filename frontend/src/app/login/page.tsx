'use client';
import { APP_VERSION_SHORT } from '@/lib/version';

import React, { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { motion } from 'framer-motion';
import {
  ArrowLeft, Mail, Lock, Eye, EyeOff, Sparkles, Shield,
  Headphones, Mic, ChevronRight, AlertCircle, Loader2
} from 'lucide-react';
import { useTranslation } from '@/i18n';
import { STORAGE_LAST_EMAIL, useAuth } from '@/contexts/AuthContext';
import { BrandShield } from '@/components/BrandShield';

export default function LoginPage() {
  const { t } = useTranslation();
  const router = useRouter();
  const { login, user, loading, lastEmail } = useAuth();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');

  // v0.7.0+: 上次登录邮箱预填 (写于 login 成功后, 读于 mount 时)
  useEffect(() => {
    if (typeof window === 'undefined') return;
    const rememberedEmail = lastEmail || window.localStorage.getItem(STORAGE_LAST_EMAIL);
    if (rememberedEmail) setEmail(current => current || rememberedEmail);
  }, [lastEmail]);
  const [showPw, setShowPw] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [emailFocused, setEmailFocused] = useState(false);
  const [pwFocused, setPwFocused] = useState(false);

  // 已登录自动跳转
  useEffect(() => {
    if (!loading && user) router.replace('/');
  }, [user, loading, router]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (busy) return;
    setError(null);
    if (!email || !password) {
      setError('请填写邮箱和密码');
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
    }
    else setError(r.error ?? t('errors.generic'));
  }

  return (
    <div className="min-h-screen grid lg:grid-cols-2 bg-white dark:bg-neutral-950">
      {/* ─── Left brand panel ─────────────────────────────── */}
      <BrandPanel />

      {/* ─── Right form panel ──────────────────────────────── */}
      <div className="relative flex flex-col p-6 sm:p-10">
        {/* Top-right: back to home + lang */}
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
            className="w-full max-w-[400px]"
          >
            {/* Mobile-only brand */}
            <div className="mb-8 flex items-center gap-2.5 lg:hidden">
              <BrandShield size={32} />
              <span className="text-[15px] font-semibold tracking-tight text-neutral-900 dark:text-neutral-50">
                离线会记
              </span>
            </div>

            <h1 className="text-[28px] font-semibold tracking-tight text-neutral-900 dark:text-neutral-50">
              {t('account.login_title')}
            </h1>
            <p className="mt-1.5 text-[13.5px] text-neutral-500 dark:text-neutral-400">
              登录以同步你的会议记录与个人热词库
            </p>

            <form onSubmit={handleSubmit} className="mt-8 space-y-4">
              {/* Email */}
              <div>
                <label className="mb-1.5 block text-[12px] font-medium text-neutral-700 dark:text-neutral-300">
                  {t('account.email')}
                </label>
                <div
                  className={`group flex h-11 items-center gap-2.5 rounded-lg border bg-white px-3 transition-all dark:bg-neutral-900 ${
                    emailFocused
                      ? 'border-blue-500 ring-2 ring-blue-500/20 dark:border-blue-400'
                      : 'border-neutral-300 dark:border-neutral-700'
                  }`}
                >
                  <Mail className={`h-4 w-4 transition-colors ${emailFocused ? 'text-blue-500' : 'text-neutral-400'}`} />
                  <input
                    type="email"
                    required
                    autoComplete="email"
                    placeholder="you@example.com"
                    value={email}
                    onFocus={() => setEmailFocused(true)}
                    onBlur={() => setEmailFocused(false)}
                    onChange={(e) => setEmail(e.target.value)}
                    className="flex-1 bg-transparent text-[14px] text-neutral-900 outline-none placeholder:text-neutral-400 dark:text-neutral-100"
                  />
                </div>
              </div>

              {/* Password */}
              <div>
                <label className="mb-1.5 block text-[12px] font-medium text-neutral-700 dark:text-neutral-300">
                  {t('account.password')}
                </label>
                <div
                  className={`group flex h-11 items-center gap-2.5 rounded-lg border bg-white px-3 transition-all dark:bg-neutral-900 ${
                    pwFocused
                      ? 'border-blue-500 ring-2 ring-blue-500/20 dark:border-blue-400'
                      : 'border-neutral-300 dark:border-neutral-700'
                  }`}
                >
                  <Lock className={`h-4 w-4 transition-colors ${pwFocused ? 'text-blue-500' : 'text-neutral-400'}`} />
                  <input
                    type={showPw ? 'text' : 'password'}
                    required
                    autoComplete="current-password"
                    placeholder="••••••••"
                    value={password}
                    onFocus={() => setPwFocused(true)}
                    onBlur={() => setPwFocused(false)}
                    onChange={(e) => setPassword(e.target.value)}
                    className="flex-1 bg-transparent text-[14px] text-neutral-900 outline-none placeholder:text-neutral-400 dark:text-neutral-100"
                  />
                  <button
                    type="button"
                    onClick={() => setShowPw(!showPw)}
                    aria-label={showPw ? '隐藏密码' : '显示密码'}
                    className="text-neutral-400 hover:text-neutral-600"
                  >
                    {showPw ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  </button>
                </div>
              </div>

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
                className="group flex h-11 w-full items-center justify-center gap-2 rounded-lg bg-blue-600 text-[14px] font-medium text-white transition-all hover:bg-blue-700 hover:shadow-md hover:shadow-blue-600/20 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {busy ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    正在登录...
                  </>
                ) : (
                  <>
                    {t('account.login')}
                    <ChevronRight className="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
                  </>
                )}
              </button>
            </form>

            {/* Switch to register */}
            <div className="mt-6 rounded-lg border border-neutral-200 bg-neutral-50 px-4 py-3 text-[13px] text-neutral-700 dark:border-neutral-800 dark:bg-neutral-900/50 dark:text-neutral-300">
              <div className="flex items-center justify-between gap-3">
                <span>{t('account.no_account')}</span>
                <Link
                  href="/register"
                  className="inline-flex items-center gap-1 rounded-md bg-white px-3 py-1 text-[12.5px] font-medium text-blue-600 shadow-sm transition-colors hover:bg-blue-50 dark:bg-neutral-800 dark:text-blue-400 dark:hover:bg-neutral-700"
                >
                  {t('account.register')}
                  <ChevronRight className="h-3 w-3" />
                </Link>
              </div>
            </div>

            {/* Trust footer */}
            <p className="mt-8 text-center text-[11px] text-neutral-400 dark:text-neutral-500">
              登录即代表你同意本机的本地数据存储 · 不会上传云端
            </p>
          </motion.div>
        </div>
      </div>
    </div>
  );
}

function BrandPanel() {
  return (
    <div className="relative hidden flex-col justify-between overflow-hidden bg-neutral-950 p-10 text-white lg:flex">
      {/* Background pattern: gentle radial gradient + grid */}
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
          <span className="text-[17px] font-semibold tracking-tight text-white">离线会记</span>
          <span className="rounded border border-white/20 px-1.5 py-px font-mono text-[10px] uppercase tracking-wider text-white/60">
            {APP_VERSION_SHORT}
          </span>
        </div>
      </div>

      <div className="relative">
        <h2 className="text-[34px] font-semibold leading-tight tracking-tight">
          欢迎回来
          <br />
          <span className="bg-gradient-to-r from-teal-300 via-cyan-300 to-emerald-300 bg-clip-text text-transparent">
            继续你的会议纪要
          </span>
        </h2>
        <p className="mt-4 max-w-md text-[14px] leading-relaxed text-white/70">
          本地 AI 转录 · 全程离线 · 数据不上传云端
        </p>

        <div className="mt-8 grid grid-cols-2 gap-3 max-w-md">
          <FeatureItem icon={<Shield className="h-3.5 w-3.5" />} label="端到端本地存储" />
          <FeatureItem icon={<Mic className="h-3.5 w-3.5" />} label="实时转录 + AI 纪要" />
          <FeatureItem icon={<Headphones className="h-3.5 w-3.5" />} label="多模型离线引擎" />
          <FeatureItem icon={<Sparkles className="h-3.5 w-3.5" />} label="¥88 永久买断" />
        </div>
      </div>

      <div className="relative text-[11px] text-white/40">
        © {new Date().getFullYear()} 离线会记 · 本地 AI 会议转录
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
