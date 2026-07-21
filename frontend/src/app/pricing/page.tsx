'use client';

import React from 'react';
import Link from 'next/link';
import { useTranslation } from '@/i18n';

// 离线会记 v0.6.10+: 独立定价页 (C6)
// 主要内容: 
// - 免费档 vs Pro 权益对比表
// - Pro ¥88/年, 一次性买断 (用户偏好终身一次性, 不订阅)
// - FAQ: 退款政策 / 激活码怎么用 / 隐私边界

export default function PricingPage() {
  const { t } = useTranslation();
const FEATURES = [
    {
      category: t('pricing.cat_basics'),
      items: [
        { name: t('pricing.feat_local_transcribe'), free: t('pricing.val_yes'), pro: t('pricing.val_yes') },
        { name: t('pricing.feat_privacy'), free: t('pricing.val_yes'), pro: t('pricing.val_yes') },
        { name: t('pricing.feat_local_summary'), free: t('pricing.val_yes'), pro: t('pricing.val_yes') },
      ],
    },
    {
      category: t('pricing.cat_quota'),
      items: [
        { name: t('pricing.feat_anonymous_quota'), free: t('pricing.val_anonymous_once'), pro: t('pricing.val_na') },
        { name: t('pricing.feat_monthly_meetings'), free: t('pricing.val_quota_5'), pro: t('pricing.val_unlimited') },
        { name: t('pricing.feat_segments_per_meeting'), free: t('pricing.val_segments_100'), pro: t('pricing.val_unlimited') },
      ],
    },
    {
      category: t('pricing.cat_advanced'),
      items: [
        { name: t('pricing.feat_hotwords'), free: t('pricing.val_hotwords_basic'), pro: t('pricing.val_hotwords_all') },
        { name: t('pricing.feat_speaker'), free: t('pricing.val_no'), pro: t('pricing.val_yes') },
        { name: t('pricing.feat_batch'), free: t('pricing.val_no'), pro: t('pricing.val_yes') },
        { name: t('pricing.feat_templates'), free: t('pricing.val_templates_basic'), pro: t('pricing.val_templates_all') },
      ],
    },
    {
      category: t('pricing.cat_support'),
      items: [
        { name: t('pricing.feat_support'), free: t('pricing.val_support_gh'), pro: t('pricing.val_support_7d') },
        { name: t('pricing.feat_priority'), free: t('pricing.val_no'), pro: t('pricing.val_yes') },
      ],
    },
  ];

  const FAQ = [
  {
    q: t('pricing.faq.q_subscription'),
    a: t('pricing.faq.a_subscription'),
  },
  {
    q: t('pricing.faq_q_anonymous_rules'),
    a: t('pricing.faq.a_anonymous'),
  },
  {
    q: t('pricing.faq.q_free_quota'),
    a: t('pricing.faq_a_free_quota'),
  },
  {
    q: t('pricing.faq.q_activate'),
    a: t('pricing.faq.a_activate'),
  },
  {
    q: t('pricing.faq_q_multi_device'),
    a: t('pricing.faq_a_multi_device'),
  },
  {
    q: t('pricing.faq_q_refund'),
    a: t('pricing.faq.a_refund'),
  },
  {
    q: t('pricing.faq_q_buy_beta'),
    a: t('pricing.note_beta'),
  },
  {
    q: t('pricing.faq.q_privacy'),
    a: t('pricing.faq.a_privacy'),
  },
];
  return (
    <div className="max-w-5xl mx-auto p-6">
      <header className="text-center mb-8">
        <h1 className="text-3xl font-semibold tracking-tight text-neutral-900 mb-2">
          {t('pricing.page_title')}
        </h1>
        <p className="text-base text-neutral-600">
          {t('pricing.page_subtitle')}
        </p>
      </header>

      {/* 三档权限卡片 */}
      <div className="grid md:grid-cols-3 gap-4 mb-10">
        <div className="border border-neutral-200 rounded-lg p-5 bg-white">
          <h2 className="text-lg font-medium text-neutral-900 mb-1">{t('pricing.tier_anonymous')}</h2>
          <p className="text-xs text-neutral-500 mb-3">{t('pricing.tier_anonymous_subtitle')}</p>
          <div className="text-3xl font-bold text-neutral-900 mb-1">{t('pricing.tier_anonymous_price_label')}</div>
          <p className="text-xs text-neutral-500 mb-4">{t('pricing.tier_anonymous_price_sub')}</p>
          <ul className="space-y-1 text-xs text-neutral-700 mb-5">
            {t.list('pricing.tier_anonymous_features').map((line, idx) => (
              <li key={idx} className={line.startsWith('✓') ? 'text-green-700' : line.startsWith('—') ? 'text-red-600' : 'text-neutral-500'}>
                {line}
              </li>
            ))}
          </ul>
          <Link href="/" className="block text-center px-4 py-2 border border-neutral-300 rounded-md text-sm font-medium hover:bg-neutral-50">
            {t('pricing.tier_anonymous_cta')}
          </Link>
        </div>

        <div className="border border-sky-300 rounded-lg p-5 bg-white">
          <h2 className="text-lg font-medium text-sky-700 mb-1">{t('pricing.tier_free')}</h2>
          <p className="text-xs text-neutral-500 mb-3">{t('pricing.tier_free_subtitle')}</p>
          <div className="text-3xl font-bold text-neutral-900 mb-1">{t('pricing.tier_free_price_label')}</div>
          <p className="text-xs text-neutral-500 mb-4">{t('pricing.tier_free_price_sub')}</p>
          <ul className="space-y-1 text-xs text-neutral-700 mb-5">
            {t.list('pricing.tier_free_features').map((line, idx) => (
              <li key={idx} className={line.startsWith('✓') ? 'text-green-700' : line.startsWith('—') ? 'text-red-600' : 'text-neutral-500'}>
                {line}
              </li>
            ))}
          </ul>
          <Link href="/register" className="block text-center px-4 py-2 border border-sky-500 text-sky-700 rounded-md text-sm font-medium hover:bg-sky-50">
            {t('pricing.tier_free_cta')}
          </Link>
        </div>

        <div className="border-2 border-blue-500 rounded-lg p-5 bg-white relative">
          <div className="absolute top-0 right-0 bg-blue-500 text-white text-xs px-2 py-1 rounded-bl-lg">
            {t('pricing.recommended')}
          </div>
          <h2 className="text-lg font-medium text-blue-600 mb-1">{t('pricing.tier_pro')}</h2>
          <p className="text-xs text-neutral-500 mb-3">{t('pricing.tier_pro_subtitle')}</p>
          <div className="text-3xl font-bold text-neutral-900 mb-1">{t('pricing.tier_pro_price_label')}</div>
          <p className="text-xs text-neutral-500 mb-4">{t('pricing.tier_pro_price_sub')}</p>
          <ul className="space-y-1 text-xs text-neutral-700 mb-5">
            {t.list('pricing.tier_pro_features').map((line, idx) => (
              <li key={idx} className="text-green-700 font-medium">
                {line}
              </li>
            ))}
          </ul>
          {/* v0.7.0+: 双 CTA — 主按钮"购买", 次按钮"已有激活码直接激活" */}
          <div className="space-y-2">
            <a href="mailto:lisangjie@icloudsend.com?subject=Pro%20%E4%B9%B0%E6%96%AD%E5%92%A8%E8%AF%A2" className="block text-center px-4 py-2 bg-blue-600 text-white rounded-md text-sm font-medium hover:bg-blue-700">
              {t('pricing.tier_pro_cta')}
            </a>
            <Link href="/redeem" className="block text-center px-4 py-2 bg-white border border-blue-300 text-blue-700 rounded-md text-sm font-medium hover:bg-blue-50">
              {t('pricing.tier_pro_cta_redeem')}
            </Link>
          </div>
        </div>
      </div>

      {/* 内测期购买流程 */}
      <section className="mb-10 bg-amber-50 border border-amber-200 rounded-lg p-4">
        <h3 className="text-base font-medium text-amber-900 mb-2">{t('pricing.beta_title')}</h3>
        <ol className="text-sm text-amber-900 list-decimal pl-5 space-y-1">
          <li>{t('pricing.beta_step_1')}</li>
          <li>{t('pricing.beta_step_2')}</li>
          <li>{t('pricing.beta_step_3')}</li>
          <li>{t('pricing.beta_step_4')}</li>
        </ol>
      </section>

      {/* 详细对比表 */}
      <section className="mb-10">
        <h2 className="text-2xl font-medium text-neutral-900 mb-4">{t('pricing.compare_title')}</h2>
        {FEATURES.map((group, gi) => (
          <div key={gi} className="mb-6">
            <h3 className="text-sm font-medium text-neutral-500 uppercase mb-2">
              {group.category}
            </h3>
            <table className="w-full text-sm border-collapse">
              <thead>
                <tr className="border-b border-neutral-200">
                  <th className="text-left py-2 pr-4 font-medium text-neutral-700 w-1/2">
                    {t('pricing.table_header_feature')}
                  </th>
                  <th className="text-center py-2 px-4 font-medium text-neutral-700">
                    {t('pricing.table_header_free')}
                  </th>
                  <th className="text-center py-2 px-4 font-medium text-blue-600">
                    {t('pricing.table_header_pro')}
                  </th>
                </tr>
              </thead>
              <tbody>
                {group.items.map((item, ii) => (
                  <tr key={ii} className="border-b border-neutral-100">
                    <td className="py-3 pr-4 text-neutral-700">{item.name}</td>
                    <td className="py-3 px-4 text-center text-neutral-600">
                      {item.free}
                    </td>
                    <td className="py-3 px-4 text-center text-blue-600 font-medium">
                      {item.pro}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ))}
      </section>

      {/* FAQ */}
      <section className="mb-10">
        <h2 className="text-2xl font-medium text-neutral-900 mb-4">{t('pricing.faq_title')}</h2>
        <div className="space-y-4">
          {FAQ.map((item, i) => (
            <details
              key={i}
              className="border border-neutral-200 rounded-lg p-4 bg-white"
            >
              <summary className="cursor-pointer font-medium text-neutral-900 list-none flex justify-between items-center">
                <span>{item.q}</span>
                <span className="text-neutral-400">+</span>
              </summary>
              <p className="mt-3 text-sm text-neutral-700 leading-relaxed">
                {item.a}
              </p>
            </details>
          ))}
        </div>
      </section>

      {/* 隐私边界提醒 */}
      <section className="mb-6">
        <div className="border border-green-200 bg-green-50 rounded-lg p-4 text-sm text-green-900">
          <strong>{t('pricing.privacy_strong')}</strong> · {t('pricing.privacy_banner_body')} <strong>{t('pricing.privacy_strong_network')}</strong>.
        </div>
      </section>

      {/* 底部链接 */}
      <footer className="text-center text-xs text-neutral-500 pt-4 border-t border-neutral-100">
        <Link href="/legal/privacy" className="hover:underline mx-2">{t('pricing.footer_privacy')}</Link>
        <Link href="/legal/terms" className="hover:underline mx-2">{t('pricing.footer_terms')}</Link>
        <Link href="/" className="hover:underline mx-2">{t('pricing.footer_download')}</Link>
        <p className="mt-2">{t('pricing.footer_copyright')}</p>
      </footer>
    </div>
  );
}
