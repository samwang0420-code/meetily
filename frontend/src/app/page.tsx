'use client';

import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { RecordingControls } from '@/components/RecordingControls';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import { useAuth } from '@/contexts/AuthContext';
import { usePermissionCheck } from '@/hooks/usePermissionCheck';
import { useRecordingState, RecordingStatus } from '@/contexts/RecordingStateContext';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useConfig } from '@/contexts/ConfigContext';
import { StatusOverlays } from '@/app/_components/StatusOverlays';
import Analytics from '@/lib/analytics';
import { SettingsModals } from './_components/SettingsModal';
import { TranscriptPanel } from './_components/TranscriptPanel';
import { HomeDashboard } from './_components/HomeDashboard';
import { useModalState } from '@/hooks/useModalState';
import { useRecordingStateSync } from '@/hooks/useRecordingStateSync';
import { useRecordingStart } from '@/hooks/useRecordingStart';
import { useRecordingStop } from '@/hooks/useRecordingStop';
import { useTranscriptRecovery } from '@/hooks/useTranscriptRecovery';
import { useQuota } from '@/hooks/useQuota';
import { QuotaPaywallModal } from './_components/QuotaPaywallModal';
import { TranscriptRecovery } from '@/components/TranscriptRecovery';
import { indexedDBService } from '@/services/indexedDBService';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'next/navigation';

