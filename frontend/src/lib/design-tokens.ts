/**
 * 离线会记 v0.6.0 「Wave」— 设计 token 中心导出
 *
 * 来源:samwang0420-code/awesome-design-md 的 6 份品牌
 *        Linear.app / Raycast / Claude / Vercel / Notion / Spotify
 *
 * 关键决策(参考 design/spec/README.md):
 * - 三状态精确分配:录音红 / 转录紫 / 摘要金
 * - 暗色为主(Linear + Raycast 风格), 仅在 hero / 摘要详情用暖色高亮
 * - 字体 Inter 默认开 ss03 calt kern (Raycast 标志)
 * - 几何 6/8/12/16 px + 9999 pill
 */

import { z } from 'zod';

const colorOrRgba = z.union([
  z.string().regex(/^#[0-9a-fA-F]{3,8}$/),
  z.string().regex(/^rgba?\(/),
]);

const ColorTokensSchema = z.object({
  canvas: colorOrRgba,
  surface1: colorOrRgba, surface2: colorOrRgba, surface3: colorOrRgba, surface4: colorOrRgba,
  hairline: colorOrRgba, hairlineStrong: colorOrRgba,
  ink: colorOrRgba, inkMuted: colorOrRgba, inkSubtle: colorOrRgba, inkTertiary: colorOrRgba,
  recording: colorOrRgba, recordingSoft: colorOrRgba,
  transcript: colorOrRgba, transcriptSoft: colorOrRgba, transcriptHover: colorOrRgba,
  summary: colorOrRgba, summarySoft: colorOrRgba, summaryDeep: colorOrRgba,
  success: colorOrRgba, successSoft: colorOrRgba,
  warning: colorOrRgba, error: colorOrRgba, info: colorOrRgba,
  canvasLight: colorOrRgba, surfaceLight1: colorOrRgba,
  inkLight: colorOrRgba, inkLightMuted: colorOrRgba,
});
export type ColorTokens = z.infer<typeof ColorTokensSchema>;

const DEFAULT_COLORS: ColorTokens = {
  canvas: '#0a0b0d',
  surface1: '#0f1011', surface2: '#141516', surface3: '#191a1b', surface4: '#1d1e20',
  hairline: '#23252a', hairlineStrong: '#34343a',
  ink: '#f4f4f6', inkMuted: '#d0d6e0', inkSubtle: '#8a8f98', inkTertiary: '#62666d',
  recording: '#ff5757', recordingSoft: 'rgba(255,87,87,0.15)',
  transcript: '#5e6ad2', transcriptSoft: 'rgba(94,106,210,0.15)', transcriptHover: '#828fff',
  summary: '#ffc533', summarySoft: 'rgba(255,197,51,0.15)', summaryDeep: '#ab570a',
  success: '#59d499', successSoft: 'rgba(89,212,153,0.15)',
  warning: '#ffa42b', error: '#f3727f', info: '#57c1ff',
  canvasLight: '#faf9f5', surfaceLight1: '#f5f0e8',
  inkLight: '#141413', inkLightMuted: '#6c6a64',
};
export const COLORS: ColorTokens = ColorTokensSchema.parse(DEFAULT_COLORS);

export const TYPOGRAPHY = {
  fontFamily: {
    sans: '"Inter", "Geist", -apple-system, BlinkMacSystemFont, "PingFang SC", "Hiragino Sans GB", sans-serif',
    mono: '"JetBrains Mono", "Geist Mono", ui-monospace, "SF Mono", monospace',
    serif: '"Copernicus", "Tiempos Headline", Georgia, serif',
  },
  fontFeature: { defaults: '"calt", "kern", "liga", "ss03"' },
};

export const RADIUS = {
  xs: '4px', sm: '6px', md: '8px', lg: '12px', xl: '16px', xxl: '24px', pill: '9999px',
};

export const SHADOWS = {
  subtle: '0 1px 2px rgba(0,0,0,0.4)',
  card: '0 1px 0 rgba(255,255,255,0.04) inset, 0 4px 16px rgba(0,0,0,0.4)',
  elevated: '0 0 0 1px rgba(255,255,255,0.06), 0 16px 48px rgba(0,0,0,0.5)',
  dialog: '0 16px 48px rgba(0,0,0,0.5), 0 0 0 1px rgba(255,255,255,0.06)',
};

export type AccentState = 'recording' | 'transcript' | 'summary';
export const ACCENT_BY_STATE: Record<AccentState, { primary: string; soft: string; label: string }> = {
  recording: { primary: COLORS.recording, soft: COLORS.recordingSoft, label: '录音中' },
  transcript: { primary: COLORS.transcript, soft: COLORS.transcriptSoft, label: '转录中' },
  summary: { primary: COLORS.summary, soft: COLORS.summarySoft, label: '摘要中' },
};

export const cssVariables = `
  /* design tokens v0.6.0 */
  --app-canvas: ${COLORS.canvas};
  --app-surface-1: ${COLORS.surface1};
  --app-surface-2: ${COLORS.surface2};
  --app-surface-3: ${COLORS.surface3};
  --app-surface-4: ${COLORS.surface4};
  --app-hairline: ${COLORS.hairline};
  --app-hairline-strong: ${COLORS.hairlineStrong};

  --app-ink: ${COLORS.ink};
  --app-ink-muted: ${COLORS.inkMuted};
  --app-ink-subtle: ${COLORS.inkSubtle};
  --app-ink-tertiary: ${COLORS.inkTertiary};

  --app-recording: ${COLORS.recording};
  --app-recording-soft: ${COLORS.recordingSoft};
  --app-transcript: ${COLORS.transcript};
  --app-transcript-soft: ${COLORS.transcriptSoft};
  --app-transcript-hover: ${COLORS.transcriptHover};
  --app-summary: ${COLORS.summary};
  --app-summary-soft: ${COLORS.summarySoft};
  --app-summary-deep: ${COLORS.summaryDeep};

  --app-success: ${COLORS.success};
  --app-success-soft: ${COLORS.successSoft};
  --app-warning: ${COLORS.warning};
  --app-error: ${COLORS.error};
  --app-info: ${COLORS.info};

  --app-canvas-light: ${COLORS.canvasLight};
  --app-surface-light-1: ${COLORS.surfaceLight1};
  --app-ink-light: ${COLORS.inkLight};
  --app-ink-light-muted: ${COLORS.inkLightMuted};

  --app-radius-xs: ${RADIUS.xs};
  --app-radius-sm: ${RADIUS.sm};
  --app-radius-md: ${RADIUS.md};
  --app-radius-lg: ${RADIUS.lg};
  --app-radius-xl: ${RADIUS.xl};
  --app-radius-xxl: ${RADIUS.xxl};
  --app-radius-pill: ${RADIUS.pill};

  --app-shadow-subtle: ${SHADOWS.subtle};
  --app-shadow-card: ${SHADOWS.card};
  --app-shadow-elevated: ${SHADOWS.elevated};
  --app-shadow-dialog: ${SHADOWS.dialog};

  --app-font-sans: ${TYPOGRAPHY.fontFamily.sans};
  --app-font-mono: ${TYPOGRAPHY.fontFamily.mono};
  --app-font-serif: ${TYPOGRAPHY.fontFamily.serif};
  --app-font-feature: ${TYPOGRAPHY.fontFeature.defaults};
`;

export default { COLORS, TYPOGRAPHY, RADIUS, SHADOWS, ACCENT_BY_STATE, cssVariables };
