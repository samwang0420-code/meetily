import { useCallback } from 'react';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { safeToast } from '@/lib/safeToast';
import { useTranslation } from '@/i18n';

interface UseMeetingOperationsProps {
  meeting: any;
}

export function useMeetingOperations({
  meeting,
}: UseMeetingOperationsProps) {
  const { t } = useTranslation();

  // Open meeting folder in file explorer
  const handleOpenMeetingFolder = useCallback(async () => {
    try {
      await invokeTauri('open_meeting_folder', { meetingId: meeting.id });
    } catch (error) {
      console.error('Failed to open meeting folder:', error);
      safeToast.error(typeof error === 'string' ? error : t('meeting_details.open_folder_failed'));
    }
  }, [meeting.id, t]);

  return {
    handleOpenMeetingFolder,
  };
}
