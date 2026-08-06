"use client";
import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useTranslation } from '@/i18n';
import { toast } from 'sonner';

interface ActionItem {
  id: number;
  meeting_id: string;
  item_index: number;
  content: string;
  done: boolean;
  created_at: string;
  updated_at: string;
}

interface Props {
  meetingId: string;
}

export function ActionItemsList({ meetingId }: Props) {
  const { t } = useTranslation();
  const [items, setItems] = useState<ActionItem[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const list = (await invoke('api_action_item_list', {
        meetingId,
      })) as ActionItem[];
      setItems(list);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast.error(t('summary.action_items_load_failed', { error: msg }));
    } finally {
      setLoading(false);
    }
  }, [meetingId, t]);

  useEffect(() => {
    void refresh();
    let un: UnlistenFn | undefined;
    (async () => {
      un = await listen('action-items-updated', (event) => {
        const payload = event.payload as { meeting_id?: string } | undefined;
        if (!payload || payload.meeting_id === meetingId) {
          void refresh();
        }
      });
    })();
    return () => {
      if (un) un();
    };
  }, [meetingId, refresh]);

  const toggle = useCallback(
    async (id: number, next: boolean) => {
      setItems((prev) =>
        prev.map((it) => (it.id === id ? { ...it, done: next } : it))
      );
      try {
        await invoke('api_action_item_toggle', { id, done: next });
      } catch (e) {
        // rollback on failure
        setItems((prev) =>
          prev.map((it) => (it.id === id ? { ...it, done: !next } : it))
        );
        const msg = e instanceof Error ? e.message : String(e);
        toast.error(t('summary.action_items_toggle_failed', { error: msg }));
      }
    },
    [t]
  );

  if (loading) {
    return <p className="text-sm text-gray-500">{t('common.loading')}</p>;
  }
  if (items.length === 0) {
    return (
      <p className="text-sm text-gray-500">{t('summary.action_items_empty')}</p>
    );
  }
  return (
    <ul className="space-y-2 pl-0">
      {items.map((item) => (
        <li
          key={item.id}
          className="flex items-start gap-2 text-sm leading-snug"
        >
          <input
            type="checkbox"
            checked={item.done}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              void toggle(item.id, e.target.checked)
            }
            aria-label={t('summary.action_item_done_aria', {
              content: item.content,
            })}
            data-testid="action-item-checkbox"
            className="mt-0.5 h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500 cursor-pointer"
          />
          <span
            className={
              item.done
                ? 'line-through text-gray-400 select-none'
                : 'text-gray-800'
            }
          >
            {item.content}
          </span>
        </li>
      ))}
    </ul>
  );
}
