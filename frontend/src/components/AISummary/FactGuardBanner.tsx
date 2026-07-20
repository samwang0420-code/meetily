'use client';

import React from 'react';
import { useTranslation } from '@/i18n';
import type { FactGuardReport } from '@/types';

interface FactGuardBannerProps {
  report?: FactGuardReport;
  severe?: boolean;
}

function joinItems(items?: string[]): string {
  if (!items || items.length === 0) return '';
  const head = items.slice(0, 3).join('、');
  return items.length > 3 ? `${head} …` : head;
}

export const FactGuardBanner: React.FC<FactGuardBannerProps> = ({ report, severe }) => {
  const { t } = useTranslation();

  if (!report) return null;
  const hasIssues =
    (report.unexpected_numbers && report.unexpected_numbers.length > 0) ||
    (report.unexpected_dates && report.unexpected_dates.length > 0) ||
    report.overclaimed_decision;
  if (!hasIssues && !severe) return null;

  const issueCount =
    (report.unexpected_numbers?.length ?? 0) +
    (report.unexpected_dates?.length ?? 0) +
    (report.overclaimed_decision ? 1 : 0);

  const containerStyle: React.CSSProperties = {
    margin: '12px 0',
    padding: '12px 16px',
    borderRadius: 8,
    border: severe ? '1px solid #dc2626' : '1px solid #f59e0b',
    background: severe ? '#fef2f2' : '#fffbeb',
    color: severe ? '#7f1d1d' : '#78350f',
    fontSize: 13,
    lineHeight: 1.6,
  };

  const titleStyle: React.CSSProperties = {
    fontWeight: 600,
    marginBottom: 6,
  };

  return (
    <div role="alert" style={containerStyle} data-testid="fact-guard-banner">
      <div style={titleStyle}>
        {severe ? t('summary.fact_guard.banner_severe') : t('summary.fact_guard.banner_title')}
        {' · '}
        {issueCount === 1
          ? t('summary.fact_guard.review_one')
          : t('summary.fact_guard.review_other', { count: issueCount })}
      </div>
      <ul style={{ margin: 0, paddingLeft: 18 }}>
        {report.unexpected_numbers && report.unexpected_numbers.length > 0 && (
          <li>
            {t('summary.fact_guard.issues_numbers', {
              items: joinItems(report.unexpected_numbers),
            })}
          </li>
        )}
        {report.unexpected_dates && report.unexpected_dates.length > 0 && (
          <li>
            {t('summary.fact_guard.issues_dates', {
              items: joinItems(report.unexpected_dates),
            })}
          </li>
        )}
        {report.overclaimed_decision && (
          <li>{t('summary.fact_guard.issues_decision')}</li>
        )}
      </ul>
    </div>
  );
};

export default FactGuardBanner;
