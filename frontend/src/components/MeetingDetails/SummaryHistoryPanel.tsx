'use client';

import { useEffect, useState, useCallback } from 'react';
import { useTranslation } from '@/i18n';
import { invoke } from '@tauri-apps/api/core';
import { X, Clock, GitCompare, FileText, Eye } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';

interface HistoryEntry {
  id: number;
  meeting_id: string;
  template_id: string | null;
  template_name: string | null;
  model_name: string | null;
  chunk_count: number;
  processing_time: number;
  created_at: string;
  archived_at: string;
  backup_reason: string;
  result_json: string;
}

interface SummaryHistoryPanelProps {
  meetingId: string;
  currentTemplateName?: string;
  onLoadHistory: (historyId: number) => void;
  open: boolean;
  onClose: () => void;
}

/**
 * §135: 历史摘要弹窗
 * - 列出该会议所有历史摘要 (按时间倒序)
 * - 每条: 模板名 + chunks + 字符数 + 时间 + 切换按钮
 * - 选 2 条可并排 diff 对比
 */
export function SummaryHistoryPanel({
  meetingId,
  currentTemplateName,
  onLoadHistory,
  open,
  onClose
}: SummaryHistoryPanelProps) {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = (await invoke('api_summary_history', { meetingId })) as HistoryEntry[];
      setEntries(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [meetingId]);

  useEffect(() => {
    if (open) { void load(); }
  }, [open, load]);

  const toggleSelect = (id: number) => {
    const next = new Set(selected);
    if (next.has(id)) { next.delete(id); }
    else if (next.size < 2) { next.add(id); }
    else {
      const first = next.values().next().value;
      if (first !== undefined) next.delete(first);
      next.add(id);
    }
    setSelected(next);
  };

  const formatTime = (s: string) => {
    try { return new Date(s).toLocaleString(); } catch { return s; }
  };

  const markdownLen = (json: string): number => {
    try {
      const p = JSON.parse(json);
      const md = p?.markdown ?? p?.english_cache?.markdown ?? '';
      return String(md).length;
    } catch { return 0; }
  };

  const preview = (json: string): string => {
    try {
      const p = JSON.parse(json);
      const md = p?.markdown ?? p?.english_cache?.markdown ?? '';
      return String(md).slice(0, 120).replace(/\n+/g, ' ');
    } catch { return ''; }
  };

  const selectedEntries = entries.filter(e => selected.has(e.id));

  if (!open) return null;

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
        onClick={onClose}
      >
        <motion.div
          initial={{ scale: 0.95, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          exit={{ scale: 0.95, opacity: 0 }}
          className="relative flex max-h-[85vh] w-full max-w-4xl flex-col rounded-2xl bg-white shadow-2xl"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="flex items-center justify-between border-b px-6 py-4">
            <div className="flex items-center gap-3">
              <Clock className="h-5 w-5 text-violet-600" />
              <h2 className="text-lg font-semibold text-neutral-900">
                {t('summary.history_dialog_title')}
              </h2>
              {entries.length > 0 && (
                <span className="text-xs text-neutral-500">
                  {t('summary.history_count_label', { n: entries.length })}
                </span>
              )}
            </div>
            <button onClick={onClose} className="rounded-lg p-1.5 text-neutral-500 hover:bg-neutral-100 hover:text-neutral-700">
              <X className="h-5 w-5" />
            </button>
          </div>

          {currentTemplateName && (
            <div className="border-b bg-violet-50/40 px-6 py-3 text-[13px]">
              <span className="font-medium text-violet-700">
                {t('summary.history_current_label')}: {currentTemplateName}
              </span>
            </div>
          )}

          <div className="flex-1 overflow-y-auto px-6 py-4">
            {loading && (
              <div className="flex items-center justify-center py-12 text-neutral-500">
                {t('common.loading')}
              </div>
            )}
            {error && (
              <div className="rounded-lg border border-red-200 bg-red-50 p-4 text-sm text-red-700">{error}</div>
            )}
            {!loading && !error && entries.length === 0 && (
              <div className="flex flex-col items-center justify-center py-16 text-center text-neutral-500">
                <FileText className="mb-3 h-10 w-10 text-neutral-300" />
                <p className="text-sm">{t('summary.history_empty')}</p>
              </div>
            )}

            {selected.size === 2 && (
              <div className="mb-4 rounded-xl border border-violet-200 bg-violet-50/50 p-4">
                <div className="mb-2 flex items-center gap-2 text-sm font-medium text-violet-700">
                  <GitCompare className="h-4 w-4" />
                  {t('summary.history_compare_label')}
                </div>
                <div className="grid grid-cols-2 gap-3 text-xs">
                  {selectedEntries.map((e) => (
                    <div key={e.id} className="rounded-lg bg-white p-3 shadow-sm">
                      <div className="font-medium text-neutral-800">
                        {e.template_name || e.template_id}
                      </div>
                      <div className="mt-1 text-neutral-500">{formatTime(e.archived_at)}</div>
                      <div className="mt-1 line-clamp-3 text-neutral-600">
                        {preview(e.result_json)}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}

            <div className="space-y-2">
              {entries.map((e) => {
                const isSelected = selected.has(e.id);
                return (
                  <div
                    key={e.id}
                    className={
                      'flex items-center gap-3 rounded-xl border px-4 py-3 transition-colors ' +
                      (isSelected ? 'border-violet-400 bg-violet-50/60' : 'border-neutral-200 hover:border-neutral-300 hover:bg-neutral-50/50')
                    }
                  >
                    <input
                      type="checkbox"
                      checked={isSelected}
                      onChange={() => toggleSelect(e.id)}
                      className="h-4 w-4 cursor-pointer accent-violet-600"
                    />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 text-sm font-medium text-neutral-800">
                        <span className="truncate">{e.template_name || e.template_id || 'unknown'}</span>
                        <span className="text-xs text-neutral-400">
                          {e.chunk_count} {t('summary.history_chunks_label')}
                        </span>
                        <span className="text-xs text-neutral-400">
                          {markdownLen(e.result_json)} {t('summary.history_chars_label')}
                        </span>
                      </div>
                      <div className="mt-1 line-clamp-1 text-xs text-neutral-500">{preview(e.result_json)}</div>
                      <div className="mt-1 text-xs text-neutral-400">
                        {formatTime(e.archived_at)}
                        {e.model_name && ` · ${e.model_name}`}
                      </div>
                    </div>
                    <button
                      onClick={() => onLoadHistory(e.id)}
                      className="flex items-center gap-1 rounded-lg bg-violet-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-violet-700"
                    >
                      <Eye className="h-3.5 w-3.5" />
                      {t('summary.history_view_button')}
                    </button>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="border-t px-6 py-3">
            <button
              onClick={onClose}
              className="rounded-lg border border-neutral-200 bg-white px-4 py-1.5 text-sm font-medium text-neutral-700 hover:bg-neutral-50"
            >
              {t('summary.history_close')}
            </button>
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
}
