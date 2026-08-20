'use client';

import { useEffect, useState, useCallback } from 'react';
import { useRouter } from 'next/navigation';
import { motion } from 'framer-motion';
import {
  Mic, Headphones, FileText, Clock, ChevronRight, Sparkles,
  Settings as SettingsIcon, Languages, Plus, BookOpen, Upload
} from 'lucide-react';
import { RecordingControls } from '@/components/RecordingControls';
import { useConfig } from '@/contexts/ConfigContext';
import { useRecordingState, RecordingStatus } from '@/contexts/RecordingStateContext';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import { useImportDialog } from '@/contexts/ImportDialogContext';
import { indexedDBService, type MeetingMetadata } from '@/services/indexedDBService';
import { CardBoundary } from './CardBoundary';
import { useTranslation } from '@/i18n';

interface HomeDashboardProps {
  onRecordingStart: () => void;
  onTranscriptReceived: () => void;
  onTranscriptionError: (message: string) => void;
  isRecordingDisabled: boolean;
  isParentProcessing: boolean;
  barHeights: string[];
  showModal: (type: string, message?: string) => void;
  meetingTitle: string;
  selectedDevices: { micDevice: string | null; systemDevice: string | null };
}

function formatRelative(ts: number, t: (path: string, vars?: Record<string, string | number>) => string) {
  const diff = Date.now() - ts;
  const min = Math.floor(diff / 60000);
  if (min < 1) return t('dashboard.just_now');
  if (min < 60) return t('dashboard.minutes_ago', { count: min });
  const hr = Math.floor(min / 60);
  if (hr < 24) return t('dashboard.hours_ago', { count: hr });
  const day = Math.floor(hr / 24);
  if (day < 7) return t('dashboard.days_ago', { count: day });
  const date = new Date(ts);
  return t('dashboard.date_md', { month: date.getMonth() + 1, day: date.getDate() });
}

function modelLabel(t: (path: string) => string, provider?: string, model?: string) {
  if (provider === 'localWhisper') return t('dashboard.model_local_whisper');
  if (model === 'funasr-nano-zh') return t('dashboard.model_funasr_nano');
  if (provider === 'sherpa_funasr_nano' || provider === 'senseVoice') return t('dashboard.model_sensevoice');
  if (provider === 'cloud') return t('dashboard.model_cloud');
  return t('dashboard.model_none');
}

