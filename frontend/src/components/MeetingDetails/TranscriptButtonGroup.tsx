"use client";

import { useState, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, FolderOpen, RefreshCw, FileText, FileCode } from 'lucide-react';
import Analytics from '@/lib/analytics';
import { RetranscribeDialog } from './RetranscribeDialog';
import { useConfig } from '@/contexts/ConfigContext';
import { useTranslation } from '@/i18n';


interface TranscriptButtonGroupProps {
  transcriptCount: number;
  onCopyTranscript: () => void;
  onExportMarkdown?: () => void;
  onExportTxt?: () => void;
  onOpenMeetingFolder: () => Promise<void>;
  meetingId?: string;
  meetingFolderPath?: string | null;
  onRefetchTranscripts?: () => Promise<void>;
}


export function TranscriptButtonGroup({
  transcriptCount,
  onCopyTranscript,
  onExportMarkdown,
  onExportTxt,
  onOpenMeetingFolder,
  meetingId,
  meetingFolderPath,
  onRefetchTranscripts,
}: TranscriptButtonGroupProps) {
  const { t } = useTranslation();
  const { betaFeatures } = useConfig();
  const [showRetranscribeDialog, setShowRetranscribeDialog] = useState(false);

  const handleRetranscribeComplete = useCallback(async () => {
    // Refetch transcripts to show the updated data
    if (onRefetchTranscripts) {
      await onRefetchTranscripts();
    }
  }, [onRefetchTranscripts]);

  return (
    <div className="flex items-center justify-center w-full gap-2">
      <ButtonGroup>
        <Button
          variant="outline"
          size="sm"
          onClick={() => {
            Analytics.trackButtonClick('copy_transcript', 'meeting_details');
            onCopyTranscript();
          }}
          disabled={transcriptCount === 0}
          title={transcriptCount === 0 ? t('meeting_details.no_transcript') : t('meeting_details.copy_transcript_title')}
        >
          <Copy />
          <span className="hidden lg:inline">{t('meeting_details.copy_transcript')}</span>
        </Button>

        {onExportMarkdown && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              Analytics.trackButtonClick('export_md', 'meeting_details');
              onExportMarkdown();
            }}
            disabled={transcriptCount === 0}
            title={t('meeting_details.export_md_title')}
          >
            <FileCode />
            <span className="hidden lg:inline">{t('meeting_details.export_md')}</span>
          </Button>
        )}

        {onExportTxt && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              Analytics.trackButtonClick('export_txt', 'meeting_details');
              onExportTxt();
            }}
            disabled={transcriptCount === 0}
            title={t('meeting_details.export_txt_title')}
          >
            <FileText />
            <span className="hidden lg:inline">{t('meeting_details.export_txt')}</span>
          </Button>
        )}

        <Button
          size="sm"
          variant="outline"
          className="xl:px-4"
          onClick={() => {
            Analytics.trackButtonClick('open_recording_folder', 'meeting_details');
            onOpenMeetingFolder();
          }}
          title={t('meeting_details.open_folder')} 
        >
          <FolderOpen className="xl:mr-2" size={18} />
          <span className="hidden lg:inline">{t('meeting_details.open_folder')}</span>
        </Button>

        {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
          <Button
            size="sm"
            variant="outline"
            className="bg-gradient-to-r from-blue-50 to-purple-50 hover:from-blue-100 hover:to-purple-100 border-blue-200 xl:px-4"
            onClick={() => {
              Analytics.trackButtonClick('enhance_transcript', 'meeting_details');
              setShowRetranscribeDialog(true);
            }}
            title={t('meeting_details.enhance_title')}
          >
            <RefreshCw className="xl:mr-2" size={18} />
            <span className="hidden lg:inline">{t('meeting_details.enhance')}</span>
          </Button>
        )}
      </ButtonGroup>

      {betaFeatures.importAndRetranscribe && meetingId && meetingFolderPath && (
        <RetranscribeDialog
          open={showRetranscribeDialog}
          onOpenChange={setShowRetranscribeDialog}
          meetingId={meetingId}
          meetingFolderPath={meetingFolderPath}
          onComplete={handleRetranscribeComplete}
        />
      )}
    </div>
  );
}
