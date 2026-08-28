'use client';

import React from 'react';
import { useTranslation } from '@/i18n';

interface NumberGuardBannerProps {
  /// §182: 数字一致性 report (来自 backend number_consistency 字段)
  report?: {
    unexpected_numbers?: string[];
    category_mismatches?: string[];
    summary_categories?: [string, string][];
  };
}

export const NumberGuardBanner: React.FC<NumberGuardBannerProps> = ({ report }) => {
  const { t } = useTranslation();
  if (!report) return null;
  const unexpected = report.unexpected_numbers ?? [];
  const mismatches = report.category_mismatches ?? [];
  if (unexpected.length === 0 && mismatches.length === 0) return null;

  return (
    <div
      role="alert"
      data-testid="number-guard-banner"
      style={{
        margin: '8px 0',
        padding: '8px 12px',
        borderRadius: 6,
        border: '1px solid #f59e0b',
        background: '#fffbeb',
        color: '#78350f',
        fontSize: 12,
        lineHeight: 1.5,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 4 }}>
        {t('summary.number_guard_182.banner_title')}
      </div>
      {unexpected.length > 0 && (
        <div>
          <span style={{ opacity: 0.85 }}>{t('summary.number_guard_182.unexpected_numbers_label')}: </span>
          {unexpected.slice(0, 6).join('、')}
        </div>
      )}
      {mismatches.length > 0 && (
        <div style={{ marginTop: 2 }}>
          <span style={{ opacity: 0.85 }}>{t('summary.number_guard_182.category_mismatches_label')}: </span>
          {mismatches.slice(0, 3).join('; ')}
        </div>
      )}
    </div>
  );
};

interface TemplateMismatchBannerProps {
  /// §182 P1: 模板错配 report
  report?: {
    criminal_hits_in_civil?: string[];
    civil_hits_in_criminal?: string[];
    mismatch_warnings?: string[];
  };
}

export const TemplateMismatchBanner: React.FC<TemplateMismatchBannerProps> = ({ report }) => {
  const { t } = useTranslation();
  if (!report) return null;
  const warnings = report.mismatch_warnings ?? [];
  if (warnings.length === 0) return null;

  return (
    <div
      role="alert"
      data-testid="template-mismatch-banner"
      style={{
        margin: '8px 0',
        padding: '8px 12px',
        borderRadius: 6,
        border: '1px solid #f97316',
        background: '#fff7ed',
        color: '#7c2d12',
        fontSize: 12,
        lineHeight: 1.5,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 4 }}>
        {t('summary.template_mismatch_182.banner_title')}
      </div>
      <div>{warnings.slice(0, 3).join('; ')}</div>
    </div>
  );
};

interface PendingFilterBannerProps {
  /// §182 P1: 待查明事项真伪过滤结果
  report?: {
    genuine_pending?: string[];
    apparent_false_positive?: string[];
    realignment_warnings?: string[];
  };
}

export const PendingFilterBanner: React.FC<PendingFilterBannerProps> = ({ report }) => {
  const { t } = useTranslation();
  if (!report) return null;
  const warnings = report.realignment_warnings ?? [];
  if (warnings.length === 0) return null;

  return (
    <div
      role="alert"
      data-testid="pending-filter-banner"
      style={{
        margin: '8px 0',
        padding: '8px 12px',
        borderRadius: 6,
        border: '1px solid #a855f7',
        background: '#faf5ff',
        color: '#581c87',
        fontSize: 12,
        lineHeight: 1.5,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 4 }}>
        {t('summary.pending_filter_182.banner_title')}
      </div>
      <div>{warnings.slice(0, 3).join('; ')}</div>
    </div>
  );
};

interface TimelineConflictBannerProps {
  /// §182 P2: 时间线冲突 report
  report?: {
    year_inconsistencies?: string[];
    age_year_inconsistencies?: string[];
  };
}

export const TimelineConflictBanner: React.FC<TimelineConflictBannerProps> = ({ report }) => {
  const { t } = useTranslation();
  if (!report) return null;
  const yearIssues = report.year_inconsistencies ?? [];
  const ageIssues = report.age_year_inconsistencies ?? [];
  if (yearIssues.length === 0 && ageIssues.length === 0) return null;

  return (
    <div
      role="alert"
      data-testid="timeline-conflict-banner"
      style={{
        margin: '8px 0',
        padding: '8px 12px',
        borderRadius: 6,
        border: '1px solid #eab308',
        background: '#fefce8',
        color: '#713f12',
        fontSize: 12,
        lineHeight: 1.5,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 4 }}>
        {t('summary.timeline_conflict_182.banner_title')}
      </div>
      <div>{[...yearIssues, ...ageIssues].slice(0, 3).join('; ')}</div>
    </div>
  );
};

interface PartyRoleBannerProps {
  /// §183 P1: 立场标注检测
  report?: {
    is_appellate?: boolean;
    matched_blacklist?: string[];
    warnings?: string[];
  };
}

export const PartyRoleBanner: React.FC<PartyRoleBannerProps> = ({ report }) => {
  const { t } = useTranslation();
  if (!report) return null;
  const warnings = report.warnings ?? [];
  if (warnings.length === 0) return null;
  return (
    <div
      role="alert"
      data-testid="party-role-banner"
      style={{
        margin: '8px 0',
        padding: '8px 12px',
        borderRadius: 6,
        border: '1px solid #ef4444',
        background: '#fef2f2',
        color: '#7f1d1d',
        fontSize: 12,
        lineHeight: 1.5,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 4 }}>
        {t('summary.party_role_183.banner_title')}
      </div>
      <div>{warnings.slice(0, 3).join('; ')}</div>
    </div>
  );
};

interface TimelineCoverageBannerProps {
  /// §183 P2: 时间线覆盖度检测
  report?: {
    transcript_case_ids?: string[];
    summary_case_ids?: string[];
    missing_case_ids?: string[];
    coverage_warnings?: string[];
  };
}

export const TimelineCoverageBanner: React.FC<TimelineCoverageBannerProps> = ({ report }) => {
  const { t } = useTranslation();
  if (!report) return null;
  const warnings = report.coverage_warnings ?? [];
  if (warnings.length === 0) return null;
  return (
    <div
      role="alert"
      data-testid="timeline-coverage-banner"
      style={{
        margin: '8px 0',
        padding: '8px 12px',
        borderRadius: 6,
        border: '1px solid #2563eb',
        background: '#eff6ff',
        color: '#1e3a8a',
        fontSize: 12,
        lineHeight: 1.5,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 4 }}>
        {t('summary.timeline_coverage_183.banner_title')}
      </div>
      <div>{warnings.slice(0, 3).join('; ')}</div>
    </div>
  );
};
