"use client";

import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, Save, Loader2, Search, FolderOpen, FileCode, FileText } from 'lucide-react';
import Analytics from '@/lib/analytics';
import { useTranslation } from '@/i18n';

interface SummaryUpdaterButtonGroupProps {
  isSaving: boolean;
  isDirty: boolean;
  onSave: () => Promise<void>;
  onCopy: () => Promise<void>;
  onExportMarkdown?: () => void;
  onExportTxt?: () => void;
  onFind?: () => void;
  onOpenFolder: () => Promise<void>;
  hasSummary: boolean;
}

export function SummaryUpdaterButtonGroup({
  isSaving,
  isDirty,
  onSave,
  onCopy,
  onExportMarkdown,
  onExportTxt,
  onFind,
  onOpenFolder,
  hasSummary
}: SummaryUpdaterButtonGroupProps) {
  const { t } = useTranslation();
  return (
    <ButtonGroup>
      {/* Save button */}
      <Button
        variant="outline"
        size="sm"
        className={`${isDirty ? 'bg-green-200' : ""}`}
        title={isSaving ? t('common.loading') : t('summary.save_changes')}
        onClick={() => {
          Analytics.trackButtonClick('save_changes', 'meeting_details');
          onSave();
        }}
        disabled={isSaving}
      >
        {isSaving ? (
          <>
            <Loader2 className="animate-spin" />
            <span className="hidden lg:inline">{t('common.loading')}</span>
          </>
        ) : (
          <>
            <Save />
            <span className="hidden lg:inline">{t('summary.save')}</span>
          </>
        )}
      </Button>

      {/* Copy button */}
      <Button
        variant="outline"
        size="sm"
        title={t('summary.copy')}
        onClick={() => {
          Analytics.trackButtonClick('copy_summary', 'meeting_details');
          onCopy();
        }}
        disabled={!hasSummary}
        className="cursor-pointer"
      >
        <Copy />
        <span className="hidden lg:inline">{t('summary.copy')}</span>
      </Button>

      {/* v0.6.15: Export MD/TXT buttons */}
      {onExportMarkdown && (
        <Button
          variant="outline"
          size="sm"
          title={t('summary.export_md_title')}
          onClick={() => {
            Analytics.trackButtonClick('export_summary_md', 'meeting_details');
            onExportMarkdown();
          }}
          disabled={!hasSummary}
          className="cursor-pointer"
        >
          <FileCode />
          <span className="hidden lg:inline">{t('summary.export_md')}</span>
        </Button>
      )}

      {onExportTxt && (
        <Button
          variant="outline"
          size="sm"
          title={t('summary.export_txt_title')}
          onClick={() => {
            Analytics.trackButtonClick('export_summary_txt', 'meeting_details');
            onExportTxt();
          }}
          disabled={!hasSummary}
          className="cursor-pointer"
        >
          <FileText />
          <span className="hidden lg:inline">{t('summary.export_txt')}</span>
        </Button>
      )}

      {/* Find button */}
      {/* {onFind && (
        <Button
          variant="outline"
          size="sm"
          title="在摘要中查找"
          onClick={() => {
            Analytics.trackButtonClick('find_in_summary', 'meeting_details');
            onFind();
          }}
          disabled={!hasSummary}
          className="cursor-pointer"
        >
          <Search />
          <span className="hidden lg:inline">Find</span>
        </Button>
      )} */}
    </ButtonGroup>
  );
}
