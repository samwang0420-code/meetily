import { useCallback, RefObject } from 'react';
import { Transcript, Summary } from '@/types';
import { BlockNoteSummaryViewRef } from '@/components/AISummary/BlockNoteSummaryView';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import Analytics from '@/lib/analytics';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { useTranslation } from '@/i18n';

interface UseCopyOperationsProps {
  meeting: any;
  transcripts: Transcript[];
  meetingTitle: string;
  aiSummary: Summary | null;
  blockNoteSummaryRef: RefObject<BlockNoteSummaryViewRef>;
}

export function useCopyOperations({
  meeting,
  transcripts,
  meetingTitle,
  aiSummary,
  blockNoteSummaryRef,
}: UseCopyOperationsProps) {
  const { t, locale } = useTranslation();

  // Helper function to fetch ALL transcripts for copying (not just paginated data)
  const fetchAllTranscripts = useCallback(async (meetingId: string): Promise<Transcript[]> => {
    try {
      console.log('📊 Fetching all transcripts for copying:', meetingId);

      // First, get total count by fetching first page
      const firstPage = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
        limit: 1,
        offset: 0,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      const totalCount = firstPage.total_count;
      console.log(`📊 Total transcripts in database: ${totalCount}`);

      if (totalCount === 0) {
        return [];
      }

      // Fetch all transcripts in one call
      const allData = await invokeTauri('api_get_meeting_transcripts', {
        meetingId,
        limit: totalCount,
        offset: 0,
      }) as { transcripts: Transcript[]; total_count: number; has_more: boolean };

      console.log(`✅ Fetched ${allData.transcripts.length} transcripts from database for copying`);
      return allData.transcripts;
    } catch (error) {
      console.error('❌ Error fetching all transcripts:', error);
      safeToast.error(t('meeting_details.copy_transcript_failed'));
      return [];
    }
  }, [t]);

  // Copy transcript to clipboard
  const handleCopyTranscript = useCallback(async () => {
    // CHANGE: Fetch ALL transcripts from database, not from pagination state
    console.log('📊 Fetching all transcripts for copying...');
    const allTranscripts = await fetchAllTranscripts(meeting.id);

    if (!allTranscripts.length) {
      const error_msg = t('meeting_details.no_transcript');
      console.log(error_msg);
      safeToast.error(error_msg);
      return;
    }

    console.log(`✅ Copying ${allTranscripts.length} transcripts to clipboard`);

    // Format timestamps as recording-relative [MM:SS] instead of wall-clock time
    const formatTime = (seconds: number | undefined, fallbackTimestamp: string): string => {
      if (seconds === undefined) {
        // For old transcripts without audio_start_time, use wall-clock time
        return fallbackTimestamp;
      }
      const totalSecs = Math.floor(seconds);
      const mins = Math.floor(totalSecs / 60);
      const secs = totalSecs % 60;
      return `[${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]`;
    };

    const header = `# Transcript of the Meeting: ${meeting.id} - ${meetingTitle ?? meeting.title}\n\n`;
    const date = `## Date: ${new Date(meeting.created_at).toLocaleDateString()}\n\n`;
    const fullTranscript = allTranscripts
      .map(t => `${formatTime(t.audio_start_time, t.timestamp)} ${t.text}  `)
      .join('\n');

    await navigator.clipboard.writeText(header + date + fullTranscript);
    safeToast.success(t('meeting_details.copy_success'));

    // Track copy analytics
    const wordCount = allTranscripts
      .map(t => t.text.split(/\s+/).length)
      .reduce((a, b) => a + b, 0);

    await Analytics.trackCopy('transcript', {
      meeting_id: meeting.id,
      transcript_length: allTranscripts.length.toString(),
      word_count: wordCount.toString()
    });
  }, [meeting, meetingTitle, fetchAllTranscripts, t]);

  // Copy summary to clipboard
  const handleCopySummary = useCallback(async () => {
    try {
      let summaryMarkdown = '';

      console.log('🔍 Copy Summary - Starting...');

      // Try to get markdown from BlockNote editor first
      if (blockNoteSummaryRef.current?.getMarkdown) {
        console.log('📝 Trying to get markdown from ref...');
        summaryMarkdown = await blockNoteSummaryRef.current.getMarkdown();
        console.log('📝 Got markdown from ref, length:', summaryMarkdown.length);
      }

      // Fallback: Check if aiSummary has markdown property
      if (!summaryMarkdown && aiSummary && 'markdown' in aiSummary) {
        console.log('📝 Using markdown from aiSummary');
        summaryMarkdown = (aiSummary as any).markdown || '';
        console.log('📝 Markdown from aiSummary, length:', summaryMarkdown.length);
      }

      // Fallback: Check for legacy format
      if (!summaryMarkdown && aiSummary) {
        console.log('📝 Converting legacy format to markdown');
        const sections = Object.entries(aiSummary)
          .filter(([key]) => {
            // Skip non-section keys
            return key !== 'markdown' && key !== 'summary_json' && key !== '_section_order' && key !== 'MeetingName';
          })
          .map(([, section]) => {
            if (section && typeof section === 'object' && 'title' in section && 'blocks' in section) {
              const sectionTitle = `## ${section.title}\n\n`;
              const sectionContent = section.blocks
                .map((block: any) => `- ${block.content}`)
                .join('\n');
              return sectionTitle + sectionContent;
            }
            return '';
          })
          .filter(s => s.trim())
          .join('\n\n');
        summaryMarkdown = sections;
        console.log('📝 Converted legacy format, length:', summaryMarkdown.length);
      }

      // If still no summary content, show message
      if (!summaryMarkdown.trim()) {
        console.error('❌ No summary content available to copy');
        safeToast.error(t('summary.no_summary_to_copy'));
        return;
      }

      // Build metadata header
      const header = `# Meeting Summary: ${meetingTitle}\n\n`;
      const metadata = `**Meeting ID:** ${meeting.id}\n**Date:** ${new Date(meeting.created_at).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      })}\n**Copied on:** ${new Date().toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit'
      })}\n\n---\n\n`;

      const fullMarkdown = header + metadata + summaryMarkdown;
      await navigator.clipboard.writeText(fullMarkdown);

      console.log('✅ Successfully copied to clipboard!');
      safeToast.success(t('summary.copy_success'));

      // Track copy analytics
      await Analytics.trackCopy('summary', {
        meeting_id: meeting.id,
        has_markdown: (!!aiSummary && 'markdown' in aiSummary).toString()
      });
    } catch (error) {
      console.error('❌ Failed to copy summary:', error);
      safeToast.error(t('summary.copy_failed'));
    }
  }, [aiSummary, meetingTitle, meeting, blockNoteSummaryRef, t]);

  // Export transcript as Markdown or TXT via browser download (zero-dependency)
  const handleExportTranscript = useCallback(async (format: 'md' | 'txt') => {
    try {
      const allTranscripts = await fetchAllTranscripts(meeting.id);

      if (!allTranscripts.length) {
        safeToast.error(t('meeting_details.no_transcript_to_export'));
        return;
      }

      const formatTime = (seconds: number | undefined, fallbackTimestamp: string): string => {
        if (seconds === undefined) return fallbackTimestamp;
        const totalSecs = Math.floor(seconds);
        const mins = Math.floor(totalSecs / 60);
        const secs = totalSecs % 60;
        return format === 'md'
          ? `**[${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]** `
          : `[${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}] `;
      };

      const safeTitle = (meetingTitle ?? meeting.title ?? '未命名会议')
        .replace(/[\\/:*?"<>|]/g, '_')
        .slice(0, 60);
      const dateStr = new Date(meeting.created_at).toISOString().slice(0, 10);

      let content = '';
      let mimeType = '';
      let ext = '';

      if (format === 'md') {
        ext = 'md';
        mimeType = 'text/markdown;charset=utf-8';
        const header = `# ${meetingTitle ?? meeting.title}\n\n` +
          `**会议 ID:** ${meeting.id}  \n` +
          `**日期:** ${dateStr}  \n` +
          `**导出时间:** ${new Date().toLocaleString('zh-CN')}  \n` +
          `**总段数:** ${allTranscripts.length}\n\n---\n\n`;
        const body = allTranscripts
          .map(t => `${formatTime(t.audio_start_time, t.timestamp)}${t.text}`)
          .join('\n\n');
        content = header + body;
      } else {
        ext = 'txt';
        mimeType = 'text/plain;charset=utf-8';
        const header = `会议: ${meetingTitle ?? meeting.title}\n` +
          `会议 ID: ${meeting.id}\n` +
          `日期: ${dateStr}\n` +
          `导出时间: ${new Date().toLocaleString('zh-CN')}\n` +
          `总段数: ${allTranscripts.length}\n\n${'='.repeat(40)}\n\n`;
        const body = allTranscripts
          .map(t => `${formatTime(t.audio_start_time, t.timestamp)}${t.text}`)
          .join('\n');
        content = header + body;
      }

      const filename = `${safeTitle}_${dateStr}.${ext}`;
      const blob = new Blob([content], { type: mimeType });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      // Defer revoke to give browser time to start download
      setTimeout(() => URL.revokeObjectURL(url), 1000);

      safeToast.success(t('meeting_details.export_success', { filename }));

      await Analytics.trackCopy(format === 'md' ? 'export_md' : 'export_txt', {
        meeting_id: meeting.id,
        transcript_length: allTranscripts.length.toString(),
      });
    } catch (error) {
      console.error('❌ Failed to export transcript:', error);
      safeToast.error(t('meeting_details.export_failed'));
    }
  }, [meeting, meetingTitle, fetchAllTranscripts, t]);

  // v0.6.15: Export summary as Markdown or TXT (零依赖 Blob + a.download)
  // 与 handleExportTranscript 不同的: 从 blockNoteSummaryRef.getMarkdown() 拿 markdown 原文
  const handleExportSummary = useCallback(async (format: 'md' | 'txt') => {
    try {
      let summaryMarkdown = '';

      // Try BlockNote ref first
      if (blockNoteSummaryRef.current?.getMarkdown) {
        summaryMarkdown = await blockNoteSummaryRef.current.getMarkdown();
      }

      // Fallback: aiSummary.markdown
      if (!summaryMarkdown && aiSummary && 'markdown' in aiSummary) {
        summaryMarkdown = (aiSummary as any).markdown || '';
      }

      // Keep legacy meetings exportable when no BlockNote markdown exists.
      if (!summaryMarkdown && aiSummary) {
        summaryMarkdown = Object.entries(aiSummary)
          .filter(([key]) => !['markdown', 'summary_json', '_section_order', 'MeetingName'].includes(key))
          .map(([, section]) => {
            if (!section || typeof section !== 'object' || !('title' in section) || !('blocks' in section)) return '';
            const blocks = Array.isArray((section as any).blocks) ? (section as any).blocks : [];
            return `## ${(section as any).title}\n\n${blocks.map((block: any) => `- ${block.content ?? ''}`).join('\n')}`;
          })
          .filter(Boolean)
          .join('\n\n');
      }

      if (!summaryMarkdown.trim()) {
        safeToast.error(t('summary.no_summary_to_export'));
        return;
      }

      const safeTitle = (meetingTitle ?? meeting.title ?? '未命名会议')
        .replace(/[\\/:*?"<>|]/g, '_')
        .slice(0, 60);
      const dateStr = new Date(meeting.created_at).toISOString().slice(0, 10);
      const ext = format === 'md' ? 'md' : 'txt';
      const mimeType = format === 'md' ? 'text/markdown;charset=utf-8' : 'text/plain;charset=utf-8';
      const filename = `${safeTitle}_${dateStr}_summary.${ext}`;

      const content = format === 'md'
        ? `# ${meetingTitle ?? meeting.title}\n\n**Meeting ID:** ${meeting.id}\n**Date:** ${dateStr}\n\n---\n\n${summaryMarkdown}`
        : summaryMarkdown.replace(/^#{1,6}\s+/gm, '').replace(/\*\*(.*?)\*\*/g, '$1');
      const blob = new Blob([content], { type: mimeType });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      setTimeout(() => URL.revokeObjectURL(url), 1000);

      safeToast.success(t('summary.export_success', { filename }));

      await Analytics.trackCopy(format === 'md' ? 'export_summary_md' : 'export_summary_txt', {
        meeting_id: meeting.id,
        summary_length: summaryMarkdown.length.toString(),
      });
    } catch (error) {
      console.error('❌ Failed to export summary:', error);
      safeToast.error(t('summary.export_failed'));
    }
  }, [meeting, meetingTitle, aiSummary, blockNoteSummaryRef, t]);

  return {
    handleCopyTranscript,
    handleCopySummary,
    handleExportTranscript,
    handleExportSummary,
  };
}
