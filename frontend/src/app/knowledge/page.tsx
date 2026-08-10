'use client';

import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'next/navigation';
import {
  ArrowLeft, BookOpen, Sparkles, ChevronRight, RefreshCw,
  CheckCircle2, Circle, Loader2, Network, ListTodo,
  FileDown, Server, Database, Clock
} from 'lucide-react';
import { motion } from 'framer-motion';
import { useTranslation } from '@/i18n';

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

interface ActionItem {
  id: number;
  meeting_id: string;
  meeting_title: string | null;
  content: string;
  owner: string | null;
  done: number;
  created_at: string;
}

interface ObsidianSettings {
  enabled: boolean;
  vault_path: string;
  folder: string;
  include_transcript: number;
}

const TYPES: Record<string, { zh: string; en: string; color: string }> = {
  project:  { zh: '项目', en: 'Project', color: 'bg-blue-50 text-blue-700 border-blue-200' },
  decision: { zh: '决议', en: 'Decision', color: 'bg-emerald-50 text-emerald-700 border-emerald-200' },
  person:   { zh: '人物', en: 'Person', color: 'bg-violet-50 text-violet-700 border-violet-200' },
  general:  { zh: '话题', en: 'General', color: 'bg-neutral-100 text-neutral-700 border-neutral-200' },
};