export function HomeDashboard({
  onRecordingStart,
  onTranscriptReceived,
  onTranscriptionError,
  isRecordingDisabled,
  isParentProcessing,
  barHeights,
  showModal,
  meetingTitle,
  selectedDevices
}: HomeDashboardProps) {
  const router = useRouter();
  const { t } = useTranslation();
  const { transcriptModelConfig } = useConfig();
  const recordingState = useRecordingState();
  const { meetings: sidebarMeetings } = useSidebar();
  const { openImportDialog } = useImportDialog();
  const { status } = recordingState;

  const [recentMeetings, setRecentMeetings] = useState<MeetingMetadata[]>([]);
  const [loading, setLoading] = useState(true);

  // 用侧栏同源 meetings 做 union 兜底 (indexedDB unsaved + sidebar saved)
  const loadRecent = useCallback(async () => {
    try {
      setLoading(true);
      // v0.6.10+: 启动先做一次 sanitize, 清理 schema 不合规的脏数据 (老数据 / 迁移残留)
      try { await indexedDBService.sanitizeMeetings(); } catch {}
      const all = await indexedDBService.getAllMeetings();
      // 取前 6: unsaved 优先 + 已保存但未在 sidebar 的补
      const indexedIds = new Set(all.map(m => m.meetingId));
      const sidebarOnly = sidebarMeetings
        .filter(m => !!m && !!m.id && !indexedIds.has(m.id))
        .slice(0, 6 - all.length)
        .map((m, i) => ({
          meetingId: String(m.id),
          title: String(m.title ?? t('meeting.untitled')),
          startTime: Date.now() - i * 3600 * 1000,
          lastUpdated: Date.now() - i * 3600 * 1000,
          transcriptCount: 0,
          savedToSQLite: true,
          folderPath: undefined
        }));
      // 过滤掉 indexedDB 里 meetingId/title/lastUpdated 缺失或非 string 的脏数据
      const cleanIndexed = all.filter(m =>
        !!m && typeof m.meetingId === 'string' &&
        typeof m.title === 'string' &&
        typeof m.lastUpdated === 'number'
      );
      const merged = [...cleanIndexed, ...sidebarOnly].slice(0, 6);
      setRecentMeetings(merged);
    } catch (e) {
      console.error('HomeDashboard loadRecent failed', e);
      setRecentMeetings([]);
    } finally {
      setLoading(false);
    }
  }, [sidebarMeetings, t]);

  useEffect(() => {
    loadRecent();
  }, [loadRecent]);

  if (recordingState.isRecording || status === RecordingStatus.PROCESSING_TRANSCRIPTS) {
    return null;
  }

  const totalMeetings = sidebarMeetings.length;
  const totalRecent = recentMeetings.length;

  return (
    <div className="flex-1 overflow-y-auto bg-gradient-to-b from-teal-50/40 via-white to-white">
      <div className="mx-auto max-w-3xl px-6 py-16">

        {/* ── Hero ────────────────────────────── */}
        <motion.section
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35 }}
          className="flex flex-col items-center text-center pb-10"
        >
          <h1 className="text-[32px] font-semibold tracking-[-0.02em] text-neutral-900">
            {t('dashboard.welcome')}
          </h1>
          <p className="mt-2 text-[14px] text-neutral-500">
            {t('dashboard.subtitle')}
          </p>

          <div className="mt-9">
            <RecordingControls
              isRecording={false}
              onRecordingStop={() => {}}
              onRecordingStart={onRecordingStart}
              onTranscriptReceived={onTranscriptReceived}
              onTranscriptionError={onTranscriptionError}
              isRecordingDisabled={isRecordingDisabled}
              isParentProcessing={isParentProcessing}
              barHeights={barHeights}
              selectedDevices={selectedDevices}
              meetingName={meetingTitle}
              variant="hero"
            />
          </div>

          <p className="mt-6 text-xs text-neutral-400">
            {t('dashboard.record_hint')}
          </p>
        </motion.section>

        {/* ── Status chips row ────────────────────────────── */}
        <motion.section
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35, delay: 0.05 }}
          className="flex flex-wrap items-center justify-center gap-2 pb-8"
        >
          <StatusChip
            icon={<Languages className="h-3.5 w-3.5" />}
            label={t('dashboard.transcript_model')}
            value={modelLabel(t, transcriptModelConfig?.provider, transcriptModelConfig?.model)}
            tone="teal"
          />
          <StatusChip
            icon={<Mic className="h-3.5 w-3.5" />}
            label={t('dashboard.microphone')}
            value={selectedDevices?.micDevice || t('dashboard.default_input')}
            tone="emerald"
            maxChars={28}
          />
          <button
            onClick={() => router.push('/settings/hotwords')}
            className="group inline-flex items-center gap-1.5 rounded-full border border-amber-100 bg-amber-50/70 px-3 py-1 text-[12px] text-amber-700 transition-colors hover:border-amber-200 hover:bg-amber-50"
          >
            <BookOpen className="h-3.5 w-3.5 text-amber-500 group-hover:text-amber-600" />
            <span>{t('dashboard.hotwords')}</span>
            <ChevronRight className="h-3 w-3 text-amber-300 transition-transform group-hover:translate-x-0.5" />
          </button>
          <button
            onClick={() => router.push('/settings')}
            className="group inline-flex items-center gap-1.5 rounded-full border border-neutral-200 bg-white px-3 py-1 text-[12px] text-neutral-700 transition-colors hover:border-neutral-300 hover:bg-neutral-50"
          >
            <SettingsIcon className="h-3.5 w-3.5 text-neutral-400 group-hover:text-neutral-600" />
            <span>{t('dashboard.settings')}</span>
            <ChevronRight className="h-3 w-3 text-neutral-300 transition-transform group-hover:translate-x-0.5" />
          </button>
        </motion.section>

        {/* ── Quick actions ─────────────────────────────────── */}
        <motion.section
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35, delay: 0.1 }}
          className="mt-2"
        >
          <div className="mx-auto grid max-w-md grid-cols-3 gap-2 pt-2">
            <QuickAction
              icon={<Upload className="h-5 w-5" />}
              label={t('dashboard.qa_import')}
              onClick={() => openImportDialog()}
              accent="teal"
            />
            <QuickAction
              icon={<BookOpen className="h-5 w-5" />}
              label={t('dashboard.qa_hotwords')}
              onClick={() => router.push('/settings/hotwords')}
              accent="amber"
            />
            <QuickAction
              icon={<SettingsIcon className="h-5 w-5" />}
              label={t('dashboard.qa_settings')}
              onClick={() => router.push('/settings')}
              accent="slate"
            />
          </div>
        </motion.section>

        {/* ── Tip footer ─────────────────────────────────── */}
        <motion.section
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.4, delay: 0.2 }}
          className="mt-12 flex items-center justify-center gap-2 text-[11px] text-neutral-400"
        >
          <span className="inline-block h-1.5 w-1.5 rounded-full bg-emerald-500"></span>
          <span>{t('dashboard.local_first')}</span>
          <span className="text-neutral-300">·</span>
          <span className="font-mono">v0.9.0</span>
        </motion.section>
      </div>
    </div>
  );
}

