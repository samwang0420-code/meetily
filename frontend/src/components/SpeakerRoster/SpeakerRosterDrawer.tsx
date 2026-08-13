"use client";

import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from '@/i18n';
import { Users, X } from 'lucide-react';
import { toast } from 'sonner';

interface SpeakerAlias {
  id: number;
  meeting_id: string;
  speaker_id: number;
  label: string;
  created_at: string;
  updated_at: string;
}

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  meetingId: string;
}

export function SpeakerRosterDrawer({ open, onOpenChange, meetingId }: Props) {
  const { t } = useTranslation();
  const [aliases, setAliases] = useState<SpeakerAlias[]>([]);
  const [draft, setDraft] = useState<Record<number, string>>({});
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState<number | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const list = (await invoke('api_speaker_alias_list', { meetingId })) as SpeakerAlias[];
      setAliases(list);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error(t('speaker.load_failed', { error: msg }));
    } finally {
      setLoading(false);
    }
  }, [meetingId, t]);

  useEffect(() => {
    if (open) {
      void refresh();
      setDraft({});
    }
  }, [open, refresh]);

  const save = useCallback(
    async (speakerId: number, label: string) => {
      setSaving(speakerId);
      try {
        await invoke('api_speaker_alias_set', {
          meetingId,
          speakerId,
          label,
        });
        await refresh();
        toast.success(t('speaker.save_success', { speaker: label }));
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        toast.error(t('speaker.save_failed', { error: msg }));
      } finally {
        setSaving(null);
      }
    },
    [meetingId, refresh, t]
  );

  const remove = useCallback(
    async (speakerId: number) => {
      try {
        await invoke('api_speaker_alias_set', {
          meetingId,
          speakerId,
          label: '',
        });
        await refresh();
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        toast.error(t('speaker.delete_failed', { error: msg }));
      }
    },
    [meetingId, refresh, t]
  );

  if (!open) return null;

  const slots = [0, 1, 2, 3, 4];

  return (
    <div
      className="fixed inset-0 z-40 flex justify-end bg-black/30 backdrop-blur-sm"
      onClick={() => onOpenChange(false)}
      role="dialog"
      aria-modal="true"
      data-testid="speaker-roster-drawer"
    >
      <div
        className="bg-white dark:bg-neutral-900 w-96 max-w-full h-full flex flex-col border-l border-neutral-200 dark:border-neutral-800"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-3 px-5 py-4 border-b border-neutral-200 dark:border-neutral-800">
          <Users className="w-5 h-5 text-neutral-500" />
          <h3 className="text-base font-semibold flex-1">{t('speaker.title')}</h3>
          <button
            onClick={() => onOpenChange(false)}
            className="text-neutral-500 hover:text-neutral-700"
            aria-label={t('common.close')}
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="px-5 py-3 text-xs text-neutral-500 leading-relaxed border-b border-neutral-100">
          {t('speaker.hint')}
        </div>

        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-4">
          {loading && (
            <div className="text-sm text-neutral-500">{t('common.loading')}</div>
          )}
          {!loading &&
            slots.map((sid) => {
              const existing = aliases.find((a) => a.speaker_id === sid);
              const value = draft[sid] ?? existing?.label ?? '';
              return (
                <div
                  key={sid}
                  className="border border-neutral-200 dark:border-neutral-700 rounded-lg p-3"
                  data-testid={`speaker-alias-slot-${sid}`}
                >
                  <div className="flex items-center gap-2 mb-2">
                    <span className="font-mono text-xs px-1.5 py-0.5 rounded bg-neutral-200 dark:bg-neutral-700 text-neutral-600 dark:text-neutral-300">
                      speaker_{sid.toString().padStart(2, '0')}
                    </span>
                    {existing && (
                      <span className="ml-auto text-[10px] uppercase tracking-wide text-emerald-600">
                        {t('speaker.saved')}
                      </span>
                    )}
                  </div>
                  <input
                    type="text"
                    value={value}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                      setDraft((prev) => ({ ...prev, [sid]: e.target.value }))
                    }
                    placeholder={t('speaker.label_placeholder')}
                    className="w-full text-sm px-3 py-2 rounded border border-neutral-300 dark:border-neutral-700 bg-transparent outline-none focus:ring-1 focus:ring-blue-500"
                    data-testid={`speaker-alias-input-${sid}`}
                  />
                  <div className="flex gap-2 mt-2">
                    <button
                      onClick={() => void save(sid, value)}
                      disabled={saving === sid || !value.trim()}
                      className="flex-1 text-xs px-3 py-1.5 rounded bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white"
                    >
                      {saving === sid ? t('speaker.saving') : t('speaker.save')}
                    </button>
                    {existing && (
                      <button
                        onClick={() => void remove(sid)}
                        className="text-xs px-3 py-1.5 rounded border border-neutral-300 hover:bg-neutral-100"
                      >
                        {t('speaker.clear')}
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          {!loading && aliases.length === 0 && (
            <div className="text-sm text-neutral-500 text-center py-6">
              {t('speaker.empty')}
            </div>
          )}
        </div>

        <div className="px-5 py-3 border-t border-neutral-200 dark:border-neutral-800 text-xs text-neutral-400">
          {t('speaker.footer_note')}
        </div>
      </div>
    </div>
  );
}
