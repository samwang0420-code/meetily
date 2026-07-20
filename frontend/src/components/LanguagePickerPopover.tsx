"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import * as Popover from "@radix-ui/react-popover";
import { Check, Search } from "lucide-react";
import { LANGUAGE_OPTIONS } from "@/lib/summary-languages";
import { useRecentLanguages } from "@/hooks/useRecentLanguages";
import { useTranslation } from "@/i18n";

interface LanguagePickerPopoverProps {
  value: string | null;
  onChange: (code: string | null) => void;
  onClose: () => void;
  mode?: "meeting" | "settings";
  autoSubtitle?: string;
}

export function LanguagePickerPopover({
  value,
  onChange,
  onClose,
  mode = "meeting",
  autoSubtitle,
}: LanguagePickerPopoverProps) {
  const { t, locale } = useTranslation();
  const { recents } = useRecentLanguages();
  const [query, setQuery] = useState("");
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    const onDocClick = (e: MouseEvent) => {
      if (!containerRef.current?.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", onDocClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDocClick);
      document.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  const filter = query.trim().toLowerCase();

  const recentCodes = useMemo(() => new Set(recents), [recents]);

  const filteredAll = useMemo(() => {
    const options = mode === "meeting"
      ? LANGUAGE_OPTIONS.filter((l) => !recentCodes.has(l.code))
      : LANGUAGE_OPTIONS;
    if (!filter) return options;
    return options.filter(
      (l) =>
        l.code.toLowerCase().includes(filter) ||
        l.label.toLowerCase().includes(filter),
    );
  }, [filter, mode, recentCodes]);

  const recentsResolved = useMemo(
    () =>
      recents
        .map((code) => LANGUAGE_OPTIONS.find((l) => l.code === code))
        .filter((l): l is (typeof LANGUAGE_OPTIONS)[number] => Boolean(l))
        .filter(
          (l) =>
            !filter ||
            l.code.toLowerCase().includes(filter) ||
            l.label.toLowerCase().includes(filter),
        ),
    [recents, filter],
  );

  const showAuto = mode === "meeting" && (!filter || "auto".includes(filter));
  const showRecents = mode === "meeting" && recentsResolved.length > 0;
  const hasNoResults =
    filteredAll.length === 0 && recentsResolved.length === 0 && !showAuto;

  return (
    <div
      ref={containerRef}
      className="
        w-80 overflow-hidden rounded-lg
        border border-neutral-200/80 bg-white shadow-lg shadow-neutral-900/5
        backdrop-blur
        dark:border-neutral-800 dark:bg-neutral-900
      "
      role="dialog"
      aria-label={t('language_picker.aria_label')}
    >
      <div className="flex items-center gap-2 border-b border-neutral-100 px-3 py-2.5 dark:border-neutral-800">
        <Search className="h-4 w-4 text-neutral-400" strokeWidth={1.75} />
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t('language_picker.search_placeholder')}
          className="flex-1 border-none bg-transparent text-sm text-neutral-900 outline-none placeholder:text-neutral-400 dark:text-neutral-100"
        />
      </div>

      <div className="max-h-80 overflow-y-auto py-1">
        {showRecents && (
          <>
            <div className="px-3 pb-1 pt-1 text-[10px] font-semibold uppercase tracking-wider text-neutral-400">
              {t('language_picker.recently_used')}
            </div>
            {recentsResolved.map((opt) => {
              const active = value === opt.code;
              return (
                <button
                  key={`recent-${opt.code}`}
                  type="button"
                  aria-pressed={active}
                  onClick={() => onChange(opt.code)}
                  className={`
                    group flex w-full items-center justify-between px-3 py-1.5 text-left text-sm
                    transition-colors
                    ${active
                      ? 'bg-blue-50 text-blue-700 dark:bg-blue-500/15 dark:text-blue-300'
                      : 'text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800/60'}
                  `}
                >
                  <span className="flex flex-col">
                    <span className="font-medium">{opt.label}</span>
                    <span className="font-mono text-[11px] text-neutral-400">{opt.code}</span>
                  </span>
                  {active && <Check className="h-4 w-4 text-blue-600 dark:text-blue-400" strokeWidth={2.5} />}
                </button>
              );
            })}
            <div className="my-1 h-px bg-neutral-100 dark:bg-neutral-800" />
          </>
        )}

        {showAuto && (
          <button
            type="button"
            aria-pressed={value === null}
            onClick={() => onChange(null)}
            className={`
              group flex w-full items-center justify-between px-3 py-1.5 text-left text-sm
              transition-colors
              ${value === null
                ? 'bg-blue-50 text-blue-700 dark:bg-blue-500/15 dark:text-blue-300'
                : 'text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800/60'}
            `}
          >
            <span className="flex flex-col">
              <span className="font-medium">{t('language_picker.auto_detect')}</span>
              {autoSubtitle && (
                <span className="text-[11px] font-normal text-neutral-500 dark:text-neutral-400">{autoSubtitle}</span>
              )}
            </span>
            {value === null && <Check className="h-4 w-4 text-blue-600 dark:text-blue-400" strokeWidth={2.5} />}
          </button>
        )}

        {filteredAll.length > 0 && (
          <div className="px-3 pb-1 pt-1 text-[10px] font-semibold uppercase tracking-wider text-neutral-400">
            {mode === "meeting" ? t('language_picker.other_languages') : t('language_picker.all_languages')}
          </div>
        )}

        {filteredAll.map((opt) => {
          const active = value === opt.code;
          return (
            <button
              key={`all-${opt.code}`}
              type="button"
              aria-pressed={active}
              onClick={() => onChange(opt.code)}
              className={`
                group flex w-full items-center justify-between px-3 py-1.5 text-left text-sm
                transition-colors
                ${active
                  ? 'bg-blue-50 text-blue-700 dark:bg-blue-500/15 dark:text-blue-300'
                  : 'text-neutral-700 hover:bg-neutral-50 dark:text-neutral-300 dark:hover:bg-neutral-800/60'}
              `}
            >
              <span className="flex flex-col">
                <span className="font-medium">{opt.label}</span>
                <span className="font-mono text-[11px] text-neutral-400">{opt.code}</span>
              </span>
              {active && <Check className="h-4 w-4 text-blue-600 dark:text-blue-400" strokeWidth={2.5} />}
            </button>
          );
        })}

        {hasNoResults && (
          <div className="px-3 py-2 text-sm text-neutral-400">{t('language_picker.no_matches')}</div>
        )}
      </div>
    </div>
  );
}
