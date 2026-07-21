"use client";

import { Transcript, TranscriptSegmentData } from '@/types';
import { TranscriptView } from '@/components/TranscriptView';
import { VirtualizedTranscriptView } from '@/components/VirtualizedTranscriptView';
import { TranscriptButtonGroup } from './TranscriptButtonGroup';
import { useState, useMemo } from 'react';
import { ChevronDown, PencilLine } from 'lucide-react';

interface TranscriptPanelProps {
  transcripts: Transcript[];
  customPrompt: string;
  onPromptChange: (value: string) => void;
  onCopyTranscript: () => void;
  onExportMarkdown?: () => void;
  onExportTxt?: () => void;
  onOpenMeetingFolder: () => Promise<void>;
  isRecording: boolean;
  disableAutoScroll?: boolean;

  // Optional pagination props (when using virtualization)
  usePagination?: boolean;
  promptOpenDefault?: boolean;
  segments?: TranscriptSegmentData[];
  hasMore?: boolean;
  isLoadingMore?: boolean;
  totalCount?: number;
  loadedCount?: number;
  onLoadMore?: () => void;

  // Retranscription props
  meetingId?: string;
  meetingFolderPath?: string | null;
  onRefetchTranscripts?: () => Promise<void>;
}

export function TranscriptPanel({
  transcripts,
  customPrompt,
  onPromptChange,
  onCopyTranscript,
  onExportMarkdown,
  onExportTxt,
  onOpenMeetingFolder,
  isRecording,
  disableAutoScroll = false,
  usePagination = false,
  promptOpenDefault = true,
  segments,
  hasMore,
  isLoadingMore,
  totalCount,
  loadedCount,
  onLoadMore,
  meetingId,
  meetingFolderPath,
  onRefetchTranscripts,
}: TranscriptPanelProps) {
  const [promptOpen, setPromptOpen] = useState(promptOpenDefault);
  // Convert transcripts to segments if pagination is not used but we want virtualization
  const convertedSegments = useMemo(() => {
    if (usePagination && segments) {
      return segments;
    }
    // Convert transcripts to segments for virtualization
    return transcripts.map(t => ({
      id: t.id,
      timestamp: t.audio_start_time ?? 0,
      endTime: t.audio_end_time,
      text: t.text,
      confidence: t.confidence,
    }));
  }, [transcripts, usePagination, segments]);

  return (
    <div className="hidden md:flex md:w-1/4 lg:w-1/3 min-w-0 min-h-0 border-r border-gray-200 bg-white flex-col relative shrink-0 overflow-hidden">
      {/* Title area */}
      <div className="p-4 border-b border-gray-200">
        <TranscriptButtonGroup
          transcriptCount={usePagination ? (totalCount ?? convertedSegments.length) : (transcripts?.length || 0)}
          onCopyTranscript={onCopyTranscript}
          onExportMarkdown={onExportMarkdown}
          onExportTxt={onExportTxt}
          onOpenMeetingFolder={onOpenMeetingFolder}
          meetingId={meetingId}
          meetingFolderPath={meetingFolderPath}
          onRefetchTranscripts={onRefetchTranscripts}
        />
      </div>

      {/* Transcript content - use virtualized view for better performance */}
      <div className="flex-1 overflow-hidden pb-4">
        <VirtualizedTranscriptView
          segments={convertedSegments}
          isRecording={isRecording}
          isPaused={false}
          isProcessing={false}
          isStopping={false}
          enableStreaming={false}
          showConfidence={true}
          disableAutoScroll={disableAutoScroll}
          hasMore={hasMore}
          isLoadingMore={isLoadingMore}
          totalCount={totalCount}
          loadedCount={loadedCount}
          onLoadMore={onLoadMore}
        />
      </div>

      {/* Custom prompt input — 折叠面板, 中文化, 字符计数 */}
      {!isRecording && convertedSegments.length > 0 && (
        <div className="shrink-0 border-t border-neutral-200 bg-neutral-50/40">
          <button
            type="button"
            onClick={() => setPromptOpen(!promptOpen)}
            className="flex w-full items-center justify-between px-4 py-3 text-left transition-colors hover:bg-neutral-100/60"
          >
            <div className="flex items-center gap-2">
              <PencilLine className="h-3.5 w-3.5 text-neutral-500" />
              <span className="text-[13px] font-medium text-neutral-800">附加提示词</span>
              <span className="rounded-full bg-neutral-200/70 px-1.5 py-0.5 font-mono text-[10px] text-neutral-600">
                可选
              </span>
              {customPrompt && (
                <span className="rounded-full bg-blue-50 px-1.5 py-0.5 text-[10px] font-medium text-blue-600">
                  {customPrompt.length} 字
                </span>
              )}
            </div>
            <ChevronDown className={`h-3.5 w-3.5 text-neutral-400 transition-transform ${promptOpen ? 'rotate-180' : ''}`} />
          </button>
          {promptOpen && (
            <div className="px-4 pb-3">
              <textarea
                placeholder="给 AI 摘要加点背景, 例如:&#10;· 与会人 (产品/法务/财务)&#10;· 会议目标 (对齐 Q3 路线图)&#10;· 上下文 (上次会议遗留的 TODO)"
                className="block w-full max-w-full box-border resize-none rounded-lg border border-neutral-200 bg-white px-3 py-2.5 text-[13px] leading-relaxed text-neutral-800 placeholder:text-neutral-400 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20"
                rows={3}
                value={customPrompt}
                onChange={(e) => onPromptChange(e.target.value)}
                style={{ minHeight: '80px', maxHeight: '180px' }}
              />
              <div className="mt-1.5 flex items-center justify-between text-[10.5px] text-neutral-400">
                <span>作为 system prompt 注入到摘要生成</span>
                <span className={customPrompt.length > 1000 ? 'text-amber-600' : ''}>
                  {customPrompt.length} / 2000 字
                </span>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
