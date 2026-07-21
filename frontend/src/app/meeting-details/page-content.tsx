"use client";
import { useState, useEffect, useRef } from 'react';
import { useRouter } from 'next/navigation';
import { ArrowLeft } from 'lucide-react';
import { listen } from '@tauri-apps/api/event';
import { motion } from 'framer-motion';
import { Summary, SummaryResponse } from '@/types';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import Analytics from '@/lib/analytics';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import { TranscriptPanel } from '@/components/MeetingDetails/TranscriptPanel';
import { SummaryPanel } from '@/components/MeetingDetails/SummaryPanel';
import { ModelConfig } from '@/components/ModelSettingsModal';

// Custom hooks
import { useMeetingData } from '@/hooks/meeting-details/useMeetingData';
import { useSummaryGeneration } from '@/hooks/meeting-details/useSummaryGeneration';
import { useTemplates } from '@/hooks/meeting-details/useTemplates';
import { useCopyOperations } from '@/hooks/meeting-details/useCopyOperations';
import { useMeetingOperations } from '@/hooks/meeting-details/useMeetingOperations';
import { useConfig } from '@/contexts/ConfigContext';
import { useTranslation } from '@/i18n';

export default function PageContent({
  meeting,
  summaryData,
  shouldAutoGenerate = false,
  onAutoGenerateComplete,
  onMeetingUpdated,
  onRefetchTranscripts,
  // Pagination props for efficient transcript loading
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
}: {
  meeting: any;
  summaryData: Summary | null;
  shouldAutoGenerate?: boolean;
  onAutoGenerateComplete?: () => void;
  onMeetingUpdated?: () => Promise<void>;
  onRefetchTranscripts?: () => Promise<void>;
  // Pagination props
  segments?: any[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;
}) {
  const { t } = useTranslation();
  console.log('📄 PAGE CONTENT: Initializing with data:', {
    meetingId: meeting.id,
    summaryDataKeys: summaryData ? Object.keys(summaryData) : null,
    transcriptsCount: meeting.transcripts?.length
  });

  // State
  const [customPrompt, setCustomPrompt] = useState<string>('');
  const router = useRouter();
  const [isRecording] = useState(false);
  const [summaryResponse] = useState<SummaryResponse | null>(null);

  // Ref to store the modal open function from SummaryGeneratorButtonGroup
  const openModelSettingsRef = useRef<(() => void) | null>(null);

  // Sidebar context
  const { serverAddress } = useSidebar();

  // Get model config from ConfigContext
  const { modelConfig, setModelConfig } = useConfig();

  // Custom hooks
  const meetingData = useMeetingData({ meeting, summaryData, onMeetingUpdated });
  const templates = useTemplates();

  // Callback to register the modal open function
  const handleRegisterModalOpen = (openFn: () => void) => {
    console.log('📝 Registering modal open function in PageContent');
    openModelSettingsRef.current = openFn;
  };

  // Callback to trigger modal open (called from error handler)
  const handleOpenModelSettings = () => {
    console.log('🔔 Opening model settings from PageContent');
    if (openModelSettingsRef.current) {
      openModelSettingsRef.current();
    } else {
      console.warn('⚠️ Modal open function not yet registered');
    }
  };

  // Save model config to backend database and sync via event
  const handleSaveModelConfig = async (config?: ModelConfig) => {
    if (!config) return;
    try {
      await invoke('api_save_model_config', {
        provider: config.provider,
        model: config.model,
        whisperModel: config.whisperModel,
        apiKey: config.apiKey ?? null,
        ollamaEndpoint: config.ollamaEndpoint ?? null,
      });

      // Emit event so ConfigContext and other listeners stay in sync
      const { emit } = await import('@tauri-apps/api/event');
      await emit('model-config-updated', config);

      safeToast.success('模型设置已保存');
    } catch (error) {
      console.error('Failed to save model config:', error);
      safeToast.error('保存模型设置失败');
    }
  };

  const summaryGeneration = useSummaryGeneration({
    meeting,
    transcripts: meetingData.transcripts,
    modelConfig: modelConfig,
    isModelConfigLoading: false, // ConfigContext loads on mount
    selectedTemplate: templates.selectedTemplate,
    onMeetingUpdated,
    updateMeetingTitle: meetingData.updateMeetingTitle,
    setAiSummary: meetingData.setAiSummary,
    onOpenModelSettings: handleOpenModelSettings,
  });

  const copyOperations = useCopyOperations({
    meeting,
    transcripts: meetingData.transcripts,
    meetingTitle: meetingData.meetingTitle,
    aiSummary: meetingData.aiSummary,
    blockNoteSummaryRef: meetingData.blockNoteSummaryRef,
  });

  const meetingOperations = useMeetingOperations({
    meeting,
  });

  // Track page view
  useEffect(() => {
    Analytics.trackPageView('meeting_details');
  }, []);

  // v0.6.10+: 监听录音后自动重新转录的进度, complete 时刷新 transcript
  useEffect(() => {
    let unlistenFn: (() => void) | undefined;
    const setup = async () => {
      try {
        unlistenFn = await listen<{
          meeting_id: string;
          stage: string;
          progress_percentage: number;
          message: string;
        }>('retranscription-progress', async (event) => {
          // 仅当属于当前 meeting
          if (event.payload.meeting_id !== meeting.id) return;
          const { stage, progress_percentage, message } = event.payload;
          if (stage === 'complete') {
            safeToast.success(t('meeting_details.retranscription_complete'), {
              description: t('meeting_details.retranscription_complete_desc'),
              duration: 5000,
            });
            // 触发父级 refetch transcripts
            if (onRefetchTranscripts) {
              try {
                await onRefetchTranscripts();
                console.log('retranscription-complete: transcripts refreshed');
              } catch (e) {
                console.warn('重新转录完成后 transcript 刷新失败:', e);
              }
            }
          } else if (stage === 'saving' || stage === 'transcribing') {
            // 中间阶段 toast 不刷屏, 用 console
            console.log(`[retranscription ${progress_percentage}%] ${stage}: ${message}`);
          }
        });
      } catch (e) {
        console.warn('注册 retranscription-progress 监听失败 (非阻塞):', e);
      }
    };
    setup();
    return () => { if (unlistenFn) unlistenFn(); };
  }, [meeting.id, onRefetchTranscripts]);

  // v0.7.0+ P0-2: 听 transcripts-updated 事件 (diar_pickup_loop 后台定时触发).
  // 当回填了当前 meeting 的 speaker 字段时, 自动 refetch transcripts 让 UI 立刻更新.
  useEffect(() => {
    let unlistenFn: (() => void) | undefined;
    const setup = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlistenFn = await listen<{ meeting_ids: string[]; source: string }>(
          'transcripts-updated',
          (event) => {
            if (event.payload.meeting_ids && event.payload.meeting_ids.includes(meeting.id)) {
              if (onRefetchTranscripts) {
                void onRefetchTranscripts();
              }
            }
          }
        );
      } catch (e) {
        console.warn('注册 transcripts-updated 监听失败 (非阻塞):', e);
      }
    };
    setup();
    return () => { if (unlistenFn) unlistenFn(); };
  }, [meeting.id, onRefetchTranscripts]);

  // Auto-generate summary when flag is set
  useEffect(() => {
    let cancelled = false;

    const autoGenerate = async () => {
      if (shouldAutoGenerate && meetingData.transcripts.length > 0 && !cancelled && summaryGeneration.summaryStatus === 'idle') {
        console.log(`🤖 Auto-generating summary with ${modelConfig.provider}/${modelConfig.model}...`);
        await summaryGeneration.handleGenerateSummary('');

        // Notify parent that auto-generation is complete (only if not cancelled)
        if (onAutoGenerateComplete && !cancelled) {
          onAutoGenerateComplete();
        }
      }
    };

    autoGenerate();

    // Cleanup: cancel if component unmounts or meeting changes
    return () => {
      cancelled = true;
    };
  }, [shouldAutoGenerate, meeting.id, meetingData.transcripts.length, summaryGeneration.summaryStatus, summaryGeneration.handleGenerateSummary, onAutoGenerateComplete]);

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: 'easeOut' }}
      className="flex flex-col h-screen bg-gray-50"
    >
      {/* 顶部固定栏: 返回工作台 */}
      <div className="flex items-center gap-3 px-6 py-3 border-b border-gray-200 bg-white">
        <button
          onClick={() => router.push('/')}
          className="flex items-center gap-1 text-gray-600 hover:text-gray-900 text-sm"
          title="返回工作台"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>返回工作台</span>
        </button>
      </div>

      <div className="flex flex-1 overflow-hidden">
        <TranscriptPanel
          transcripts={meetingData.transcripts}
          customPrompt={customPrompt}
          onPromptChange={setCustomPrompt}
          onCopyTranscript={copyOperations.handleCopyTranscript}
          onExportMarkdown={() => copyOperations.handleExportTranscript('md')}
          onOpenMeetingFolder={meetingOperations.handleOpenMeetingFolder}
          isRecording={isRecording}
          disableAutoScroll={true}
          // Pagination props for efficient loading
          usePagination={true}
          segments={segments}
          hasMore={hasMore}
          isLoadingMore={isLoadingMore}
          totalCount={totalCount}
          loadedCount={loadedCount}
          onLoadMore={onLoadMore}
          // Retranscription props
          meetingId={meeting.id}
          meetingFolderPath={meeting.folder_path}
          onRefetchTranscripts={onRefetchTranscripts}
        />
        <SummaryPanel
          meeting={meeting}
          meetingTitle={meetingData.meetingTitle}
          onTitleChange={meetingData.handleTitleChange}
          isEditingTitle={meetingData.isEditingTitle}
          onStartEditTitle={() => meetingData.setIsEditingTitle(true)}
          onFinishEditTitle={() => meetingData.setIsEditingTitle(false)}
          isTitleDirty={meetingData.isTitleDirty}
          summaryRef={meetingData.blockNoteSummaryRef}
          isSaving={meetingData.isSaving}
          onSaveAll={meetingData.saveAllChanges}
          onCopySummary={copyOperations.handleCopySummary}
          onExportSummaryMarkdown={() => copyOperations.handleExportSummary('md')}
          onOpenFolder={meetingOperations.handleOpenMeetingFolder}
          aiSummary={meetingData.aiSummary}
          summaryStatus={summaryGeneration.summaryStatus}
          transcripts={meetingData.transcripts}
          modelConfig={modelConfig}
          setModelConfig={setModelConfig}
          onSaveModelConfig={handleSaveModelConfig}
          onGenerateSummary={summaryGeneration.handleGenerateSummary}
          onStopGeneration={summaryGeneration.handleStopGeneration}
          customPrompt={customPrompt}
          summaryResponse={summaryResponse}
          onSaveSummary={meetingData.handleSaveSummary}
          onSummaryChange={meetingData.handleSummaryChange}
          onDirtyChange={meetingData.setIsSummaryDirty}
          summaryError={summaryGeneration.summaryError}
          onRegenerateSummary={summaryGeneration.handleRegenerateSummary}
          getSummaryStatusMessage={summaryGeneration.getSummaryStatusMessage}
          availableTemplates={templates.availableTemplates}
          selectedTemplate={templates.selectedTemplate}
          onTemplateSelect={templates.handleTemplateSelection}
          isModelConfigLoading={false}
          onOpenModelSettings={handleRegisterModalOpen}
        />
      </div>
    </motion.div>
  );
}
