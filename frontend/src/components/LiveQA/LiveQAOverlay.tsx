"use client";

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "@/i18n";
import { useConfig } from "@/contexts/ConfigContext";
import { Sparkles, Send, X, Loader2 } from "lucide-react";

interface LiveQASuggestion {
  text: string;
  rationale: string;
}

interface LiveQAResult {
  suggestions: LiveQASuggestion[];
  context_chars: number;
  model: string;
}

interface Props {
  meetingId: string | null;
}

export function LiveQAOverlay({ meetingId }: Props) {
  // §137.5: 拿用户当前 LLM provider + model (live_qa 用)
  const { modelConfig } = useConfig();

  const { t, locale } = useTranslation();
  const [open, setOpen] = useState(false);
  const [question, setQuestion] = useState("");
  const [asking, setAsking] = useState(false);
  const [result, setResult] = useState<LiveQAResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const isZh = locale === "zh";

  // ⌥+Space / Alt+Space global listener
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.altKey && (e.code === "Space" || e.key === " ")) {
        // 只在会议页生效, 避免与其它组件冲突
        if (!meetingId) return;
        e.preventDefault();
        setOpen((o) => !o);
        setResult(null);
        setError(null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [meetingId]);

  const submit = useCallback(async () => {
    if (!meetingId) return;
    const q = question.trim();
    if (!q) return;
    setAsking(true);
    setError(null);
    setResult(null);
    try {
      const r = (await invoke("api_meeting_live_qa", {
        meetingId,
        question: q,
        provider: modelConfig.provider,  // §137.5: 用用户选的 provider
        modelName: modelConfig.model,    // §137.5: 用用户选的 model_name
      })) as LiveQAResult;
      setResult(r);
    } catch (e) {
      const msg = typeof e === "string" ? e : (e as { message?: string })?.message ?? "unknown";
      setError(msg);
    } finally {
      setAsking(false);
    }
  }, [meetingId, question, modelConfig.provider, modelConfig.model]);

  if (!open || !meetingId) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-24 bg-black/40 backdrop-blur-sm"
      onClick={(e) => {
        if (e.target === e.currentTarget) setOpen(false);
      }}
      data-testid="live-qa-overlay"
    >
      <div className="w-full max-w-xl rounded-lg bg-white dark:bg-zinc-900 shadow-2xl border border-zinc-200 dark:border-zinc-800 p-4">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2 text-sm font-semibold text-zinc-700 dark:text-zinc-200">
            <Sparkles className="w-4 h-4 text-amber-500" />
            {isZh ? "实时会议助手" : "Live Meeting Q&A"}
          </div>
          <button
            onClick={() => setOpen(false)}
            className="text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100"
            aria-label="close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="flex gap-2 mb-3">
          <input
            autoFocus
            type="text"
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                void submit();
              } else if (e.key === "Escape") {
                setOpen(false);
              }
            }}
            placeholder={
              isZh
                ? "问个问题, 例如: 上次讨论过 API 限流吗?"
                : "Ask a question, e.g. did we discuss API rate limiting?"
            }
            className="flex-1 rounded-md border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-amber-400"
            data-testid="live-qa-input"
          />
          <button
            onClick={() => void submit()}
            disabled={asking || !question.trim()}
            className="rounded-md bg-amber-500 hover:bg-amber-600 disabled:opacity-50 text-white px-3 py-2 text-sm flex items-center gap-1"
            data-testid="live-qa-submit"
          >
            {asking ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Send className="w-4 h-4" />
            )}
          </button>
        </div>

        {error && (
          <div className="text-xs text-red-500 dark:text-red-400 mb-2">
            {isZh ? "失败: " : "Error: "}
            {error}
          </div>
        )}

        {result && (
          <div className="space-y-2" data-testid="live-qa-suggestions">
            <div className="text-xs text-zinc-500 dark:text-zinc-400">
              {isZh
                ? `基于最近 ${Math.round(result.context_chars / 100) * 100} 字上下文 · ${result.model}`
                : `From last ${result.context_chars} chars context · ${result.model}`}
            </div>
            <ol className="space-y-2">
              {result.suggestions.map((s, i) => (
                <li
                  key={i}
                  className="rounded-md border border-zinc-200 dark:border-zinc-700 p-3 text-sm"
                >
                  <div className="font-medium text-zinc-800 dark:text-zinc-100">
                    {i + 1}. {s.text}
                  </div>
                  {s.rationale && (
                    <div className="text-xs text-zinc-500 dark:text-zinc-400 mt-1">
                      {s.rationale}
                    </div>
                  )}
                </li>
              ))}
            </ol>
          </div>
        )}

        <div className="text-xs text-zinc-400 mt-3">
          {isZh
            ? "快捷键: ⌥+Space 开启/关闭 · Enter 提问 · Esc 退出"
            : "Hotkeys: ⌥+Space toggle · Enter ask · Esc close"}
        </div>
      </div>
    </div>
  );
}
