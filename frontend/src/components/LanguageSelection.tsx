'use client';

import React, { useState } from 'react';
import * as Popover from '@radix-ui/react-popover';
import { Globe, Check, ChevronDown, AlertTriangle, Info } from 'lucide-react';
import Analytics from '@/lib/analytics';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import { useConfig } from '@/contexts/ConfigContext';
import { useTranslation } from '@/i18n';
import { LANGUAGES, displayLanguage } from '@/constants/languages';

export type Language = typeof LANGUAGES[number];

interface LanguageSelectionProps {
  selectedLanguage: string;
  onLanguageChange: (language: string) => void;
  disabled?: boolean;
  provider?: 'localWhisper' | 'parakeet' | 'deepgram' | 'elevenLabs' | 'groq' | 'openai' | 'sherpa_paraformer' | 'sherpa_funasr_nano';
}

export function LanguageSelection({
  selectedLanguage,
  onLanguageChange,
  disabled = false,
  provider = 'localWhisper',
}: LanguageSelectionProps) {
  const [saving, setSaving] = useState(false);
  const [open, setOpen] = useState(false);
  const { setSelectedLanguage } = useConfig();
  const { t, locale } = useTranslation();

  // Parakeet only supports auto-detection
  const isParakeet = provider === 'parakeet';
  const availableLanguages = isParakeet
    ? LANGUAGES.filter(lang => lang.code === 'auto' || lang.code === 'auto-translate')
    : LANGUAGES;

  const handleLanguageChange = async (languageCode: string) => {
    setSaving(true);
    try {
      setSelectedLanguage(languageCode);
      onLanguageChange(languageCode);
      console.log('Language preference saved:', languageCode);

      const selectedLang = LANGUAGES.find(lang => lang.code === languageCode);
      await Analytics.track('language_selected', {
        language_code: languageCode,
        language_name: selectedLang?.name || 'Unknown',
        is_auto_detect: (languageCode === 'auto').toString(),
        is_auto_translate: (languageCode === 'auto-translate').toString(),
      });

      const displayName = displayLanguage(languageCode, locale);
      safeToast.success(t('language_selection.saved_toast'), {
        description: t('language_selection.saved_desc', { name: displayName }),
      });
      setOpen(false);
    } catch (error) {
      console.error('Failed to save language preference:', error);
      safeToast.error(t('language_selection.save_failed'), {
        description: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setSaving(false);
    }
  };

  const selectedLanguageName = displayLanguage(selectedLanguage, locale);

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <Globe className="h-4 w-4 text-neutral-500" strokeWidth={1.75} />
        <h4 className="text-sm font-medium text-neutral-900 dark:text-neutral-100">
          {t('language_selection.title')}
        </h4>
      </div>

      <Popover.Root open={open} onOpenChange={(v) => !disabled && !saving && setOpen(v)}>
        <Popover.Trigger asChild>
          <button
            disabled={disabled || saving}
            className="
              group flex h-10 w-full items-center justify-between gap-2 rounded-md
              border border-neutral-200 bg-white px-3 text-sm
              transition-colors
              hover:border-neutral-300 hover:bg-neutral-50
              focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40
              disabled:cursor-not-allowed disabled:opacity-50
              dark:border-neutral-800 dark:bg-neutral-900 dark:hover:border-neutral-700 dark:hover:bg-neutral-800
            "
          >
            <span className="flex flex-col items-start text-left">
              <span className="text-[13px] font-medium text-neutral-900 dark:text-neutral-100">
                {selectedLanguageName}
              </span>
              <span className="text-[11px] text-neutral-500 dark:text-neutral-400">
                {selectedLanguage === 'auto' ? t('language_selection.auto_hint') : selectedLanguage === 'auto-translate' ? t('language_selection.translate_hint') : t('language_selection.specific_hint', { name: selectedLanguageName })}
              </span>
            </span>
            <ChevronDown
              className={`h-4 w-4 text-neutral-400 transition-transform ${open ? 'rotate-180' : ''}`}
              strokeWidth={1.75}
            />
          </button>
        </Popover.Trigger>
        <Popover.Portal>
          <Popover.Content
            align="start"
            sideOffset={6}
            className="
              z-50 w-[var(--radix-popover-trigger-width)] overflow-hidden rounded-lg
              border border-neutral-200/80 bg-white p-1 shadow-lg shadow-neutral-900/5
              backdrop-blur
              dark:border-neutral-800 dark:bg-neutral-900
            "
          >
            <div className="px-3 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-wider text-neutral-400">
              {t('language_selection.popover_label')}
            </div>
            {availableLanguages.map((language) => {
              const active = language.code === selectedLanguage;
              const display = displayLanguage(language.code, locale);
              return (
                <button
                  key={language.code}
                  onClick={() => handleLanguageChange(language.code)}
                  disabled={saving}
                  className={`
                    flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-sm
                    transition-colors
                    ${active
                      ? 'bg-blue-50 text-blue-700 dark:bg-blue-500/15 dark:text-blue-300'
                      : 'text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800'}
                    disabled:opacity-50
                  `}
                >
                  <span className="flex flex-col items-start text-left">
                    <span className="text-[13px] font-medium">{display}</span>
                    {language.code !== 'auto' && language.code !== 'auto-translate' && (
                      <span className="text-[11px] text-neutral-500 dark:text-neutral-400 font-mono">
                        {language.code}
                      </span>
                    )}
                  </span>
                  {active && <Check className="h-4 w-4 text-blue-600 dark:text-blue-400" strokeWidth={2.5} />}
                </button>
              );
            })}
          </Popover.Content>
        </Popover.Portal>
      </Popover.Root>

      {/* Parakeet warning */}
      {isParakeet && (
        <div className="flex gap-2 rounded-md border border-amber-200 bg-amber-50 p-2.5 text-amber-900 dark:border-amber-900/50 dark:bg-amber-950/30 dark:text-amber-200">
          <Info className="h-4 w-4 mt-0.5 shrink-0" strokeWidth={1.75} />
          <div className="space-y-0.5 text-xs">
            <p className="font-medium">{t('language_selection.parakeet_title')}</p>
            <p className="text-amber-800/80 dark:text-amber-300/80">{t('language_selection.parakeet_desc')}</p>
          </div>
        </div>
      )}

      {/* Mode hints */}
      {selectedLanguage === 'auto' && (
        <div className="flex gap-2 rounded-md border border-yellow-200 bg-yellow-50 p-2.5 text-yellow-900 dark:border-yellow-900/50 dark:bg-yellow-950/30 dark:text-yellow-200">
          <AlertTriangle className="h-4 w-4 mt-0.5 shrink-0" strokeWidth={1.75} />
          <div className="space-y-0.5 text-xs">
            <p className="font-medium">{t('language_selection.auto_warn_title')}</p>
            <p className="text-yellow-800/80 dark:text-yellow-300/80">{t('language_selection.auto_warn_desc')}</p>
          </div>
        </div>
      )}
      {selectedLanguage === 'auto-translate' && (
        <div className="flex gap-2 rounded-md border border-blue-200 bg-blue-50 p-2.5 text-blue-900 dark:border-blue-900/50 dark:bg-blue-950/30 dark:text-blue-200">
          <Info className="h-4 w-4 mt-0.5 shrink-0" strokeWidth={1.75} />
          <div className="space-y-0.5 text-xs">
            <p className="font-medium">{t('language_selection.translate_mode_title')}</p>
            <p className="text-blue-800/80 dark:text-blue-300/80">{t('language_selection.translate_mode_desc')}</p>
          </div>
        </div>
      )}
      {selectedLanguage !== 'auto' && selectedLanguage !== 'auto-translate' && (
        <p className="text-xs text-neutral-500 dark:text-neutral-400">
          {t('language_selection.specific_desc', { name: selectedLanguageName })}
        </p>
      )}
    </div>
  );
}