export default function KnowledgePage() {
  const router = useRouter();
  const { t, locale } = useTranslation();
  const isZh = locale === 'zh';

  const [topics, setTopics] = useState<TopicSearchHit[]>([]);
  const [selectedTopic, setSelectedTopic] = useState<TopicDossier | null>(null);
  const [loadingTopic, setLoadingTopic] = useState(false);
  const [rebuilding, setRebuilding] = useState(false);
  const [rebuildError, setRebuildError] = useState<string | null>(null);

  const [actionItems, setActionItems] = useState<ActionItem[]>([]);
  const [loadingActions, setLoadingActions] = useState(false);

  const [obsidian, setObsidian] = useState<ObsidianSettings | null>(null);

  const [searchQuery, setSearchQuery] = useState('');
  const [filterType, setFilterType] = useState<string>('all');

  const loadTopics = useCallback(async () => {
    try {
      const list = (await invoke('api_topic_recent', { limit: 30 })) as TopicSearchHit[];
      setTopics(list);
    } catch (e) {
      console.warn('load topics failed', e);
    }
  }, []);

  const loadActionItems = useCallback(async () => {
    setLoadingActions(true);
    try {
      // list across all meetings: need backend support; we approximate via topic dashboard query
      const list = (await invoke('api_action_item_dashboard', { limit: 20 })) as ActionItem[];
      setActionItems(list);
    } catch (e) {
      // backend may not yet expose; show empty state instead of error
      setActionItems([]);
    } finally {
      setLoadingActions(false);
    }
  }, []);

  const loadObsidian = useCallback(async () => {
    try {
      // user_id=0 means "current session user" (mirroring §49 fallback)
      const s = (await invoke('api_obsidian_get_settings', { userId: 0 })) as ObsidianSettings;
      setObsidian(s);
    } catch (e) {
      setObsidian(null);
    }
  }, []);

  useEffect(() => {
    void loadTopics();
    void loadActionItems();
    void loadObsidian();
  }, [loadTopics, loadActionItems, loadObsidian]);

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
      await invoke('api_topic_rebuild_dossier', { topicId: selectedTopic.topic_id });
      const ds = (await invoke('api_topic_get_dossier', { topicId: selectedTopic.topic_id })) as TopicDossier | null;
      setSelectedTopic(ds);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setRebuildError(msg);
    } finally {
      setRebuilding(false);
    }
  }, [selectedTopic]);

  const toggleAction = useCallback(async (id: number, done: boolean) => {
    try {
      await invoke('api_action_item_toggle', { id, done });
      setActionItems(prev => prev.map(it => it.id === id ? { ...it, done: done ? 1 : 0 } : it));
    } catch (e) {
      console.warn('toggle action failed', e);
    }
  }, []);

  const filteredTopics = topics.filter(tp => {
    if (filterType !== 'all' && tp.topic_type !== filterType) return false;
    if (searchQuery.trim() && !tp.canonical_name.toLowerCase().includes(searchQuery.toLowerCase())) return false;
    return true;
  });

  const openCount = actionItems.filter(it => it.done === 0).length;
  const totalTopics = topics.length;
  const decisionCount = topics.filter(tp => tp.topic_type === 'decision').length;
  const projectCount = topics.filter(tp => tp.topic_type === 'project').length;

  return (
    <div className="flex-1 overflow-y-auto bg-gradient-to-b from-violet-50/30 via-white to-white">
      <div className="mx-auto max-w-5xl px-6 py-10">
        {/* Header */}
        <div className="flex items-center justify-between pb-6">
          <div className="flex items-center gap-3">
            <button
              onClick={() => router.push('/')}
              className="flex h-8 w-8 items-center justify-center rounded-lg border border-neutral-200 bg-white transition-colors hover:bg-neutral-50"
              aria-label={isZh ? '返回' : 'Back'}
            >
              <ArrowLeft className="h-4 w-4 text-neutral-600" />
            </button>
            <div>
              <h1 className="flex items-center gap-2 text-[24px] font-semibold tracking-[-0.02em] text-neutral-900">
                <Network className="h-6 w-6 text-violet-500" />
                {isZh ? '知识图谱' : 'Knowledge Graph'}
                <span className="rounded-full bg-violet-100 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-violet-700">
                  {isZh ? 'P0' : 'P0'}
                </span>
              </h1>
              <p className="mt-1 text-[13px] text-neutral-500">
                {isZh
                  ? '跨会议主题追踪 · 每场会议结束自动提取主题、人物、决议、问题'
                  : 'Cross-meeting topic tracking · auto-extract topics/people/decisions/questions per meeting'}
              </p>
            </div>
          </div>
        </div>

        {/* Stat cards */}
        <div className="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
          <StatCard
            icon={<Network className="h-4 w-4" />}
            label={isZh ? '主题总数' : 'Topics'}
            value={totalTopics}
            color="violet"
          />
          <StatCard
            icon={<Sparkles className="h-4 w-4" />}
            label={isZh ? '决议' : 'Decisions'}
            value={decisionCount}
            color="emerald"
          />
          <StatCard
            icon={<BookOpen className="h-4 w-4" />}
            label={isZh ? '项目' : 'Projects'}
            value={projectCount}
            color="blue"
          />
          <StatCard
            icon={<ListTodo className="h-4 w-4" />}
            label={isZh ? '待办行动项' : 'Open Actions'}
            value={openCount}
            color="amber"
          />
        </div>

        {/* Two-column layout */}
        <div className="grid gap-6 lg:grid-cols-[1fr_400px]">
          {/* Left: Topics */}
          <section>
            <div className="mb-3 flex items-center justify-between">
              <h2 className="flex items-center gap-2 text-[15px] font-semibold text-neutral-900">
                <Network className="h-4 w-4 text-violet-500" />
                {isZh ? '近期主题' : 'Recent Topics'}
              </h2>
              <button
                onClick={() => void loadTopics()}
                className="flex items-center gap-1 rounded-md px-2 py-1 text-[11px] text-neutral-500 transition-colors hover:bg-neutral-100"
              >
                <RefreshCw className="h-3 w-3" />
                {isZh ? '刷新' : 'Refresh'}
              </button>
            </div>

            {/* Type filter */}
            <div className="mb-3 flex flex-wrap gap-1.5">
              {['all', 'project', 'decision', 'person', 'general'].map(tp => {
                const active = filterType === tp;
                const label = tp === 'all' ? (isZh ? '全部' : 'All') : (isZh ? TYPES[tp].zh : TYPES[tp].en);
                return (
                  <button
                    key={tp}
                    onClick={() => setFilterType(tp)}
                    className={`rounded-full border px-2.5 py-1 text-[11px] font-medium transition-colors ${
                      active
                        ? 'border-violet-300 bg-violet-100 text-violet-700'
                        : 'border-neutral-200 bg-white text-neutral-600 hover:border-neutral-300'
                    }`}
                  >
                    {label}
                  </button>
                );
              })}
            </div>

            {/* Topic list */}
            <div className="space-y-2">
              {filteredTopics.length === 0 && (
                <div className="rounded-lg border border-dashed border-neutral-300 bg-neutral-50/60 px-6 py-10 text-center">
                  <Network className="mx-auto h-8 w-8 text-neutral-300" />
                  <p className="mt-2 text-[13px] text-neutral-500">
                    {isZh
                      ? '暂无主题。每场会议结束会自动提取主题、人物、决议。'
                      : 'No topics yet. Topics/people/decisions are auto-extracted after each meeting.'}
                  </p>
                </div>
              )}
              {filteredTopics.map(tp => {
                const typeMeta = TYPES[tp.topic_type] || TYPES.general;
                return (
                  <button
                    key={tp.topic_id}
                    onClick={() => void openTopic(tp.topic_id)}
                    className="group flex w-full flex-col items-start gap-2 rounded-lg border border-neutral-200 bg-white p-4 text-left transition-all hover:-translate-y-0.5 hover:border-violet-300 hover:shadow-sm"
                  >
                    <div className="flex w-full items-start justify-between gap-2">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className={`rounded border px-1.5 py-0.5 text-[10px] font-medium ${typeMeta.color}`}>
                          {isZh ? typeMeta.zh : typeMeta.en}
                        </span>
                        <span className="text-[14px] font-medium text-neutral-900">
                          {tp.canonical_name}
                        </span>
                        {tp.status && (
                          <span className="rounded bg-neutral-100 px-1.5 py-0.5 text-[10px] uppercase tracking-wider text-neutral-500">
                            {tp.status}
                          </span>
                        )}
                      </div>
                      <span className="shrink-0 rounded-full bg-neutral-100 px-2 py-0.5 text-[10px] font-medium text-neutral-500">
                        ×{tp.mention_count}
                      </span>
                    </div>
                    {tp.sample_excerpts.length > 0 && (
                      <p className="line-clamp-2 text-[12px] text-neutral-500">
                        "{tp.sample_excerpts[0]}"
                      </p>
                    )}
                    <div className="flex w-full items-center gap-3 text-[10.5px] text-neutral-400">
                      <span className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        {tp.last_touched_at}
                      </span>
                      {tp.last_decided && (
                        <span className="flex items-center gap-1 text-emerald-600">
                          <CheckCircle2 className="h-3 w-3" />
                          {isZh ? '已决议' : 'decided'}
                        </span>
                      )}
                      <ChevronRight className="ml-auto h-3 w-3 transition-transform group-hover:translate-x-0.5" />
                    </div>
                  </button>
                );
              })}
            </div>
          </section>

          {/* Right: Topic dossier + Action items + Obsidian */}
          <aside className="space-y-4">
            {/* Topic dossier panel */}
            {selectedTopic && (
              <motion.div
                initial={{ opacity: 0, y: 4 }}
                animate={{ opacity: 1, y: 0 }}
                className="rounded-lg border border-violet-200 bg-violet-50/40 p-4"
                data-testid="knowledge-dossier"
              >
                <div className="mb-2 flex items-start justify-between gap-2">
                  <h3 className="text-[14px] font-semibold text-neutral-900">
                    {selectedTopic.canonical_name}
                  </h3>
                  <button
                    onClick={() => setSelectedTopic(null)}
                    className="text-[11px] text-neutral-500 hover:text-neutral-700"
                  >
                    ✕
                  </button>
                </div>
                <div className="mb-3 flex items-center gap-2 text-[11px] text-neutral-500">
                  <span className="rounded bg-violet-100 px-1.5 py-0.5 text-violet-700">
                    {selectedTopic.status}
                  </span>
                  <span>·</span>
                  <span>{isZh ? `已聚集 ${selectedTopic.episodes.length} 段` : `${selectedTopic.episodes.length} episodes`}</span>
                </div>

                <button
                  onClick={() => void triggerRebuild()}
                  disabled={rebuilding}
                  data-testid="knowledge-rebuild"
                  className="mb-3 flex w-full items-center justify-center gap-1.5 rounded-md border border-violet-200 bg-white py-1.5 text-[11px] font-medium text-violet-700 transition-colors hover:bg-violet-50 disabled:opacity-50"
                >
                  {rebuilding ? <Loader2 className="h-3 w-3 animate-spin" /> : <RefreshCw className="h-3 w-3" />}
                  {rebuilding ? (isZh ? '重建中…' : 'Rebuilding…') : (isZh ? '重建档案' : 'Rebuild dossier')}
                </button>

                {rebuildError && (
                  <div className="mb-2 rounded border border-red-200 bg-red-50 px-2 py-1 text-[11px] text-red-700">
                    {rebuildError}
                  </div>
                )}

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
                  <div className="mt-3">
                    <h4 className="mb-1 text-[10px] font-semibold uppercase tracking-wider text-neutral-500">
                      {isZh ? '相关片段' : 'Episodes'}
                    </h4>
                    <ul className="space-y-1.5">
                      {selectedTopic.episodes.slice(0, 4).map(ep => (
                        <li key={ep.id} className="rounded border border-neutral-200 bg-white p-2 text-[11px] text-neutral-700">
                          <div className="text-[10px] text-neutral-400">
                            {ep.created_at.slice(0, 16)} · {ep.sentiment}
                          </div>
                          {ep.excerpt && (
                            <div className="line-clamp-2 mt-1">"{ep.excerpt}"</div>
                          )}
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </motion.div>
            )}

            {/* Action items */}
            <div className="rounded-lg border border-neutral-200 bg-white p-4">
              <h3 className="mb-2 flex items-center justify-between text-[14px] font-semibold text-neutral-900">
                <span className="flex items-center gap-2">
                  <ListTodo className="h-4 w-4 text-amber-500" />
                  {isZh ? '行动项' : 'Action Items'}
                </span>
                <span className="rounded-full bg-amber-100 px-2 py-0.5 text-[10px] font-medium text-amber-700">
                  {openCount} {isZh ? '待办' : 'open'}
                </span>
              </h3>
              {loadingActions && (
                <div className="py-4 text-center text-[12px] text-neutral-400">
                  <Loader2 className="mx-auto h-4 w-4 animate-spin" />
                </div>
              )}
              {!loadingActions && actionItems.length === 0 && (
                <div className="py-4 text-center text-[12px] text-neutral-400">
                  {isZh ? '暂无行动项。每场会议摘要生成后会自动提取。' : 'No action items yet. Extracted after each meeting summary.'}
                </div>
              )}
              {!loadingActions && actionItems.length > 0 && (
                <ul className="space-y-1.5">
                  {actionItems.slice(0, 8).map(it => (
                    <li key={it.id} className="flex items-start gap-2 text-[12px]">
                      <button
                        onClick={() => void toggleAction(it.id, it.done === 0)}
                        className="mt-0.5 shrink-0"
                        aria-label={it.done ? (isZh ? '标记未完成' : 'Mark incomplete') : (isZh ? '标记完成' : 'Mark done')}
                      >
                        {it.done ? (
                          <CheckCircle2 className="h-4 w-4 text-emerald-500" />
                        ) : (
                          <Circle className="h-4 w-4 text-neutral-300 hover:text-emerald-500" />
                        )}
                      </button>
                      <div className={`flex-1 ${it.done ? 'text-neutral-400 line-through' : 'text-neutral-700'}`}>
                        <div className="line-clamp-2">{it.content}</div>
                        {it.meeting_title && (
                          <div className="mt-0.5 text-[10px] text-neutral-400">
                            ← {it.meeting_title}
                          </div>
                        )}
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </div>

            {/* Obsidian vault status */}
            <div className="rounded-lg border border-neutral-200 bg-white p-4">
              <h3 className="mb-2 flex items-center gap-2 text-[14px] font-semibold text-neutral-900">
                <FileDown className="h-4 w-4 text-violet-500" />
                {isZh ? 'Obsidian 同步' : 'Obsidian Vault'}
                <span className="rounded-full bg-violet-100 px-1.5 py-0.5 text-[10px] font-medium text-violet-700">
                  {isZh ? 'P0' : 'P0'}
                </span>
              </h3>
              {obsidian === null && (
                <div className="py-3 text-[12px] text-neutral-400">
                  {isZh ? '未配置 (设置 → 通用 → Obsidian 同步)' : 'Not configured (Settings → General → Obsidian)'}
                </div>
              )}
              {obsidian && (
                <div className="space-y-1.5 text-[12px]">
                  <div className="flex items-center gap-1.5">
                    <span className={`inline-block h-2 w-2 rounded-full ${obsidian.enabled ? 'bg-emerald-500' : 'bg-neutral-300'}`} />
                    <span className={obsidian.enabled ? 'text-emerald-700' : 'text-neutral-500'}>
                      {obsidian.enabled ? (isZh ? '已启用' : 'Enabled') : (isZh ? '已停用' : 'Disabled')}
                    </span>
                  </div>
                  <div className="line-clamp-1 text-[11px] text-neutral-500" title={obsidian.vault_path}>
                    {obsidian.vault_path}
                  </div>
                  <div className="text-[11px] text-neutral-400">
                    {isZh ? `写入目录: ${obsidian.folder || '(根目录)'}` : `Folder: ${obsidian.folder || '(root)'}`}
                  </div>
                </div>
              )}
              <button
                onClick={() => router.push('/settings')}
                className="mt-2 flex w-full items-center justify-center gap-1 rounded-md border border-neutral-200 py-1.5 text-[11px] text-neutral-600 transition-colors hover:bg-neutral-50"
              >
                {isZh ? '设置 →' : 'Configure →'}
              </button>
            </div>

            {/* MCP server status */}
            <div className="rounded-lg border border-neutral-200 bg-white p-4">
              <h3 className="mb-2 flex items-center gap-2 text-[14px] font-semibold text-neutral-900">
                <Server className="h-4 w-4 text-blue-500" />
                {isZh ? 'MCP Server' : 'MCP Server'}
                <span className="rounded-full bg-blue-100 px-1.5 py-0.5 text-[10px] font-medium text-blue-700">
                  {isZh ? 'P1' : 'P1'}
                </span>
              </h3>
              <div className="space-y-1.5 text-[12px] text-neutral-600">
                <p>
                  {isZh
                    ? '暴露 5 个工具给 Claude / Cursor：search_meetings / get_meeting_summary / get_action_items / create_meeting_note / get_topic_status'
                    : 'Exposes 5 tools to Claude / Cursor: search_meetings / get_meeting_summary / get_action_items / create_meeting_note / get_topic_status'}
                </p>
                <p className="text-[11px] text-neutral-400">
                  {isZh ? '配置: meetily-mcp 目录 + Claude/Cursor MCP settings.json' : 'Configure via meetily-mcp + your MCP client settings'}
                </p>
              </div>
              <button
                onClick={() => router.push('/settings')}
                className="mt-2 flex w-full items-center justify-center gap-1 rounded-md border border-neutral-200 py-1.5 text-[11px] text-neutral-600 transition-colors hover:bg-neutral-50"
              >
                {isZh ? '查看文档 →' : 'View docs →'}
              </button>
            </div>

            {/* Scheduler status */}
            <div className="rounded-lg border border-neutral-200 bg-white p-4">
              <h3 className="mb-2 flex items-center gap-2 text-[14px] font-semibold text-neutral-900">
                <Database className="h-4 w-4 text-neutral-500" />
                {isZh ? '夜间重建' : 'Nightly Rebuild'}
                <span className="rounded-full bg-neutral-100 px-1.5 py-0.5 text-[10px] font-medium text-neutral-600">
                  {isZh ? 'P2' : 'P2'}
                </span>
              </h3>
              <div className="space-y-1.5 text-[12px] text-neutral-600">
                <p>
                  {isZh
                    ? '0:00-6:00 本地空闲时, 自动重建未更新的主题档案。'
                    : 'Rebuilds stale dossiers during local idle hours (0:00-6:00).'}
                </p>
                <p className="text-[11px] text-neutral-400">
                  {isZh ? '默认: 3 个主题/晚, idle 30min 触发' : 'Default: 3 topics/night, idle 30min'}
                </p>
              </div>
            </div>
          </aside>
        </div>
      </div>
    </div>
  );
}

function StatCard({ icon, label, value, color }: {
  icon: React.ReactNode;
  label: string;
  value: number;
  color: 'violet' | 'emerald' | 'blue' | 'amber';
}) {
  const colorMap = {
    violet: 'bg-violet-50 text-violet-700 border-violet-200',
    emerald: 'bg-emerald-50 text-emerald-700 border-emerald-200',
    blue: 'bg-blue-50 text-blue-700 border-blue-200',
    amber: 'bg-amber-50 text-amber-700 border-amber-200',
  };
  return (
    <div className={`rounded-lg border p-3 ${colorMap[color]}`}>
      <div className="flex items-center gap-1.5 text-[11px] font-medium opacity-80">
        {icon}
        {label}
      </div>
      <div className="mt-1 text-[22px] font-semibold tabular-nums">{value}</div>
    </div>
  );
}

function DossierSection({ title, color, children }: {
  title: string;
  color: 'neutral' | 'emerald' | 'amber';
  children: React.ReactNode;
}) {
  const colorMap = {
    neutral: 'border-neutral-200 bg-white text-neutral-700',
    emerald: 'border-emerald-200 bg-emerald-50 text-emerald-900',
    amber: 'border-amber-200 bg-amber-50 text-amber-900',
  };
  return (
    <div className={`mt-2 rounded border p-2.5 ${colorMap[color]}`}>
      <h4 className="mb-1 text-[10px] font-semibold uppercase tracking-wider opacity-70">
        {title}
      </h4>
      <p className="text-[12px] whitespace-pre-wrap">{children}</p>
    </div>
  );
}
