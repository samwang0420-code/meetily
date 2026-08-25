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
        margin: '12px 0',
        padding: '14px 16px',
        borderRadius: 8,
        border: '1px solid #f59e0b',
        background: '#fffbeb',
        color: '#78350f',
        fontSize: 13,
        lineHeight: 1.6,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 8 }}>
        ⚠️ {t('summary.number_guard_182.banner_title') || '数字一致性校验 — 可能存在幻觉'}
      </div>
      {mismatches.length > 0 && (
        <div style={{ marginBottom: 6 }}>
          <strong>{t('summary.number_guard_182.category_mismatches_label') || '民事赔偿分类错位'}:</strong>
          <ul style={{ margin: '4px 0 0', paddingLeft: 18 }}>
            {mismatches.slice(0, 5).map((m, i) => (
              <li key={i} style={{ fontSize: 12 }}>{m}</li>
            ))}
          </ul>
        </div>
      )}
      {unexpected.length > 0 && (
        <div>
          <strong>{t('summary.number_guard_182.unexpected_numbers_label') || '原文中不存在的数字'}:</strong>{' '}
          {unexpected.slice(0, 6).join('、')}
        </div>
      )}
      <div style={{ marginTop: 8, fontSize: 12, opacity: 0.85 }}>
        💡 {t('summary.number_guard_182.banner_hint') || '类别错位或原文无该数字 — 请人工核对原始转录'}
      </div>
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
        margin: '12px 0',
        padding: '14px 16px',
        borderRadius: 8,
        border: '1px solid #f97316',
        background: '#fff7ed',
        color: '#7c2d12',
        fontSize: 13,
        lineHeight: 1.6,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 8 }}>
        ⚠️ {t('summary.template_mismatch_182.banner_title') || '模板与内容错配 — 请人工复核'}
      </div>
      <ul style={{ margin: 0, paddingLeft: 18 }}>
        {warnings.slice(0, 4).map((w, i) => (
          <li key={i} style={{ fontSize: 12 }}>{w}</li>
        ))}
      </ul>
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
        margin: '12px 0',
        padding: '14px 16px',
        borderRadius: 8,
        border: '1px solid #a855f7',
        background: '#faf5ff',
        color: '#581c87',
        fontSize: 13,
        lineHeight: 1.6,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 8 }}>
        ⚠️ {t('summary.pending_filter_182.banner_title') || '待查明事项真伪过滤 — 可能是辩论数据, 不是待查项'}
      </div>
      <ul style={{ margin: 0, paddingLeft: 18 }}>
        {warnings.slice(0, 4).map((w, i) => (
          <li key={i} style={{ fontSize: 12 }}>{w}</li>
        ))}
      </ul>
      <div style={{ marginTop: 8, fontSize: 12, opacity: 0.85 }}>
        💡 建议: 把这些项移到"庭审争议数据"段, 不要列入"待查明"
      </div>
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
        margin: '12px 0',
        padding: '14px 16px',
        borderRadius: 8,
        border: '1px solid #eab308',
        background: '#fefce8',
        color: '#713f12',
        fontSize: 13,
        lineHeight: 1.6,
      }}
    >
      <div style={{ fontWeight: 600, marginBottom: 8 }}>
        ⚠️ {t('summary.timeline_conflict_182.banner_title') || '时间线逻辑冲突 — 请人工核对'}
      </div>
      {yearIssues.length > 0 && (
        <ul style={{ margin: 0, paddingLeft: 18 }}>
          {yearIssues.map((m, i) => (
            <li key={i} style={{ fontSize: 12 }}>{m}</li>
          ))}
        </ul>
      )}
      {ageIssues.length > 0 && (
        <ul style={{ margin: '4px 0 0', paddingLeft: 18 }}>
          {ageIssues.map((m, i) => (
            <li key={i} style={{ fontSize: 12 }}>{m}</li>
          ))}
        </ul>
      )}
    </div>
  );
};
