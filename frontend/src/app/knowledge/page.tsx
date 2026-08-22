'use client';

import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'next/navigation';
import {
  ArrowLeft, Sparkles, ChevronRight, RefreshCw,
  Loader2, Network, GitBranch, AlertCircle, X,
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from '@/i18n';
import { openExternalUrl } from '@/lib/openExternalUrl';
import { useConfig } from '@/contexts/ConfigContext';

interface TopicSearchHit {
  topic_id: number;
  canonical_name: string;
  topic_type: string;
  mention_count: number;
  last_touched_at: string;
  last_decided: string | null;
  status: string | null;
  sample_excerpts: string[];
}

interface TopicEpisode {
  id: number;
  topic_id: number;
  meeting_id: string;
  excerpt: string | null;
  sentiment: string;
  created_at: string;
}

interface TopicDossier {
  topic_id: number;
  canonical_name: string;
  status: string;
  summary: string | null;
  open_questions: string | null;
  last_decided: string | null;
  last_updated_at: string;
  rebuild_count: number;
  episodes: TopicEpisode[];
}

const TYPES: Record<string, { zh: string; en: string; bar: string; chip: string; ink: string; soft: string }> = {
  project:  { zh: '项目', en: 'Project',  bar: 'bg-blue-500',    chip: 'bg-blue-50 text-blue-700 ring-blue-600/20',       ink: 'text-blue-700',    soft: 'bg-blue-50/60' },
  decision: { zh: '决议', en: 'Decision', bar: 'bg-emerald-500', chip: 'bg-emerald-50 text-emerald-700 ring-emerald-600/20', ink: 'text-emerald-700', soft: 'bg-emerald-50/60' },
  person:   { zh: '人物', en: 'Person',   bar: 'bg-violet-500',  chip: 'bg-violet-50 text-violet-700 ring-violet-600/20',   ink: 'text-violet-700',  soft: 'bg-violet-50/60' },
  general:  { zh: '话题', en: 'General',  bar: 'bg-neutral-400', chip: 'bg-neutral-100 text-neutral-700 ring-neutral-500/20', ink: 'text-neutral-600', soft: 'bg-neutral-100/60' },
};

const FILTERS: Array<{ key: string; label_zh: string; label_en: string }> = [
  { key: 'all',      label_zh: '全部', label_en: 'All' },
  { key: 'project',  label_zh: '项目', label_en: 'Projects' },
  { key: 'decision', label_zh: '决议', label_en: 'Decisions' },
  { key: 'person',   label_zh: '人物', label_en: 'People' },
  { key: 'general',  label_zh: '话题', label_en: 'Topics' },
];

// §141.7: 用户 8/20 反馈"会议脉络看不懂" — 整个页面 UI 隐藏,代码保留便于恢复
// 恢复: 把 DISABLED 改 false 即可
const KNOWLEDGE_DISABLED = true;

export default function KnowledgePage() {
  const router = useRouter();
  const { t, locale } = useTranslation();
  const isZh = locale === 'zh';
  // §137.5: 拿用户当前 LLM provider + model (topic extract 用)
  const { modelConfig } = useConfig();

  const [topics, setTopics] = useState<TopicSearchHit[]>([]);
  const [selectedTopic, setSelectedTopic] = useState<TopicDossier | null>(null);
  const [loadingTopic, setLoadingTopic] = useState(false);
  const [rebuilding, setRebuilding] = useState(false);
  const [rebuildError, setRebuildError] = useState<string | null>(null);
  const [recovering, setRecovering] = useState(false);
  // §132: 'running' | 'ollama_offline' | 'done' — 给用户准确状态, 不再写 1~2 分钟误导
  const [recoverStatus, setRecoverStatus] = useState<'idle' | 'running' | 'ollama_offline' | 'done'>('idle');

  const [searchQuery, setSearchQuery] = useState('');
  const [filterType, setFilterType] = useState<string>('all');

  const loadTopics = useCallback(async () => {
    try {
      let list = (await invoke('api_topic_recent', { limit: 60 })) as TopicSearchHit[];
      // §126: 首次进入若 topics 为空, 自动从已完成摘要补提 (历史 silent fail 修复)
      if (list.length === 0) {
        try {
          setRecovering(true);
          // §132: maxMeetings 30 -> 5 (每场 ≤ 30s, 5 场 ≤ 2.5 min, 30 场 = 15 min 等太久了)
          //       Ollama 不可用由后端 emit topic-recover-skipped 事件提示 (useEffect listener),
          //       不再在返回值里塞 sentinel (usize 不能是 -1).
          // §137.5: 传用户当前选的 LLM provider + model_name (不再硬编码 qwen3.5:2b)
          const recover = (await invoke('api_topic_extract_missing', {
            maxMeetings: 5,
            provider: modelConfig.provider,
            modelName: modelConfig.model,
          })) as [number, number];
          if (recover[0] > 0) {
            console.info(`[§132] topic recover: processed=${recover[0]} total_topics=${recover[1]}`);
            list = (await invoke('api_topic_recent', { limit: 60 })) as TopicSearchHit[];
          }
        } catch (e) {
          console.warn('[§126] topic recover failed (Ollama may be offline)', e);
        } finally {
          setRecovering(false);
        }
      }
      setTopics(list);
    } catch (e) {
      console.warn('load topics failed', e);
    }
  }, []);

  useEffect(() => { void loadTopics(); }, [loadTopics]);

  // §132: 监听后端 emit 的 topic-recover-skipped 事件 (Ollama 不可用 preflight fail)
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    (async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlisten = await listen<{ reason: string; at: string }>('topic-recover-skipped', (e) => {
          console.info('[§132] topic recover skipped:', e.payload.reason);
          setRecoverStatus('ollama_offline');
          setRecovering(false);
        });
      } catch (e) {
        console.warn('[§132] failed to subscribe topic-recover-skipped', e);
      }
    })();
    return () => { if (unlisten) void unlisten(); };
  }, []);

  const openTopic = useCallback(async (topicId: number) => {
    setLoadingTopic(true);
    try {
      const ds = (await invoke('api_topic_get_dossier', { topicId })) as TopicDossier | null;
      setSelectedTopic(ds);
      setRebuildError(null);
    } finally {
      setLoadingTopic(false);
    }
  }, []);

  const triggerRebuild = useCallback(async () => {
    if (!selectedTopic) return;
    setRebuilding(true);
    setRebuildError(null);
    try {
      // §137.5: 传用户选的 provider + model (不再硬编码 qwen3.5:2b)
      await invoke('api_topic_rebuild_dossier', {
        topicId: selectedTopic.topic_id,
        provider: modelConfig.provider,
        modelName: modelConfig.model,
      });
      const ds = (await invoke('api_topic_get_dossier', { topicId: selectedTopic.topic_id })) as TopicDossier | null;
      setSelectedTopic(ds);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setRebuildError(msg);
    } finally {
      setRebuilding(false);
    }
  }, [selectedTopic]);

  // §127 大气版: 让"主题"自然过渡到二级聚合 (project/decision/person/general)
  const filteredTopics = topics.filter(tp => {
    if (filterType !== 'all' && tp.topic_type !== filterType) return false;
    if (searchQuery.trim() && !tp.canonical_name.toLowerCase().includes(searchQuery.toLowerCase())) return false;
    return true;
  });

  const totalTopics = topics.length;
  const decisionCount = topics.filter(tp => tp.topic_type === 'decision').length;
  const projectCount  = topics.filter(tp => tp.topic_type === 'project').length;
  const personCount   = topics.filter(tp => tp.topic_type === 'person').length;
  const totalEpisodes = topics.reduce((s, tp) => s + (tp.mention_count || 0), 0);

  if (KNOWLEDGE_DISABLED) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-neutral-50 via-white to-violet-50/30">
        <div className="mx-auto max-w-3xl px-10 py-20">
          <button
            onClick={() => router.push('/')}
            className="mb-8 inline-flex items-center gap-2 rounded-lg px-3 py-1.5 text-[13px] text-neutral-500 transition-colors hover:bg-neutral-100 hover:text-neutral-700"
            data-testid="knowledge-disabled-back"
          >
            <ArrowLeft className="h-4 w-4" />
            {isZh ? '返回工作台' : 'Back to dashboard'}
          </button>
          <div className="rounded-2xl border border-neutral-200/80 bg-white/60 p-10">
            <div className="mx-auto mb-5 flex h-12 w-12 items-center justify-center rounded-xl bg-neutral-100">
              <Network className="h-6 w-6 text-neutral-400" strokeWidth={1.5} />
            </div>
            <h2 className="text-[20px] font-semibold text-neutral-900">
              {isZh ? '会议脉络已停用' : 'Meeting timeline disabled'}
            </h2>
            <p className="mt-3 text-[14px] leading-[1.7] text-neutral-600">
              {isZh
                ? '此功能已隐藏 — 当前的 6 主题 / 2 决议 / 1 项目 / 2 人物 数据未删除,后续如需恢复,把 src/app/knowledge/page.tsx 顶部 KNOWLEDGE_DISABLED 改为 false 即可。'
                : 'This feature is hidden. Existing topic/decision/project/person data is preserved. To restore, set KNOWLEDGE_DISABLED to false in src/app/knowledge/page.tsx.'}
            </p>
            <p className="mt-2 text-[12px] leading-[1.6] text-neutral-400">
              §141.7 · {isZh ? '用户反馈: 会议脉络看不懂' : 'user feedback: cannot understand the timeline'}
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-neutral-50 via-white to-violet-50/30">
      <div className="mx-auto max-w-7xl px-10 py-14">

        {/* ── Hero ── */}
        <header className="mb-16 flex items-end justify-between gap-10">
          <div>
            <button
              onClick={() => router.push('/')}
              className="group mb-7 inline-flex items-center gap-1.5 text-[13px] text-neutral-500 transition-colors hover:text-neutral-900"
            >
              <ArrowLeft className="h-3.5 w-3.5 transition-transform group-hover:-translate-x-0.5" />
              {isZh ? '返回工作台' : 'Back to workspace'}
            </button>
            <h1 className="flex items-baseline gap-3 font-semibold tracking-[-0.04em] text-neutral-900 text-[44px] leading-[1.1]">
              <Network className="h-9 w-9 -translate-y-1 text-violet-500" strokeWidth={1.5} />
              {isZh ? '会议脉络' : 'Meeting Timeline'}
            </h1>
            <p className="mt-4 max-w-2xl text-[16px] leading-[1.6] text-neutral-600">
              {isZh
                ? '每场会议结束, 系统会从摘要中自动抽取主题、人物、项目与决议, 在此聚合为可追溯的脉络。点击任意主题查看背景、决策与相关片段。'
                : 'After each meeting, topics, people, projects, and decisions are auto-extracted from the summary. Click any node to trace its full dossier.'}
            </p>
          </div>
          {/* 4 个数字 stat 一行 */}
          <div className="flex shrink-0 items-stretch divide-x divide-neutral-200/80 rounded-2xl border border-neutral-200/80 bg-white/80 backdrop-blur shadow-sm">
            <StatMini label={isZh ? '主题' : 'Topics'}    value={totalTopics}  accent="text-violet-600"  />
            <StatMini label={isZh ? '决议' : 'Decisions'} value={decisionCount} accent="text-emerald-600" />
            <StatMini label={isZh ? '项目' : 'Projects'}  value={projectCount}  accent="text-blue-600"    />
            <StatMini label={isZh ? '人物' : 'People'}    value={personCount}   accent="text-violet-600"  />
          </div>
        </header>

        {/* ── Toolbar ── */}
        <div className="mb-10 flex items-center justify-between gap-4">
          <div className="flex flex-wrap gap-2">
            {FILTERS.map(f => {
              const active = filterType === f.key;
              const label = isZh ? f.label_zh : f.label_en;
              return (
                <button
                  key={f.key}
                  onClick={() => setFilterType(f.key)}
                  className={`rounded-full px-4 py-1.5 text-[13px] font-medium transition-all ${
                    active
                      ? 'bg-neutral-900 text-white shadow-sm'
                      : 'bg-white text-neutral-600 ring-1 ring-neutral-200 hover:ring-neutral-300 hover:text-neutral-900'
                  }`}
                >
                  {label}
                </button>
              );
            })}
          </div>
          <div className="flex items-center gap-3">
            <div className="relative">
              <input
                value={searchQuery}
                onChange={e => setSearchQuery(e.target.value)}
                placeholder={isZh ? '搜索主题 / 人物 / 项目…' : 'Search topics / people / projects…'}
                className="w-72 rounded-full bg-white px-4 py-2 text-[13px] text-neutral-700 ring-1 ring-neutral-200 transition-all placeholder:text-neutral-400 focus:outline-none focus:ring-2 focus:ring-violet-300"
              />
            </div>
            <button
              onClick={() => void loadTopics()}
              disabled={recovering}
              className="flex items-center gap-1.5 rounded-full bg-white px-4 py-2 text-[13px] font-medium text-neutral-700 ring-1 ring-neutral-200 transition-all hover:ring-neutral-300 disabled:opacity-50"
            >
              {recovering ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
              {recovering ? (isZh ? '回填中…' : 'Recovering…') : (isZh ? '刷新' : 'Refresh')}
            </button>
          </div>
        </div>

        {/* ── Main: topics grid + dossier panel ── */}
        <div className="grid gap-10 lg:grid-cols-[minmax(0,1fr)_420px]">
          {/* Left: topic grid */}
          <section>
            {recovering && (
              <div className="mb-6 flex items-center gap-3 rounded-2xl border border-violet-200 bg-violet-50/60 px-5 py-4">
                <Loader2 className="h-4 w-4 animate-spin text-violet-600" />
                <p className="text-[13px] text-violet-900">
                  {isZh
                    ? `正在从历史摘要回填主题 (需 Ollama 在跑, 单场 ≤ 30s, 最多 5 场)…`
                    : `Backfilling topics from historical summaries (Ollama required, ≤ 30s/meeting, max 5)…`}
                </p>
              </div>
            )}

            {recoverStatus === 'ollama_offline' && (
              <div className="mb-6 rounded-2xl border border-amber-200 bg-amber-50/60 p-5">
                <div className="flex items-start justify-between gap-3">
                  <div className="flex items-start gap-3">
                    <AlertCircle className="mt-0.5 h-5 w-5 flex-shrink-0 text-amber-600" />
                    <div className="flex-1">
                      <h3 className="text-[14px] font-semibold text-amber-900">
                        {t('knowledge.ollama_offline_title')}
                      </h3>
                      <p className="mt-1 text-[13px] leading-[1.6] text-amber-800">
                        {t('knowledge.ollama_offline_desc')}
                      </p>
                      <div className="mt-3 grid gap-2 sm:grid-cols-2">
                        <a
                          href="https://ollama.com/download"
                          onClick={(e) => { e.preventDefault(); openExternalUrl('https://ollama.com/download'); }}
                          className="rounded-lg border border-amber-300 bg-white px-3 py-2 text-[12px] text-amber-900 transition-colors hover:bg-amber-50 cursor-pointer"
                        >
                          <div className="font-medium">{t('knowledge.ollama_offline_option1_title')}</div>
                          <div className="mt-0.5 text-amber-700">{t('knowledge.ollama_offline_option1_desc')}</div>
                          <div className="mt-1 font-medium text-amber-600">→ {t('knowledge.ollama_offline_download')}</div>
                        </a>
                        <a
                          href="/settings/models"
                          className="rounded-lg border border-amber-300 bg-white px-3 py-2 text-[12px] text-amber-900 transition-colors hover:bg-amber-50"
                        >
                          <div className="font-medium">{t('knowledge.ollama_offline_option2_title')}</div>
                          <div className="mt-0.5 text-amber-700">{t('knowledge.ollama_offline_option2_desc')}</div>
                        </a>
                      </div>
                    </div>
                  </div>
                  <button
                    onClick={() => setRecoverStatus('idle')}
                    className="rounded-lg p-1 text-amber-600 transition-colors hover:bg-amber-100"
                    title={t('knowledge.ollama_offline_dismiss')}
                    aria-label={t('knowledge.ollama_offline_dismiss')}
                  >
                    <X className="h-4 w-4" />
                  </button>
                </div>
              </div>
            )}

            {filteredTopics.length === 0 && !recovering && (
              <div className="rounded-3xl border border-dashed border-neutral-300 bg-white/60 px-12 py-20 text-center">
                <div className="mx-auto mb-5 flex h-14 w-14 items-center justify-center rounded-2xl bg-violet-50">
                  <Network className="h-7 w-7 text-violet-400" strokeWidth={1.5} />
                </div>
                <h3 className="text-[17px] font-semibold text-neutral-900">
                  {isZh ? '会议脉络即将自动建立' : 'Timeline builds automatically'}
                </h3>
                <p className="mx-auto mt-2 max-w-sm text-[14px] leading-[1.6] text-neutral-500">
                  {isZh
                    ? '完成一场会议 → 自动生成摘要 → 主题、人物、决议会出现在这里。也可以点击右上角"刷新"手动触发回填。'
                    : 'Finish a meeting → generate summary → topics, people, and decisions appear here. Click "Refresh" to backfill manually.'}
                </p>
              </div>
            )}

            {filteredTopics.length > 0 && (
              <div className="grid gap-4 sm:grid-cols-2">
                {filteredTopics.map(tp => {
                  const meta = TYPES[tp.topic_type] || TYPES.general;
                  const lastTouched = tp.last_touched_at ? new Date(tp.last_touched_at).toLocaleDateString(isZh ? 'zh-CN' : 'en-US', { month: 'short', day: 'numeric' }) : '';
                  return (
                    <button
                      key={tp.topic_id}
                      onClick={() => void openTopic(tp.topic_id)}
                      className="group relative flex flex-col items-start gap-4 overflow-hidden rounded-2xl border border-neutral-200/80 bg-white p-6 text-left shadow-sm transition-all hover:-translate-y-0.5 hover:border-violet-300 hover:shadow-md"
                    >
                      {/* 左侧细条 */}
                      <span className={`absolute left-0 top-6 bottom-6 w-0.5 rounded-r ${meta.bar}`} aria-hidden />
                      <div className="flex w-full items-center justify-between pl-2">
                        <span className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-[11px] font-medium ring-1 ring-inset ${meta.chip}`}>
                          {isZh ? meta.zh : meta.en}
                        </span>
                        <span className="text-[11px] tabular-nums text-neutral-400">
                          {tp.mention_count} {isZh ? '次提及' : 'mentions'} · {lastTouched}
                        </span>
                      </div>
                      <h3 className="w-full pl-2 text-[18px] font-semibold leading-snug text-neutral-900 group-hover:text-violet-700">
                        {tp.canonical_name}
                      </h3>
                      {tp.sample_excerpts && tp.sample_excerpts[0] && (
                        <p className="w-full pl-2 line-clamp-2 text-[13px] leading-[1.55] text-neutral-500">
                          "{tp.sample_excerpts[0]}"
                        </p>
                      )}
                      <div className="flex w-full items-center justify-between pl-2 pt-1">
                        {tp.status ? (
                          <span className={`inline-flex items-center rounded-md px-2 py-0.5 text-[11px] font-medium ${meta.soft} ${meta.ink}`}>
                            {tp.status}
                          </span>
                        ) : <span />}
                        <ChevronRight className="h-4 w-4 text-neutral-300 transition-transform group-hover:translate-x-0.5 group-hover:text-violet-500" />
                      </div>
                    </button>
                  );
                })}
              </div>
            )}
          </section>

          {/* Right: dossier panel */}
          <aside className="lg:sticky lg:top-6 lg:self-start">
            <AnimatePresence mode="wait">
              {!selectedTopic && !loadingTopic && (
                <motion.div
                  key="empty"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  className="rounded-2xl border border-neutral-200/80 bg-white/60 p-10 text-center"
                >
                  <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-neutral-100">
                    <GitBranch className="h-6 w-6 text-neutral-400" strokeWidth={1.5} />
                  </div>
                  <h3 className="text-[15px] font-medium text-neutral-900">
                    {isZh ? '选择一个主题查看档案' : 'Select a topic to view its dossier'}
                  </h3>
                  <p className="mx-auto mt-2 max-w-[260px] text-[12.5px] leading-[1.6] text-neutral-500">
                    {isZh
                      ? '档案包含背景摘要、上次决议、待解决问题, 以及来自相关会议的原文片段。'
                      : 'Dossiers include summary, latest decision, open questions, and original excerpts from related meetings.'}
                  </p>
                  <div className="mt-6 grid grid-cols-2 gap-3 text-left">
                    <CounterMini label={isZh ? '总主题' : 'Topics'}    value={totalTopics} />
                    <CounterMini label={isZh ? '总提及' : 'Mentions'} value={totalEpisodes} />
                  </div>
                </motion.div>
              )}

              {loadingTopic && (
                <motion.div
                  key="loading"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  className="flex h-64 items-center justify-center rounded-2xl border border-neutral-200/80 bg-white/60"
                >
                  <Loader2 className="h-6 w-6 animate-spin text-violet-500" />
                </motion.div>
              )}

              {selectedTopic && !loadingTopic && (
                <motion.div
                  key={`d-${selectedTopic.topic_id}`}
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -8 }}
                  className="rounded-2xl border border-neutral-200/80 bg-white p-7 shadow-sm"
                  data-testid="knowledge-dossier"
                >
                  <div className="mb-5 flex items-start justify-between gap-3">
                    <h3 className="font-semibold text-[20px] leading-[1.3] text-neutral-900">
                      {selectedTopic.canonical_name}
                    </h3>
                    <span className={`shrink-0 rounded-md px-2 py-0.5 text-[11px] font-medium ${TYPES[
                      (topics.find(t => t.topic_id === selectedTopic.topic_id)?.topic_type) || 'general'
                    ].soft} ${TYPES[
                      (topics.find(t => t.topic_id === selectedTopic.topic_id)?.topic_type) || 'general'
                    ].ink}`}>
                      {isZh ? '已聚集档案' : 'Dossier'}
                    </span>
                  </div>

                  <div className="mb-5 flex items-center gap-2 text-[12px] text-neutral-500">
                    <span className="font-medium text-violet-700">{selectedTopic.status}</span>
                    <span>·</span>
                    <span>{isZh ? `${selectedTopic.episodes.length} 段相关片段` : `${selectedTopic.episodes.length} related episodes`}</span>
                    {selectedTopic.rebuild_count > 0 && (<>
                      <span>·</span>
                      <span>{isZh ? `已重建 ${selectedTopic.rebuild_count} 次` : `rebuilt ${selectedTopic.rebuild_count}×`}</span>
                    </>)}
                  </div>

                  <button
                    onClick={() => void triggerRebuild()}
                    disabled={rebuilding}
                    data-testid="knowledge-rebuild"
                    className="mb-6 flex w-full items-center justify-center gap-2 rounded-xl border border-violet-200 bg-violet-50/60 py-2.5 text-[13px] font-medium text-violet-700 transition-colors hover:bg-violet-100 disabled:opacity-50"
                  >
                    {rebuilding ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Sparkles className="h-3.5 w-3.5" />}
                    {rebuilding ? (isZh ? '重建中…' : 'Rebuilding…') : (isZh ? '重建档案' : 'Rebuild dossier')}
                  </button>

                  {rebuildError && (
                    <div className="mb-4 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-[12px] text-red-700">
                      {rebuildError}
                    </div>
                  )}

                  <div className="space-y-4">
                    {selectedTopic.summary && (
                      <DossierSection title={isZh ? '背景' : 'Summary'} color="neutral">
                        {selectedTopic.summary}
                      </DossierSection>
                    )}
                    {selectedTopic.last_decided && (
                      <DossierSection title={isZh ? '上次决议' : 'Last decided'} color="emerald">
                        {selectedTopic.last_decided}
                      </DossierSection>
                    )}
                    {selectedTopic.open_questions && (
                      <DossierSection title={isZh ? '待解决问题' : 'Open questions'} color="amber">
                        {selectedTopic.open_questions}
                      </DossierSection>
                    )}

                    {selectedTopic.episodes.length > 0 && (
                      <div className="pt-2">
                        <h4 className="mb-3 text-[11px] font-semibold uppercase tracking-[0.12em] text-neutral-400">
                          {isZh ? '相关片段' : 'Episodes'}
                        </h4>
                        <ul className="space-y-2.5">
                          {selectedTopic.episodes.slice(0, 5).map(ep => (
                            <li key={ep.id} className="rounded-xl border border-neutral-200/80 bg-neutral-50/50 p-3.5 text-[12.5px] leading-[1.55] text-neutral-700">
                              <div className="mb-1 text-[10.5px] uppercase tracking-wider text-neutral-400">
                                {ep.created_at.slice(0, 16)} · {ep.sentiment}
                              </div>
                              {ep.excerpt && (
                                <div className="line-clamp-3 text-neutral-700">"{ep.excerpt}"</div>
                              )}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </aside>
        </div>
      </div>
    </div>
  );
}

/* ============== Subcomponents ============== */

function StatMini({ label, value, accent }: { label: string; value: number; accent: string }) {
  return (
    <div className="flex min-w-[110px] flex-col items-center justify-center px-6 py-4">
      <div className={`font-semibold tabular-nums leading-none text-[26px] ${accent}`}>{value}</div>
      <div className="mt-1.5 text-[11px] uppercase tracking-[0.08em] text-neutral-500">{label}</div>
    </div>
  );
}

function CounterMini({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-xl border border-neutral-200/80 bg-white p-3.5">
      <div className="text-[11px] uppercase tracking-[0.08em] text-neutral-400">{label}</div>
      <div className="mt-1 font-semibold tabular-nums text-[20px] text-neutral-900">{value}</div>
    </div>
  );
}

function DossierSection({ title, color, children }: {
  title: string; color: 'neutral' | 'emerald' | 'amber'; children: React.ReactNode;
}) {
  const map = {
    neutral: 'border-neutral-200/80 bg-neutral-50/60 text-neutral-800',
    emerald: 'border-emerald-200/60 bg-emerald-50/60 text-emerald-900',
    amber:   'border-amber-200/60 bg-amber-50/60 text-amber-900',
  };
  return (
    <div className={`rounded-xl border p-4 ${map[color]}`}>
      <h4 className="mb-2 text-[10.5px] font-semibold uppercase tracking-[0.12em] opacity-70">{title}</h4>
      <p className="text-[13.5px] leading-[1.65] whitespace-pre-wrap">{children}</p>
    </div>
  );
}
