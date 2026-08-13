"use client";

import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from '@/i18n';
import { Search, Sparkles, ChevronRight, X } from 'lucide-react';

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

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSelectTopic?: (topicId: number) => void;
}

const TYPES: Array<{ value: string; labelZh: string; labelEn: string }> = [
  { value: 'all', labelZh: '全部', labelEn: 'All' },
  { value: 'project', labelZh: '项目', labelEn: 'Project' },
  { value: 'decision', labelZh: '决议', labelEn: 'Decision' },
  { value: 'person', labelZh: '人物', labelEn: 'Person' },
  { value: 'general', labelZh: '话题', labelEn: 'General' },
];

export function TopicSearchModal({ open, onOpenChange, onSelectTopic }: Props) {
  const { t, locale } = useTranslation();
  const [query, setQuery] = useState('');
  const [filterType, setFilterType] = useState('all');
  const [recent, setRecent] = useState<TopicSearchHit[]>([]);
  const [results, setResults] = useState<TopicSearchHit[]>([]);
  const [selectedTopic, setSelectedTopic] = useState<TopicDossier | null>(null);
  const [searching, setSearching] = useState(false);
  const [loadingDossier, setLoadingDossier] = useState(false);
  const [rebuilding, setRebuilding] = useState(false);
  const [rebuildError, setRebuildError] = useState<string | null>(null);

  const isZh = locale === 'zh';

  const refreshRecent = useCallback(async () => {
    try {
      const list = (await invoke('api_topic_recent', { limit: 8 })) as TopicSearchHit[];
      setRecent(list);
    } catch {
      /* no-op */
    }
  }, []);

  useEffect(() => {
    if (open) {
      void refreshRecent();
      setQuery('');
      setResults([]);
      setSelectedTopic(null);
    }
  }, [open, refreshRecent]);

  const runSearch = useCallback(async () => {
    setSearching(true);
    try {
      if (!query.trim()) {
        setResults([]);
        return;
      }
      const list = (await invoke('api_topic_search', {
        query,
        limit: 20,
      })) as TopicSearchHit[];
      const filtered =
        filterType === 'all'
          ? list
          : list.filter((r) => r.topic_type === filterType);
      setResults(filtered);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.error('topic search failed:', msg);
    } finally {
      setSearching(false);
    }
  }, [query, filterType]);

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      void runSearch();
    }, 200);
    return () => clearTimeout(timeoutId);
  }, [runSearch]);

  const openTopic = useCallback(async (topicId: number) => {
    setLoadingDossier(true);
    try {
      const ds = (await invoke('api_topic_get_dossier', {
        topicId,
      })) as TopicDossier | null;
      setSelectedTopic(ds);
      onSelectTopic?.(topicId);
      setRebuildError(null);
    } finally {
      setLoadingDossier(false);
    }
  }, [onSelectTopic]);

  const triggerRebuild = useCallback(async () => {
    if (!selectedTopic) return;
    setRebuilding(true);
    setRebuildError(null);
    try {
      await invoke('api_topic_rebuild_dossier', {
        topicId: selectedTopic.topic_id,
      });
      // re-fetch dossier
      const ds = (await invoke('api_topic_get_dossier', {
        topicId: selectedTopic.topic_id,
      })) as TopicDossier | null;
      setSelectedTopic(ds);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setRebuildError(msg);
    } finally {
      setRebuilding(false);
    }
  }, [selectedTopic]);

  const visible = useMemo(
    () => (query.trim() ? results : recent),
    [query, results, recent]
  );

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-20 px-4 bg-black/40 backdrop-blur-sm"
      onClick={() => onOpenChange(false)}
      role="dialog"
      aria-modal="true"
      data-testid="topic-search-modal"
    >
      <div
        className="bg-white dark:bg-neutral-900 rounded-xl shadow-2xl w-full max-w-3xl border border-neutral-200 dark:border-neutral-800 overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-3 px-5 py-4 border-b border-neutral-200 dark:border-neutral-800">
          <Search className="w-5 h-5 text-neutral-500" />
          <input
            autoFocus
            data-testid="topic-search-input"
            value={query}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              setQuery(e.target.value)
            }
            placeholder={
              isZh
                ? '搜索跨会议主题 (例: API 限流 / 张伟招聘 / Q3 OKR)'
                : 'Search cross-meeting topics (e.g. API limits / Q3 OKR)'
            }
            className="flex-1 bg-transparent outline-none text-sm placeholder:text-neutral-400"
          />
          <select
            value={filterType}
            onChange={(e: React.ChangeEvent<HTMLSelectElement>) =>
              setFilterType(e.target.value)
            }
            className="text-xs px-2 py-1 rounded-md border border-neutral-300 dark:border-neutral-700 bg-transparent"
            data-testid="topic-search-filter"
          >
            {TYPES.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {isZh ? opt.labelZh : opt.labelEn}
              </option>
            ))}
          </select>
          <button
            onClick={() => onOpenChange(false)}
            className="text-neutral-500 hover:text-neutral-700"
            aria-label={t('common.close')}
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 max-h-[60vh] overflow-hidden">
          <div className="overflow-y-auto border-r border-neutral-200 dark:border-neutral-800">
            <div className="text-xs px-4 py-2 text-neutral-500 uppercase tracking-wide">
              {query.trim()
                ? isZh
                  ? `搜索结果 (${results.length})`
                  : `Results (${results.length})`
                : isZh
                  ? '近期活跃主题'
                  : 'Recently active topics'}
            </div>
            {searching && visible.length === 0 && (
              <div className="px-4 py-3 text-sm text-neutral-500">
                {isZh ? '搜索中…' : 'Searching...'}
              </div>
            )}
            {!searching && visible.length === 0 && (
              <div className="px-4 py-6 text-sm text-neutral-500 text-center">
                {isZh ? '暂无主题' : 'No topics yet'}
              </div>
            )}
            {visible.map((topic) => (
              <button
                key={topic.topic_id}
                data-testid="topic-search-hit"
                onClick={() => void openTopic(topic.topic_id)}
                className={`w-full text-left px-4 py-3 hover:bg-neutral-100 dark:hover:bg-neutral-800 transition-colors border-b border-neutral-100 dark:border-neutral-800 ${
                  selectedTopic?.topic_id === topic.topic_id
                    ? 'bg-neutral-100 dark:bg-neutral-800'
                    : ''
                }`}
              >
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">{topic.canonical_name}</span>
                  <span className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded bg-neutral-200 dark:bg-neutral-700 text-neutral-600 dark:text-neutral-300">
                    {topic.topic_type}
                  </span>
                  <span className="ml-auto text-xs text-neutral-500">
                    {topic.mention_count}x
                  </span>
                </div>
                {topic.sample_excerpts[0] && (
                  <div className="text-xs text-neutral-500 mt-1 line-clamp-2">
                    "{topic.sample_excerpts[0]}"
                  </div>
                )}
                {topic.last_decided && (
                  <div className="text-xs text-emerald-600 dark:text-emerald-400 mt-1">
                    ✓ {topic.last_decided}
                  </div>
                )}
              </button>
            ))}
          </div>

          <div className="overflow-y-auto p-4">
            {!selectedTopic && (
              <div className="text-sm text-neutral-500 py-12 text-center">
                <Sparkles className="w-6 h-6 mx-auto mb-3 text-neutral-400" />
                <p>
                  {isZh
                    ? '选一个主题查看跨会议档案 (status / open_questions / last_decided)'
                    : 'Select a topic to view its cross-meeting dossier'}
                </p>
              </div>
            )}
            {loadingDossier && (
              <div className="text-sm text-neutral-500 py-12 text-center">
                {isZh ? '加载中…' : 'Loading...'}
              </div>
            )}
            {selectedTopic && !loadingDossier && (
              <div className="space-y-3" data-testid="topic-search-dossier">
                <div className="flex items-center gap-2">
                  <h3 className="text-lg font-semibold">
                    {selectedTopic.canonical_name}
                  </h3>
                  <span className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded bg-neutral-200">
                    {selectedTopic.status}
                  </span>
                </div>
                <div className="text-xs text-neutral-500">
                  {isZh
                    ? `更新于 ${selectedTopic.last_updated_at} · 已聚集 ${selectedTopic.episodes.length} 段`
                    : `Updated ${selectedTopic.last_updated_at} · ${selectedTopic.episodes.length} episodes`}
                </div>
                <button
                  onClick={() => void triggerRebuild()}
                  disabled={rebuilding}
                  data-testid="topic-search-rebuild"
                  className="text-xs px-3 py-1 rounded border border-blue-200 hover:bg-blue-50 disabled:opacity-50 text-blue-700"
                >
                  {rebuilding
                    ? (isZh ? '重建中…' : 'Rebuilding…')
                    : (isZh ? '重建档案' : 'Rebuild dossier')}
                </button>
                {rebuildError && (
                  <div className="text-xs text-red-600">{rebuildError}</div>
                )}

                {selectedTopic.summary && (
                  <Section title={isZh ? '背景' : 'Summary'}>
                    {selectedTopic.summary}
                  </Section>
                )}
                {selectedTopic.last_decided && (
                  <Section title={isZh ? '上次决议' : 'Last decided'} accent="emerald">
                    {selectedTopic.last_decided}
                  </Section>
                )}
                {selectedTopic.open_questions && (
                  <Section title={isZh ? '待解决问题' : 'Open questions'} accent="amber">
                    {selectedTopic.open_questions}
                  </Section>
                )}

                <div>
                  <h4 className="text-xs uppercase tracking-wide text-neutral-500 mt-3 mb-2">
                    {isZh ? '相关会议片段' : 'Episodes'}
                  </h4>
                  <ul className="space-y-2">
                    {selectedTopic.episodes.map((ep) => (
                      <li
                        key={ep.id}
                        className="text-xs bg-neutral-50 dark:bg-neutral-800/50 rounded p-2"
                      >
                        <div className="text-neutral-500 mb-1">
                          {ep.created_at} · {ep.sentiment} ·{' '}
                          <span className="font-mono">{ep.meeting_id.slice(0, 12)}</span>
                        </div>
                        {ep.excerpt && (
                          <div className="line-clamp-3 text-neutral-700 dark:text-neutral-300">
                            "{ep.excerpt}"
                          </div>
                        )}
                      </li>
                    ))}
                    {selectedTopic.episodes.length === 0 && (
                      <li className="text-xs text-neutral-500">
                        {isZh ? '暂无相关会议片段' : 'No episodes yet'}
                      </li>
                    )}
                  </ul>
                </div>
              </div>
            )}
          </div>
        </div>

        <div className="px-5 py-2 border-t border-neutral-200 dark:border-neutral-800 text-xs text-neutral-400 flex items-center gap-3">
          <kbd className="px-1.5 py-0.5 rounded bg-neutral-100 dark:bg-neutral-800 text-[10px]">
            ⌘K
          </kbd>
          <span>{isZh ? '快速唤起' : 'Quick open'}</span>
          <ChevronRight className="w-3 h-3" />
          <span>{isZh ? '按 Esc 关闭' : 'Esc to close'}</span>
        </div>
      </div>
    </div>
  );
}

function Section({
  title,
  children,
  accent = 'default',
}: {
  title: string;
  children: React.ReactNode;
  accent?: 'default' | 'emerald' | 'amber';
}) {
  const accentClass =
    accent === 'emerald'
      ? 'border-emerald-300 bg-emerald-50 dark:bg-emerald-900/20 text-emerald-900 dark:text-emerald-200'
      : accent === 'amber'
      ? 'border-amber-300 bg-amber-50 dark:bg-amber-900/20 text-amber-900 dark:text-amber-200'
      : 'border-neutral-200 bg-neutral-50 dark:bg-neutral-800/40 text-neutral-700 dark:text-neutral-300';
  return (
    <div className={`rounded border ${accentClass} p-3`}>
      <h4 className="text-xs uppercase tracking-wide opacity-70 mb-1">{title}</h4>
      <p className="text-sm whitespace-pre-wrap">{children}</p>
    </div>
  );
}
