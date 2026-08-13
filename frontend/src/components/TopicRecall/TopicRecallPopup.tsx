"use client";

import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "@/i18n";
import { X, Sparkles, ArrowRight } from "lucide-react";

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

export function TopicRecallPopup() {
  const { t, locale } = useTranslation();
  const [open, setOpen] = useState(false);
  const [topics, setTopics] = useState<TopicSearchHit[]>([]);
  const [loading, setLoading] = useState(false);

  const isZh = locale === "zh";

  const fetchAndShow = useCallback(async () => {
    setLoading(true);
    try {
      // §91 P0-A: 新会议开始 → 拉最近高频 topic (mention_count > 0)
      const list = (await invoke("api_topic_recent", { limit: 5 })) as TopicSearchHit[];
      const relevant = list
        .filter((t) => t.mention_count >= 1 && (t.status === "open" || !t.status))
        .slice(0, 3);
      if (relevant.length > 0) {
        setTopics(relevant);
        setOpen(true);
      }
    } catch (e) {
      console.warn("topic recall fetch failed:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;
    (async () => {
      const fn = await listen<{ meeting_id?: string }>("recording-started", async (event) => {
        if (cancelled) return;
        if (!event.payload?.meeting_id) return;
        // 等 1s 让 DB session 落库再查
        setTimeout(() => {
          if (!cancelled) void fetchAndShow();
        }, 1000);
      });
      unlisten = fn;
    })();
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }, [fetchAndShow]);

  if (!open || topics.length === 0) return null;

  return (
    <div
      className="fixed top-4 right-4 z-40 max-w-md w-full rounded-lg bg-white dark:bg-zinc-900 shadow-2xl border border-blue-200 dark:border-blue-800 p-4"
      data-testid="topic-recall-popup"
    >
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2 text-sm font-semibold text-blue-700 dark:text-blue-300">
          <Sparkles className="w-4 h-4" />
          {isZh ? "上次讨论过的话题" : "Topics from past meetings"}
        </div>
        <button
          onClick={() => setOpen(false)}
          className="text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100"
          aria-label="close"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      <div className="space-y-2 mb-3">
        {topics.map((topic) => (
          <div
            key={topic.topic_id}
            className="p-2 rounded-md bg-blue-50 dark:bg-blue-950/30 border border-blue-100 dark:border-blue-900"
          >
            <div className="flex items-center justify-between text-sm">
              <span className="font-medium text-zinc-800 dark:text-zinc-100">
                {topic.canonical_name}
              </span>
              <span className="text-xs text-zinc-500">
                {isZh ? "提及" : "mentions"} {topic.mention_count}
              </span>
            </div>
            {topic.last_decided && (
              <div className="text-xs text-zinc-600 dark:text-zinc-400 mt-1">
                <span className="font-medium">
                  {isZh ? "上次决议:" : "Last decided:"}
                </span>{" "}
                {topic.last_decided}
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="text-xs text-zinc-500 mb-3">
        {isZh
          ? "新会议开始, 自动提示相关话题. 3 秒内可回顾上下文."
          : "New meeting started. Review related topics within 3 seconds."}
      </div>

      <div className="flex gap-2">
        <button
          onClick={() => setOpen(false)}
          className="flex-1 rounded-md border border-zinc-300 dark:border-zinc-700 px-3 py-1.5 text-sm hover:bg-zinc-50 dark:hover:bg-zinc-800"
        >
          {isZh ? "知道了" : "Got it"}
        </button>
        <button
          onClick={() => {
            // 跳转 topic search modal
            setOpen(false);
            window.dispatchEvent(new CustomEvent("open-topic-search"));
          }}
          className="flex-1 rounded-md bg-blue-500 hover:bg-blue-600 text-white px-3 py-1.5 text-sm flex items-center justify-center gap-1"
        >
          {isZh ? "查看全部" : "View all"}
          <ArrowRight className="w-3 h-3" />
        </button>
      </div>
    </div>
  );
}