export default function Home() {
  // Local page state (not moved to contexts)
  const [isRecording, setIsRecordingState] = useState(false);
  const [barHeights, setBarHeights] = useState(['58%', '76%', '58%']);
  const [showRecoveryDialog, setShowRecoveryDialog] = useState(false);

  // Use contexts for state management
  const { meetingTitle } = useTranscripts();
  const { transcriptModelConfig, selectedDevices } = useConfig();
  const recordingState = useRecordingState();

  // Extract status from global state
  const { status, isStopping, isProcessing, isSaving } = recordingState;

  // Hooks
  const { hasMicrophone } = usePermissionCheck();
  // v0.6.10+: 拿 session 用于 quota 检查
  const { session } = useAuth();
  const { setIsMeetingActive, isCollapsed: sidebarCollapsed, refetchMeetings } = useSidebar();
  const { modals, messages, showModal, hideModal } = useModalState(transcriptModelConfig);
  const { isRecordingDisabled, setIsRecordingDisabled } = useRecordingStateSync(isRecording, setIsRecordingState, setIsMeetingActive);
  const { handleRecordingStart } = useRecordingStart(isRecording, setIsRecordingState, showModal);

  // v0.6.10+: 商业化配额 (C1+C2)
  // - 未登录: 试用 1 次, 用完弹"请注册"
  // - free 注册: 每月 5 次录音
  // - member: 无上限
  const { quota, refresh: refreshQuota, recordAfterSave } = useQuota(session ?? null);

  const [paywall, setPaywall] = useState<{ open: boolean; reason: 'anonymous_trial_exhausted' | 'free_monthly_limit_reached' | null }>({ open: false, reason: null });

  const startRecordingWithQuota = async () => {
    if (!quota.can_record) {
      const isAnon = quota.tier === 'anonymous';
      setPaywall({ open: true, reason: isAnon ? 'anonymous_trial_exhausted' : 'free_monthly_limit_reached' });
      return;
    }
    await handleRecordingStart();
  };

  const closePaywall = () => setPaywall({ open: false, reason: null });

  const recordLead = async () => {
    try {
      const userEmail = (typeof window !== 'undefined') ? (window.localStorage.getItem('lixianhuiji.last_email') || '') : '';
      if (!userEmail) {
        alert('请先在 Account 页输入您的邮箱');
        return;
      }
      await invoke('lead_record_upgrade', { email: userEmail, contact: userEmail, note: 'In-app paywall click' });
      safeToast.success('已记录您的升级意向, 客服会通过邮件联系您。');
    } catch (e) {
      console.warn(e);
      safeToast.error('记录失败,请稍后重试');
    }
  };

  // Get handleRecordingStop function and setIsStopping (state comes from global context)
  const { handleRecordingStop, setIsStopping } = useRecordingStop(
    setIsRecordingState,
    setIsRecordingDisabled
  );

  // Recovery hook
  const {
    recoverableMeetings,
    isLoading: isLoadingRecovery,
    isRecovering,
    checkForRecoverableTranscripts,
    recoverMeeting,
    loadMeetingTranscripts,
    deleteRecoverableMeeting
  } = useTranscriptRecovery();

  const router = useRouter();

  useEffect(() => {
    console.log('[v0.6-debug] page.tsx mounted, hand status');
    // Track page view
    Analytics.trackPageView('Home');
    console.log('[v0.6-debug] trackPageView OK');
  }, []);

  // Startup recovery check
  useEffect(() => {
    const performStartupChecks = async () => {
      try {
        // Skip recovery check if currently recording or processing stop
        // This prevents the recovery dialog from showing when:
        if (recordingState.isRecording ||
          status === RecordingStatus.STOPPING ||
          status === RecordingStatus.PROCESSING_TRANSCRIPTS ||
          status === RecordingStatus.SAVING) {
          console.log('Skipping recovery check - recording in progress or processing');
          return;
        }

        // 1. Clean up old meetings (7+ days)
        try {
          await indexedDBService.deleteOldMeetings(7);
        } catch (error) {
          console.warn('⚠️ Failed to clean up old meetings:', error);
        }

        // 2. Clean up saved meetings (24+ hours after save)
        try {
          await indexedDBService.deleteSavedMeetings(24);
        } catch (error) {
          console.warn('⚠️ Failed to clean up saved meetings:', error);
        }

        // 3. Always check for recoverable meetings on startup
        // Don't skip based on sessionStorage - we need to check every time
        await checkForRecoverableTranscripts();
      } catch (error) {
        console.error('Failed to perform startup checks:', error);
      }
    };

    performStartupChecks();
  }, [checkForRecoverableTranscripts, recordingState.isRecording, status]);

  // Watch for recoverable meetings changes and show dialog once per session
  useEffect(() => {
    // Only show dialog if we have meetings and haven't shown it yet this session
    if (recoverableMeetings.length > 0) {
      const shownThisSession = sessionStorage.getItem('recovery_dialog_shown');
      if (!shownThisSession) {
        setShowRecoveryDialog(true);
        sessionStorage.setItem('recovery_dialog_shown', 'true');
      }
    }
  }, [recoverableMeetings]);

  // Handle recovery with toast notifications and navigation
  const handleRecovery = async (meetingId: string) => {
    try {
      const result = await recoverMeeting(meetingId);

      if (result.success) {
        safeToast.success('会议恢复成功！', {
          description: result.audioRecoveryStatus?.status === 'success'
            ? '转录和音频已恢复'
            : '转录已恢复 (无可用音频)',
          action: result.meetingId ? {
            label: '查看会议',
            onClick: () => {
              router.push(`/meeting-details?id=${result.meetingId}`);
            }
          } : undefined,
          duration: 10000,
        });

        // Refresh sidebar to show the newly recovered meeting
        await refetchMeetings();

        // If no more recoverable meetings, clear session flag so dialog can show again
        if (recoverableMeetings.length === 0) {
          sessionStorage.removeItem('recovery_dialog_shown');
        }

        // Auto-navigate after a short delay
        if (result.meetingId) {
          setTimeout(() => {
            router.push(`/meeting-details?id=${result.meetingId}`);
          }, 2000);
        }
      }
    } catch (error) {
      safeToast.error('恢复会议失败', {
        description: error instanceof Error ? error.message : 'Unknown error occurred',
      });
      throw error;
    }
  };

  // Handle dialog close - clear session flag if no meetings left
  const handleDialogClose = () => {
    setShowRecoveryDialog(false);
    // If user closes dialog and there are no more meetings, clear the flag
    // This allows the dialog to show again next session if new meetings appear
    if (recoverableMeetings.length === 0) {
      sessionStorage.removeItem('recovery_dialog_shown');
    }
  };

  useEffect(() => {
    if (recordingState.isRecording) {
      const interval = setInterval(() => {
        setBarHeights(prev => {
          const newHeights = [...prev];
          newHeights[0] = Math.random() * 20 + 10 + 'px';
          newHeights[1] = Math.random() * 20 + 10 + 'px';
          newHeights[2] = Math.random() * 20 + 10 + 'px';
          return newHeights;
        });
      }, 300);

      return () => clearInterval(interval);
    }
  }, [recordingState.isRecording]);

  // Computed values using global status
  const isProcessingStop = status === RecordingStatus.PROCESSING_TRANSCRIPTS || isProcessing;

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: 'easeOut' }}
      className="flex flex-col h-screen bg-gray-50"
    >
      {/* All Modals supported*/}
      <SettingsModals
        modals={modals}
        messages={messages}
        onClose={hideModal}
      />

      {/* Recovery Dialog */}
      <TranscriptRecovery
        isOpen={showRecoveryDialog}
        onClose={handleDialogClose}
        recoverableMeetings={recoverableMeetings}
        onRecover={handleRecovery}
        onDelete={deleteRecoverableMeeting}
        onLoadPreview={loadMeetingTranscripts}
      />
      <div className="flex flex-1 overflow-hidden">
        {recordingState.isRecording ? (
          <TranscriptPanel
            isProcessingStop={isProcessingStop}
            isStopping={isStopping}
            showModal={showModal}
          />
        ) : (
          <HomeDashboard
            onRecordingStart={startRecordingWithQuota}
            onTranscriptReceived={() => {}}
            onTranscriptionError={(message: string) => showModal('errorAlert', message)}
            isRecordingDisabled={isRecordingDisabled}
            isParentProcessing={isProcessingStop}
            barHeights={barHeights}
            showModal={(type: string, message?: string) => showModal(type as any, message)}
            meetingTitle={meetingTitle}
            selectedDevices={selectedDevices}
          />
        )}

        {/* Recording controls - 录音中底部药丸 (原有设计) */}
        {recordingState.isRecording && (
          <div className="fixed bottom-12 left-0 right-0 z-10">
            <div
              className="flex justify-center pl-8 transition-[margin] duration-300"
              style={{
                marginLeft: sidebarCollapsed ? '4rem' : '16rem'
              }}
            >
              <div className="w-2/3 max-w-[750px] flex justify-center">
                <div className="bg-white rounded-full shadow-lg flex items-center">
                  <RecordingControls
                    isRecording={recordingState.isRecording}
                    onRecordingStop={(callApi = true) => handleRecordingStop(callApi)}
                    onRecordingStart={startRecordingWithQuota}
                    onTranscriptReceived={() => { }}
                    onStopInitiated={() => setIsStopping(true)}
                    barHeights={barHeights}
                    onTranscriptionError={(message) => {
                      showModal('errorAlert', message);
                    }}
                    isRecordingDisabled={isRecordingDisabled}
                    isParentProcessing={isProcessingStop}
                    selectedDevices={selectedDevices}
                    meetingName={meetingTitle}
                  />
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Status Overlays - Processing and Saving */}
        <StatusOverlays
          isProcessing={status === RecordingStatus.PROCESSING_TRANSCRIPTS && !recordingState.isRecording}
          isSaving={status === RecordingStatus.SAVING}
          sidebarCollapsed={sidebarCollapsed}
        />
      </div>

      {/* v0.6.10+: 商业化付费墙弹窗 (C1+C2) */}
      <QuotaPaywallModal
        open={paywall.open}
        reason={paywall.reason}
        onClose={closePaywall}
        onUpgradeInterest={recordLead}
      />
    </motion.div>
  );
}
