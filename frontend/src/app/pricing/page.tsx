'use client';

import React from 'react';
import Link from 'next/link';
import { motion } from 'framer-motion';
import { Check, X, Sparkles, Github, Download, ChevronDown } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { BrandShield } from '@/components/BrandShield';

// §145: 言镜 AI 官网重做 — pricing 页
// 减文字 (246→180) · 深色 hero · 同心环 logo · 三档极简卡 · 合并 FAQ
// 设计 token: --app-canvas / --app-surface-* / --app-transcript / --app-summary (globals.css 已定义)

export default function PricingPage() {
  const { t } = useTranslation();

  // 横向三档 — 仅保留必要信息
  const tiers = [
    {
      key: 'anonymous',
      title: t('pricing.tier_anonymous'),
      subtitle: t('pricing.tier_anonymous_subtitle'),
      price: t('pricing.tier_anonymous_price_label'),
      priceSub: t('pricing.tier_anonymous_price_sub'),
      features: t.list('pricing.tier_anonymous_features'),
      cta: t('pricing.tier_anonymous_cta'),
      ctaHref: '/',
      accent: false,
      elevated: false,
    },
    {
      key: 'free',
      title: t('pricing.tier_free'),
      subtitle: t('pricing.tier_free_subtitle'),
      price: t('pricing.tier_free_price_label'),
      priceSub: t('pricing.tier_free_price_sub'),
      features: t.list('pricing.tier_free_features'),
      cta: t('pricing.tier_free_cta'),
      ctaHref: '/register',
      accent: false,
      elevated: false,
    },
    {
      key: 'pro',
      title: t('pricing.tier_pro'),
      subtitle: t('pricing.tier_pro_subtitle'),
      price: t('pricing.tier_pro_price_label'),
      priceSub: t('pricing.tier_pro_price_sub'),
      features: t.list('pricing.tier_pro_features'),
      cta: t('pricing.tier_pro_cta'),
      ctaHref: 'mailto:lisangjie@icloudsend.com?subject=Pro%20%E4%B9%B0%E6%96%AD%E5%92%A8%E8%AF%A2',
      ctaRedeem: t('pricing.tier_pro_cta_redeem'),
      ctaRedeemHref: '/redeem',
      accent: true,
      elevated: true,
    },
  ];

  const trustItems = [
    { label: t('pricing.trust_local'), dot: 'var(--app-success)' },
    { label: t('pricing.trust_open_source'), dot: 'var(--app-transcript)' },
    { label: t('pricing.trust_github'), dot: 'var(--app-summary)', icon: Github, href: 'https://github.com/samwang0420-code/meetily' },
    { label: t('pricing.trust_version'), dot: 'var(--app-ink-subtle)' },
  ];

  const whyLocal = [
    { title: t('pricing.why_local_1_title'), body: t('pricing.why_local_1_body') },
    { title: t('pricing.why_local_2_title'), body: t('pricing.why_local_2_body') },
    { title: t('pricing.why_local_3_title'), body: t('pricing.why_local_3_body') },
  ];

  // 合并 FAQ — 只留最常问的 5 个,删除冗余
  const faq = [
    { q: t('pricing.faq.q_subscription'), a: t('pricing.faq.a_subscription') },
    { q: t('pricing.faq.q_anonymous_rules'), a: t('pricing.faq_q_anonymous_rules').length ? t('pricing.faq.q_anonymous_rules') : t('pricing.faq.a_anonymous') },
    { q: t('pricing.faq.q_free_quota'), a: t('pricing.faq_a_free_quota') },
    { q: t('pricing.faq.q_activate'), a: t('pricing.faq.a_activate') },
    { q: t('pricing.faq.q_refund'), a: t('pricing.faq.a_refund') },
    { q: t('pricing.faq.q_privacy'), a: t('pricing.faq.a_privacy') },
  ];

  return (
    <div className="min-h-screen bg-[var(--app-canvas)] text-[var(--app-ink)]">
      {/* ────── HERO ────── */}
      <section className="relative overflow-hidden">
        {/* 同心圆背景渐变 */}
        <div
          aria-hidden
          className="absolute inset-0 opacity-40"
          style={{
            background: 'radial-gradient(ellipse at 50% 0%, rgba(94,106,210,0.18) 0%, transparent 60%)',
          }}
        />
        <div className="relative mx-auto max-w-5xl px-6 pt-16 pb-12 sm:pt-20 sm:pb-16 text-center">
          <motion.div
            initial={{ opacity: 0, y: -8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
            className="inline-flex items-center gap-2 rounded-full border border-[var(--app-hairline-strong)] bg-[var(--app-surface-2)] px-3 py-1 text-xs text-[var(--app-ink-muted)] mb-6"
          >
            <Sparkles className="w-3 h-3 text-[var(--app-summary)]" />
            <span>{t('pricing.hero_eyebrow')}</span>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{ duration: 0.6, delay: 0.05 }}
            className="mx-auto mb-6"
          >
            <BrandShield size={72} className="mx-auto" />
          </motion.div>

          <motion.h1
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.1 }}
            className="text-[clamp(2rem,5vw,3.25rem)] font-semibold tracking-tight leading-[1.15] mb-3"
          >
            {t('pricing.hero_title')}
          </motion.h1>

          <motion.p
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.15 }}
            className="text-[clamp(0.95rem,1.6vw,1.1rem)] text-[var(--app-ink-muted)] max-w-xl mx-auto leading-relaxed mb-8"
          >
            {t('pricing.hero_subtitle')}
          </motion.p>

          {/* 信任行 */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.5, delay: 0.2 }}
            className="flex flex-wrap items-center justify-center gap-x-5 gap-y-2 text-xs text-[var(--app-ink-subtle)]"
          >
            {trustItems.map((item, i) => {
              const Icon = item.icon;
              const inner = (
                <span className="inline-flex items-center gap-1.5">
                  <span
                    className="inline-block w-1.5 h-1.5 rounded-full"
                    style={{ backgroundColor: item.dot }}
                  />
                  <span>{item.label}</span>
                  {Icon && <Icon className="w-3 h-3" />}
                </span>
              );
              return item.href ? (
                <a
                  key={i}
                  href={item.href}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="hover:text-[var(--app-ink)] transition-colors"
                >
                  {inner}
                </a>
              ) : (
                <span key={i}>{inner}</span>
              );
            })}
          </motion.div>
        </div>
      </section>

      {/* ────── 隐私一行 ────── */}
      <div className="border-y border-[var(--app-hairline)] bg-[var(--app-surface-1)]">
        <p className="mx-auto max-w-5xl px-6 py-3 text-center text-sm text-[var(--app-ink-muted)]">
          <span className="inline-block w-1.5 h-1.5 rounded-full bg-[var(--app-success)] mr-2" />
          {t('pricing.privacy_one_liner')}
        </p>
      </div>

      {/* ────── 三档定价 ────── */}
      <section className="mx-auto max-w-5xl px-6 pt-16 pb-12">
        <div className="grid md:grid-cols-3 gap-4">
          {tiers.map((tier, i) => (
            <motion.div
              key={tier.key}
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.4, delay: 0.05 * i }}
              className={[
                'relative rounded-2xl p-6 flex flex-col',
                'border',
                tier.elevated
                  ? 'border-[var(--app-summary)] bg-[var(--app-surface-2)] shadow-[0_0_0_1px_var(--app-summary),0_24px_48px_-12px_rgba(255,197,51,0.25)] md:-translate-y-2'
                  : 'border-[var(--app-hairline)] bg-[var(--app-surface-1)]',
              ].join(' ')}
            >
              {tier.elevated && (
                <div className="absolute -top-2.5 left-1/2 -translate-x-1/2 px-2.5 py-0.5 rounded-full bg-[var(--app-summary)] text-[var(--app-canvas)] text-[10px] font-semibold uppercase tracking-wider">
                  {t('pricing.recommended')}
                </div>
              )}

              <div className="mb-4">
                <h2 className="text-lg font-medium mb-1">{tier.title}</h2>
                <p className="text-xs text-[var(--app-ink-subtle)]">{tier.subtitle}</p>
              </div>

              <div className="mb-5">
                <div className="flex items-baseline gap-1">
                  <span className="text-4xl font-semibold tracking-tight">{tier.price}</span>
                </div>
                <p className="text-xs text-[var(--app-ink-subtle)] mt-1">{tier.priceSub}</p>
              </div>

              <ul className="space-y-2 mb-6 flex-1">
                {tier.features.map((line: string, idx: number) => {
                  const ok = line.startsWith('✓');
                  const dash = line.startsWith('—') || line.startsWith('-');
                  const text = line.replace(/^[✓—\-]\s*/, '');
                  return (
                    <li key={idx} className="flex items-start gap-2 text-sm">
                      {ok ? (
                        <Check className="w-4 h-4 mt-0.5 text-[var(--app-success)] flex-shrink-0" />
                      ) : dash ? (
                        <X className="w-4 h-4 mt-0.5 text-[var(--app-ink-tertiary)] flex-shrink-0" />
                      ) : (
                        <span className="w-4 h-4 mt-0.5 flex-shrink-0" />
                      )}
                      <span
                        className={
                          ok
                            ? 'text-[var(--app-ink-muted)]'
                            : dash
                            ? 'text-[var(--app-ink-tertiary)] line-through'
                            : 'text-[var(--app-ink)]'
                        }
                      >
                        {text}
                      </span>
                    </li>
                  );
                })}
              </ul>

              {tier.ctaRedeem ? (
                <div className="space-y-2">
                  <a
                    href={tier.ctaHref}
                    className="block w-full text-center px-4 py-2.5 rounded-xl bg-[var(--app-summary)] text-[var(--app-canvas)] text-sm font-medium hover:opacity-90 transition-opacity"
                  >
                    {tier.cta}
                  </a>
                  <Link
                    href={tier.ctaRedeemHref!}
                    className="block w-full text-center px-4 py-2.5 rounded-xl border border-[var(--app-hairline-strong)] bg-transparent text-[var(--app-ink-muted)] text-sm hover:bg-[var(--app-surface-3)] transition-colors"
                  >
                    {tier.ctaRedeem}
                  </Link>
                </div>
              ) : (
                <Link
                  href={tier.ctaHref}
                  className={[
                    'block w-full text-center px-4 py-2.5 rounded-xl text-sm font-medium transition-colors',
                    tier.accent
                      ? 'bg-[var(--app-summary)] text-[var(--app-canvas)] hover:opacity-90'
                      : 'border border-[var(--app-hairline-strong)] bg-transparent text-[var(--app-ink)] hover:bg-[var(--app-surface-3)]',
                  ].join(' ')}
                >
                  {tier.cta}
                </Link>
              )}
            </motion.div>
          ))}
        </div>
      </section>

      {/* ────── 为什么坚持本地 ────── */}
      <section className="mx-auto max-w-5xl px-6 py-12">
        <h2 className="text-center text-2xl font-semibold mb-2">{t('pricing.why_local_title')}</h2>
        <div className="grid md:grid-cols-3 gap-3 mt-8">
          {whyLocal.map((item, i) => (
            <div
              key={i}
              className="rounded-xl border border-[var(--app-hairline)] bg-[var(--app-surface-1)] p-5"
            >
              <h3 className="text-base font-medium mb-1.5">{item.title}</h3>
              <p className="text-sm text-[var(--app-ink-muted)] leading-relaxed">{item.body}</p>
            </div>
          ))}
        </div>
      </section>

      {/* ────── 对比表 (简化) ────── */}
      <section className="mx-auto max-w-5xl px-6 py-12">
        <h2 className="text-center text-2xl font-semibold mb-1">{t('pricing.feature_compare')}</h2>
        <p className="text-center text-sm text-[var(--app-ink-subtle)] mb-8">{t('pricing.compare_caption')}</p>
        <div className="rounded-xl border border-[var(--app-hairline)] bg-[var(--app-surface-1)] overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-[var(--app-hairline)] bg-[var(--app-surface-2)]">
                <th className="text-left py-3 px-4 font-medium text-[var(--app-ink-muted)]">
                  {t('pricing.table_feature')}
                </th>
                <th className="py-3 px-4 font-medium text-[var(--app-ink-muted)] w-28">
                  {t('pricing.table_free')}
                </th>
                <th className="py-3 px-4 font-medium text-[var(--app-summary)] w-28">
                  {t('pricing.table_pro')}
                </th>
              </tr>
            </thead>
            <tbody>
              {[
                { feat: t('pricing.feat_local_transcribe'), free: t('pricing.val_yes'), pro: t('pricing.val_yes') },
                { feat: t('pricing.feat_privacy'), free: t('pricing.val_yes'), pro: t('pricing.val_yes') },
                { feat: t('pricing.feat_local_summary'), free: t('pricing.val_yes'), pro: t('pricing.val_yes') },
                { feat: t('pricing.feat_monthly_meetings'), free: t('pricing.val_quota_5'), pro: t('pricing.val_unlimited') },
                { feat: t('pricing.feat_segments_per_meeting'), free: t('pricing.val_segments_100'), pro: t('pricing.val_unlimited') },
                { feat: t('pricing.feat_hotwords'), free: t('pricing.val_hotwords_basic'), pro: t('pricing.val_hotwords_all') },
                { feat: t('pricing.feat_speaker'), free: t('pricing.val_no'), pro: t('pricing.val_yes') },
                { feat: t('pricing.feat_batch'), free: t('pricing.val_no'), pro: t('pricing.val_yes') },
                { feat: t('pricing.feat_templates'), free: t('pricing.val_templates_basic'), pro: t('pricing.val_templates_all') },
                { feat: t('pricing.feat_priority'), free: t('pricing.val_no'), pro: t('pricing.val_yes') },
              ].map((row, i) => (
                <tr key={i} className="border-b border-[var(--app-hairline)] last:border-0">
                  <td className="py-3 px-4 text-[var(--app-ink-muted)]">{row.feat}</td>
                  <td className="py-3 px-4 text-center text-[var(--app-ink-muted)]">{row.free}</td>
                  <td className="py-3 px-4 text-center text-[var(--app-summary)] font-medium">{row.pro}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      {/* ────── FAQ ────── */}
      <section className="mx-auto max-w-3xl px-6 py-12">
        <h2 className="text-center text-2xl font-semibold mb-1">{t('pricing.faq_title')}</h2>
        <p className="text-center text-sm text-[var(--app-ink-subtle)] mb-8">{t('pricing.faq_caption')}</p>
        <div className="space-y-2">
          {faq.map((item, i) => (
            <details
              key={i}
              className="group rounded-xl border border-[var(--app-hairline)] bg-[var(--app-surface-1)] open:bg-[var(--app-surface-2)] transition-colors"
            >
              <summary className="cursor-pointer list-none flex items-center justify-between gap-4 p-4 text-[var(--app-ink)] font-medium text-sm">
                <span>{item.q}</span>
                <ChevronDown className="w-4 h-4 text-[var(--app-ink-subtle)] transition-transform group-open:rotate-180" />
              </summary>
              <p className="px-4 pb-4 text-sm text-[var(--app-ink-muted)] leading-relaxed">
                {item.a}
              </p>
            </details>
          ))}
        </div>
      </section>

      {/* ────── FOOTER (极简) ────── */}
      <footer className="border-t border-[var(--app-hairline)] mt-12">
        <div className="mx-auto max-w-5xl px-6 py-6 flex flex-wrap items-center justify-between gap-3 text-xs text-[var(--app-ink-subtle)]">
          <div className="flex items-center gap-2">
            <BrandShield size={18} />
            <span>言镜 AI · v0.9.0</span>
          </div>
          <div className="flex items-center gap-4">
            <Link href="/legal/privacy" className="hover:text-[var(--app-ink)] transition-colors">
              {t('pricing.footer_privacy')}
            </Link>
            <Link href="/legal/terms" className="hover:text-[var(--app-ink)] transition-colors">
              {t('pricing.footer_terms')}
            </Link>
            <a
              href="https://github.com/samwang0420-code/meetily"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1 hover:text-[var(--app-ink)] transition-colors"
            >
              <Github className="w-3 h-3" />
              GitHub
            </a>
          </div>
        </div>
      </footer>
    </div>
  );
}
