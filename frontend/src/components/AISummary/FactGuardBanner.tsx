'use client';

import React from 'react';
import { useTranslation } from '@/i18n';
import type { FactGuardReport } from '@/types';

interface FactGuardBannerProps {
  report?: FactGuardReport;
  severe?: boolean;
  /// §148: 法律模板 critical 标记 — 任一命中 (人名漂移 / 角色混淆 / 判决编造) 即显示
  legalCritical?: boolean;
}

function joinItems(items?: string[]): string {
  if (!items || items.length === 0) return '';
  const head = items.slice(0, 3).join('、');
  return items.length > 3 ? `${head} …` : head;
}

export const FactGuardBanner: React.FC<FactGuardBannerProps> = ({ report, severe, legalCritical }) => {
  const { t } = useTranslation();
  if (!report) return null;

  // §148: 法律 critical 横幅 — 仅在 legal_critical=true 时显示, 不堆砌
  const nameDrift = report.name_drift ?? [];
  const roleConfusion = report.role_confusion ?? [];
  const fabricatedVerdict = report.fabricated_verdict ?? [];
  const hasLegalIssue =
    nameDrift.length > 0 || roleConfusion.length > 0 || fabricatedVerdict.length > 0;
  const showLegalCritical = legalCritical ?? hasLegalIssue;

  if (!showLegalCritical) return null;

  return (
    <div
      role="alert"
      data-testid="fact-guard-legal-critical"
      style={{
        margin: '12px 0',
        padding: '14px 16px',
        borderRadius: 8,
        border: '1px solid #dc2626',
        background: '#fef2f2',
        color: '#7f1d1d',
        fontSize: 13,
        lineHeight: 1.6,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 8 }}>
        {t('summary.fact_guard_148.banner_title')}
      </div>
      <ul style={{ margin: 0, paddingLeft: 18 }}>
        {nameDrift.length > 0 && (
          <li>
            <strong>{t('summary.fact_guard_148.name_drift_label')}:</strong>{' '}
            {joinItems(nameDrift)}
          </li>
        )}
        {roleConfusion.length > 0 && (
          <li>
            <strong>{t('summary.fact_guard_148.role_confusion_label')}:</strong>{' '}
            {joinItems(roleConfusion)}
          </li>
        )}
        {fabricatedVerdict.length > 0 && (
          <li>
            <strong>{t('summary.fact_guard_148.fabricated_verdict_label')}:</strong>{' '}
            {joinItems(fabricatedVerdict)}
          </li>
        )}
      </ul>
      <div style={{ marginTop: 8, fontSize: 12, opacity: 0.85 }}>
        {t('summary.fact_guard_148.banner_hint')}
      </div>
    </div>
  );
};

export default FactGuardBanner;
