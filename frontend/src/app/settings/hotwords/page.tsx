'use client';
import React, { useEffect, useState, useRef, useCallback } from 'react';
import { useRouter } from 'next/navigation';
import { useTranslation } from '@/i18n';
import { useAuth } from '@/contexts/AuthContext';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import { Loader2, ArrowLeft, LayoutDashboard } from 'lucide-react';

// v0.7.0+: 选词/勾选/输入自动保存 (debounce 500ms), 不依赖底部"保存"按钮.
type HotwordsConfig = { builtin: string; custom: string; enabled: boolean };

// v0.7.0+: 精简到 2 个对上线内容最敏感的行业 (法律诉讼 + 医疗会诊).
// 技术 / 通用工程词已挪到 daemon STATIC_HOMO 通用段, 任何 pack 都生效.
const PACKS = [
  { value: 'none', i18n: 'hotwords.none' },
  { value: 'legal', i18n: 'hotwords.builtin_legal' },
  { value: 'medical', i18n: 'hotwords.builtin_medical' },
];

export default function HotwordsPage() {
  const { t } = useTranslation();
  const { user, session, loading: authLoading } = useAuth();
  const router = useRouter();

  // 等 auth context 恢复完 session 再决定渲染 — 避免 hooks 顺序错乱 + 未登录闪烁.
  useEffect(() => {
    if (!authLoading && !user) {
      location.href = '/login';
    }
  }, [authLoading, user]);

  const [cfg, setCfg] = useState<HotwordsConfig>({ builtin: 'none', custom: '', enabled: false });
  const [loadingCfg, setLoadingCfg] = useState(true);  // loading initial config from DB
  const [saving, setSaving] = useState(false);
  const [savedTick, setSavedTick] = useState(0);  // 显示"已保存"提示
  const loadedRef = useRef(false);  // 防止初次 load 触发 auto-save 回写

  // 加载 DB 现有配置
  useEffect(() => {
    if (!session) return;
    let cancelled = false;
    (async () => {
      try {
        const r = await invoke<HotwordsConfig>('hotwords_get', { session });
        if (cancelled) return;
        setCfg({ builtin: r.builtin || 'none', custom: r.custom || '', enabled: !!r.enabled });
        // 同步到 daemon globals (供后续转录立刻生效)
        await invoke('hotwords_set_globals', { pack: r.builtin || 'none', custom: r.custom || '' });
      } catch (e) {
        console.error('hotwords_get failed', e);
      } finally {
        if (!cancelled) {
          setLoadingCfg(false);
          // 等下一帧再放开 loadedRef, 确保初次 setCfg 不触发 auto-save
          setTimeout(() => { loadedRef.current = true; }, 0);
        }
      }
    })();
    return () => { cancelled = true; };
  }, [session]);

  const persist = useCallback(async (next: HotwordsConfig, opts: { silent?: boolean } = {}) => {
    if (!session) return;
    setSaving(true);
    try {
      await invoke('hotwords_save', {
        session,
        builtin: next.builtin,
        custom: next.custom,
        enabled: next.enabled,
      });
      await invoke('hotwords_set_globals', { pack: next.builtin, custom: next.custom });
      if (!opts.silent) {
        setSavedTick(t => t + 1);
      }
    } catch (e) {
      console.error('hotwords_save failed', e);
      safeToast.error(t('errors.generic'));
    } finally {
      setSaving(false);
    }
  }, [session, t]);

  // v0.7.0+: cfg 变化后 600ms 自动落库
  const debounceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const latestCfgRef = useRef(cfg);
  useEffect(() => { latestCfgRef.current = cfg; }, [cfg]);
  useEffect(() => {
    if (!loadedRef.current) return;  // skip until initial load done
    if (debounceTimer.current) clearTimeout(debounceTimer.current);
    debounceTimer.current = setTimeout(() => {
      persist(cfg);
    }, 600);
    return () => {
      if (debounceTimer.current) clearTimeout(debounceTimer.current);
    };
  }, [cfg, persist]);

  // 选内置词库时: 强制 enabled=true (除非切到 none)
  const pickBuiltin = (value: string) => {
    setCfg(c => {
      const next = {
        ...c,
        builtin: value,
        enabled: value !== 'none' ? true : c.enabled,
      };
      void persist(next);
      return next;
    });
  };

  const leavePage = async (destination: '/' | '/settings') => {
    if (debounceTimer.current) clearTimeout(debounceTimer.current);
    await persist(latestCfgRef.current, { silent: true });
    router.push(destination);
  };

  const totalWords = (cfg.builtin !== 'none' ? 1 : 0)
    + cfg.custom.split(/[,;\n\s]+/).map(w => w.trim()).filter(Boolean).length;

  // 显示"已保存"提示 1.5s 后自动消失
  useEffect(() => {
    if (savedTick === 0) return;
    const t = setTimeout(() => setSavedTick(0), 1500);
    return () => clearTimeout(t);
  }, [savedTick]);

  if (authLoading || loadingCfg) {
    return (
      <div className="flex items-center justify-center p-12 gap-2 text-sm text-gray-500">
        <Loader2 className="w-4 h-4 animate-spin" />
        {t('common.loading')}
      </div>
    );
  }
  if (!user) {
    return null;  // 上面的 effect 会负责 redirect
  }

  return (
    <div className="min-h-screen bg-gray-50">
      {/* 顶部固定栏: 返回按钮 + 面包屑 + 主标题 */}
      <div className="sticky top-0 z-10 bg-gray-50 border-b border-gray-200">
        <div className="max-w-3xl mx-auto px-6 py-4 flex items-center gap-3">
          <button
            onClick={() => void leavePage('/')}
            className="flex items-center gap-1 text-gray-600 hover:text-gray-900 transition-colors text-sm"
            title={t('common.back')}
          >
            <ArrowLeft className="w-4 h-4" />
            <span>{t('hotwords.back_workspace')}</span>
          </button>
          <span className="text-gray-300">/</span>
          <span className="text-sm text-gray-500">热词词库</span>
        </div>
      </div>

      <div className="max-w-2xl mx-auto p-6 space-y-5">
        <div>
          <h1 className="text-2xl font-semibold text-gray-900">{t('hotwords.title')}</h1>
          <p className="text-sm text-gray-500 mt-2">{t('hotwords.desc')}</p>
        </div>

        <div className="flex items-center justify-between rounded-xl border border-blue-100 bg-blue-50/70 px-4 py-3">
          <p className="text-xs text-blue-800">{t('hotwords.workspace_hint')}</p>
          <button
            type="button"
            onClick={() => void leavePage('/')}
            className="ml-4 inline-flex shrink-0 items-center gap-1.5 rounded-lg bg-blue-600 px-3 py-2 text-xs font-medium text-white hover:bg-blue-700"
          >
            <LayoutDashboard className="h-3.5 w-3.5" />
            {t('hotwords.back_workspace')}
          </button>
        </div>

      <section className="bg-white border border-gray-200 rounded-xl p-5 space-y-3">
        <h2 className="text-sm font-medium text-gray-700">{t('hotwords.builtin')}</h2>
        <div className="grid grid-cols-2 gap-2">
          {PACKS.map(p => (
            <button
              key={p.value}
              onClick={() => pickBuiltin(p.value)}
              className={`text-left text-xs px-3 py-2.5 rounded-lg border transition-colors ${
                cfg.builtin === p.value
                  ? 'border-blue-500 bg-blue-50 text-blue-700 font-medium'
                  : 'border-gray-200 hover:border-gray-300'
              }`}
            >
              {t(p.i18n)}
            </button>
          ))}
        </div>
      </section>

      <section className="bg-white border border-gray-200 rounded-xl p-5 space-y-3">
        <h2 className="text-sm font-medium text-gray-700">{t('hotwords.custom')}</h2>
        <textarea
          rows={4}
          className="w-full px-3 py-2 border border-gray-300 rounded-lg text-xs focus:border-blue-500 focus:ring-1 focus:ring-blue-500"
          placeholder={t('hotwords.custom_placeholder')}
          value={cfg.custom}
          onChange={(e) => setCfg(c => ({ ...c, custom: e.target.value }))}
        />
        <div className="text-xs text-gray-500">{t('hotwords.count_words', { n: totalWords })}</div>
      </section>

      <section className="bg-white border border-gray-200 rounded-xl p-5 space-y-3">
        <label className="flex items-center justify-between cursor-pointer">
          <span className="text-sm font-medium text-gray-700">{t('settings.hotwords_enable')}</span>
          <input
            type="checkbox"
            className="w-4 h-4"
            checked={cfg.enabled}
            onChange={(e) => setCfg(c => {
              const next = { ...c, enabled: e.target.checked };
              void persist(next);
              return next;
            })}
            disabled={cfg.builtin === 'none' && !cfg.custom.trim()}
          />
        </label>
        <p className="text-xs text-gray-500">
          {cfg.builtin === 'none' && !cfg.custom.trim()
            ? t('hotwords.none')
            : `${cfg.builtin !== 'none' ? t('hotwords.builtin') : t('hotwords.custom')} · ${cfg.enabled ? '✓' : '✗'}`}
        </p>
      </section>

      <div className="flex items-center justify-end gap-3 min-h-[24px]">
        {saving && <span className="text-xs text-gray-400">{t('common.loading')}</span>}
        {!saving && savedTick > 0 && (
          <span className="text-xs text-green-600">✓ {t('hotwords.saved')}</span>
        )}
      </div>
      </div>
    </div>
  );
}
