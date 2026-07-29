// 离线会记 i18n: 轻量运行时切换, 不引入 next-intl
// 启动时根据浏览器语言 + 设置项决定 locale

import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { zh } from './locales/zh';
import { en } from './locales/en';

export type Locale = 'zh' | 'en';
export type Dict = Record<string, unknown>;

export const DICTS: Record<Locale, Dict> = { zh, en };

const STORAGE_KEY = 'lixianhuiji.locale';

function detectInitialLocale(): Locale {
  if (typeof window === 'undefined') return 'zh';
  const saved = window.localStorage?.getItem(STORAGE_KEY);
  if (saved === 'zh' || saved === 'en') return saved;
  const navLang = (navigator?.language ?? '').toLowerCase();
  if (navLang.startsWith('en')) return 'en';
  return 'zh';
}

function lookup(dict: Dict, path: string): unknown {
  const parts = path.split('.');
  let cur: unknown = dict;
  for (const p of parts) {
    if (cur && typeof cur === 'object' && p in (cur as Dict)) cur = (cur as Dict)[p];
    else return undefined;
  }
  return cur;
}

export type TranslateVars = Record<string, string | number>;

function isStringList(v: unknown): v is string[] {
  return Array.isArray(v) && v.every((x) => typeof x === 'string');
}

function interpolateString(v: string, vars?: TranslateVars): string {
  if (!vars) return v;
  return v.replace(/\{(\w+)\}/g, (_, k) => (k in vars ? String(vars[k]) : `{${k}}`));
}

export function makeT(dict: Dict, fallback: Dict) {
  const t = function t(path: string, vars?: TranslateVars): string {
    let v: unknown = lookup(dict, path);
    if (typeof v !== 'string') v = lookup(fallback, path);
    if (typeof v !== 'string') return path;
    return interpolateString(v, vars);
  };
  // tList returns a string array. Missing key → empty array (so `.map()` is safe).
  t.list = function tList(path: string): string[] {
    let v: unknown = lookup(dict, path);
    if (!isStringList(v)) v = lookup(fallback, path);
    return isStringList(v) ? v : [];
  };
  return t;
}

type TranslateFn = ((path: string, vars?: TranslateVars) => string) & {
  list: (path: string) => string[];
};

type Ctx = {
  locale: Locale;
  setLocale: (l: Locale) => void;
  t: TranslateFn;
};

const I18nContext = createContext<Ctx | null>(null);

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [locale, _setLocale] = useState<Locale>('zh');

  useEffect(() => { _setLocale(detectInitialLocale()); }, []);

  const setLocale = useCallback((l: Locale) => {
    _setLocale(l);
    try { window.localStorage?.setItem(STORAGE_KEY, l); } catch {}
    window.dispatchEvent(new CustomEvent('lixianhuiji:locale', { detail: l }));
  }, []);

  useEffect(() => {
    const h = (e: Event) => {
      const l = (e as CustomEvent).detail as Locale;
      if (l === 'zh' || l === 'en') _setLocale(l);
    };
    window.addEventListener('lixianhuiji:locale', h);
    return () => window.removeEventListener('lixianhuiji:locale', h);
  }, []);

  const t = useMemo(() => makeT(DICTS[locale], DICTS.zh), [locale]);
  const value = useMemo(() => ({ locale, setLocale, t }), [locale, setLocale, t]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useTranslation() {
  const ctx = useContext(I18nContext);
  if (!ctx) {
    const fallbackT = makeT(DICTS.zh, DICTS.zh) as TranslateFn; return { locale: 'zh' as Locale, setLocale: () => {}, t: fallbackT };
  }
  return ctx;
}

/**
 * Translate a backend-emitted snake_case progress stage to a localized label.
 * Falls back to the raw stage (capitalised) if no mapping exists.
 */
export function translateStage(stage: string | undefined | null, t: (key: string, vars?: Record<string, string | number>) => string): string {
  if (!stage) return '';
  const key = stage.replace(/-/g, '_');
  const translated = t(`progress_stages.${key}`);
  if (translated && translated !== `progress_stages.${key}`) return translated;
  // fallback: humanise snake_case
  return key
    .split('_')
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
}