/* ─── Sub-components ────────────────────────────── */

function StatusChip({
  icon, label, value, tone, maxChars
}: {
  icon: React.ReactNode
  label: string
  value: string
  tone: 'blue' | 'emerald' | 'violet' | 'teal'
  maxChars?: number
}) {
  const toneMap = {
    blue: 'bg-blue-50/80 text-blue-700 border-blue-100',
    emerald: 'bg-emerald-50/80 text-emerald-700 border-emerald-100',
    violet: 'bg-violet-50/80 text-violet-700 border-violet-100',
    teal: 'bg-teal-50/80 text-teal-700 border-teal-100',
  }
  const display = maxChars && value.length > maxChars ? value.slice(0, maxChars - 1) + '…' : value
  return (
    <div className={`inline-flex items-center gap-1.5 rounded-full border px-3 py-1 ${toneMap[tone]}`}>
      {icon}
      <span className="text-[11px] uppercase tracking-wider opacity-70">{label}</span>
      <span className="text-[12px] font-medium">{display}</span>
    </div>
  )
}

function MeetingCard({
  meeting, onClick
}: {
  meeting: MeetingMetadata
  onClick: () => void
}) {
  const { t } = useTranslation();
  const safe = meeting || {} as MeetingMetadata
  const total = typeof safe.transcriptCount === 'number' && safe.transcriptCount > 0 ? safe.transcriptCount : 0
  const approxMinutes = Math.max(1, Math.round(total * 4 / 60))
  return (
    <button
      onClick={onClick}
      className="group flex flex-col items-start gap-2 rounded-lg border border-neutral-200 bg-white p-4 text-left transition-all hover:-translate-y-0.5 hover:border-neutral-300 hover:shadow-sm"
    >
      <div className="flex w-full items-start justify-between gap-2">
        <FileText className="h-4 w-4 shrink-0 text-neutral-400 group-hover:text-blue-500" />
        {total > 0 && (
          <span className="rounded-full bg-neutral-100 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-neutral-500">
            {t('dashboard.segments', { count: total })}
          </span>
        )}
      </div>
      <div className="line-clamp-2 text-[13.5px] font-medium text-neutral-900">
        {safe.title || t('meeting.untitled')}
      </div>
      <div className="flex w-full items-center justify-between text-[11px] text-neutral-500">
        <span className="flex items-center gap-1">
          <Clock className="h-3 w-3" />
          {typeof safe.lastUpdated === 'number' ? formatRelative(safe.lastUpdated, t) : t('dashboard.unknown_time')}
        </span>
        {total > 0 && <span>{t('dashboard.approx_minutes', { count: approxMinutes })}</span>}
      </div>
    </button>
  )
}


function QuickAction({ icon, label, onClick, accent }: {
  icon: React.ReactNode
  label: string
  onClick: () => void
  accent: 'teal' | 'amber' | 'slate'
}) {
  const accentMap = {
    teal: 'text-teal-700 group-hover:bg-teal-50 group-hover:border-teal-200',
    amber: 'text-amber-600 group-hover:bg-amber-50 group-hover:border-amber-200',
    slate: 'text-neutral-500 group-hover:bg-neutral-50 group-hover:border-neutral-300',
  }
  return (
    <button
      onClick={onClick}
      className="group flex flex-col items-center gap-2.5 rounded-xl border border-neutral-200/70 bg-white px-4 py-5 transition-all hover:-translate-y-0.5 hover:shadow-md"
    >
      <span className={`flex h-10 w-10 items-center justify-center rounded-full border border-neutral-200 bg-neutral-50 transition-colors ${accentMap[accent]}`}>
        {icon}
      </span>
      <span className="text-[12px] font-medium tracking-tight text-neutral-700">{label}</span>
    </button>
  )
}

function EmptyState({ onStart }: { onStart: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-center justify-center rounded-xl border border-dashed border-neutral-300 bg-neutral-50/60 px-6 py-14 text-center">
      <div className="flex h-14 w-14 items-center justify-center rounded-full bg-white shadow-sm">
        <Headphones className="h-7 w-7 text-neutral-400" />
      </div>
      <h3 className="mt-4 text-[15px] font-medium text-neutral-800">{t('dashboard.empty_title')}</h3>
      <p className="mt-1.5 max-w-sm text-[13px] text-neutral-500">
        {t('dashboard.empty_desc')}
      </p>
      <button
        onClick={onStart}
        className="mt-5 inline-flex items-center gap-2 rounded-md bg-blue-600 px-4 py-1.5 text-sm font-medium text-white hover:bg-blue-700"
      >
        <Plus className="h-3.5 w-3.5" />
        {t('dashboard.empty_action')}
      </button>
    </div>
  )
}
