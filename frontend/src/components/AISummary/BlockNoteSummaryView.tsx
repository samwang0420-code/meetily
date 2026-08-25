"use client";

import { useState, useEffect, useCallback, useRef, useMemo, forwardRef, useImperativeHandle } from 'react';
import { useTranslation } from '@/i18n';
import dynamic from 'next/dynamic';
import { Summary, SummaryDataResponse, SummaryFormat, BlockNoteBlock } from '@/types';
import { AISummary } from './index';
import { Block } from '@blocknote/core';
import { useCreateBlockNote } from '@blocknote/react';
import { BlockNoteView } from '@blocknote/shadcn';
import { blocksToMarkdownSafely } from '@/lib/blocknote-markdown';
import { highlightUnexpectedFacts } from '@/lib/highlight_facts';
import { FactGuardBanner } from './FactGuardBanner';
import { NumberGuardBanner, TemplateMismatchBanner, PendingFilterBanner, TimelineConflictBanner, PartyRoleBanner, TimelineCoverageBanner } from './NumberGuardBanner';
import "@blocknote/shadcn/style.css";

// Dynamically import BlockNote Editor to avoid SSR issues
const Editor = dynamic(() => import('../BlockNoteEditor/Editor'), { ssr: false });

interface BlockNoteSummaryViewProps {
  summaryData: SummaryDataResponse | Summary | null;
  onSave?: (data: { markdown?: string; summary_json?: BlockNoteBlock[] }) => void;
  onSummaryChange?: (summary: Summary) => void;
  status?: 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error';
  error?: string | null;
  onRegenerateSummary?: () => void;
  meeting?: {
    id: string;
    title: string;
    created_at: string;
  };
  onDirtyChange?: (isDirty: boolean) => void;
}

export interface BlockNoteSummaryViewRef {
  saveSummary: () => Promise<void>;
  getMarkdown: () => Promise<string>;
  isDirty: boolean;
}


// §170.7: 极宽松正则 fallback — 当 JSON.parse 失败时, 用正则匹配 case_index / defendant / content
function extractCasesByRegex(md: string): any[] {
  const cases: any[] = [];
  // 匹配每个 { ... } 块 (非贪婪, 不跨 case 嵌套)
  const caseBlocks = md.match(/\{[^{}]*?"case_index"\s*:\s*\d+[^{}]*?\}/g) || [];
  if (!caseBlocks) return cases;
  for (const block of caseBlocks) {
    const caseIdxMatch = block.match(/"case_index"\s*:\s*(\d+)/);
    const defMatch = block.match(/"defendant"\s*:\s*"([^"]*)"/);
    const contentMatch = block.match(/"content"\s*:\s*"([\s\S]*?)"\s*,\s*"warning"/);
    const warningMatch = block.match(/"warning"\s*:\s*(?:"([^"]*)"|null)/);
    if (caseIdxMatch) {
      cases.push({
        case_index: parseInt(caseIdxMatch[1], 10),
        defendant: defMatch ? defMatch[1] : 'Unknown',
        content: contentMatch ? contentMatch[1].replace(/\\n/g, '\n').replace(/\\"/g, '"') : '',
        warning: warningMatch ? (warningMatch[1] || null) : null,
      });
    }
  }
  return cases;
}

// Format detection helper
function detectSummaryFormat(data: any): { format: SummaryFormat; data: any } {
  if (!data) {
    return { format: 'legacy', data: null };
  }
  // §170.7 DEBUG: trace what data structure we received
  console.log('[§170.7 DEBUG] detectSummaryFormat input:', {
    type: typeof data,
    keys: Object.keys(data || {}),
    markdownType: typeof data?.markdown,
    markdownLen: typeof data?.markdown === 'string' ? data.markdown.length : 0,
    markdownPrefix: typeof data?.markdown === 'string' ? data.markdown.slice(0, 50) : null,
    hasSummaryJson: Array.isArray(data?.summary_json),
    hasMultiCase: Array.isArray(data?._multiCase),
  });


  // Priority 1: BlockNote format (has summary_json)
  if (data.summary_json && Array.isArray(data.summary_json)) {
    console.log('✅ FORMAT: BLOCKNOTE (summary_json exists)');
    return { format: 'blocknote', data };
  }

  // §170.7: Priority 1.5 - Multi-case JSON array (§165 wrap_summary_as_multi_case_array)
  // 必须在 Markdown 检测之前. 后端 LLM 输出多案件时, markdown 字段是 JSON 数组字符串.
  // 关键 bug (§170.6 老代码): JSON.parse 严格模式不认 content 字段里的字面 \n 换行符,
  // 直接抛 'Bad control character' 异常, fallback 到 markdown 解析, 显示 raw JSON.
  // §170.7 修复: 先尝试宽松 JSON 解析 (替换转义符), 失败再用正则逐个 case 提取.
  if (typeof data?.markdown === 'string') {
    const trimmed = data.markdown.trimStart();
    if (/^\[\s*\{/.test(trimmed)) {
      // Step 1: 严格 JSON.parse (干净数据)
      let candidate: any = null;
      try { candidate = JSON.parse(trimmed); } catch {}
      // Step 2: 宽松 JSON 解析 (content 字段里可能有未转义的换行符)
      if (!Array.isArray(candidate) || candidate.length < 2 || candidate[0]?.case_index === undefined) {
        try {
          const sanitized = trimmed
            .replace(/\\n/g, '\n')
            .replace(/\\r/g, '\r')
            .replace(/\\t/g, '\t');
          // 把 content: "..." 里未转义的换行符 escape 掉
          const loose = sanitized.replace(
            /"(content|warning|defendant)"\s*:\s*"((?:[^"\\\\]|[\\s\\S])*?)"/g,
            (_m: string, key: string, val: string) => {
              const escaped = val
                .replace(/\\\\/g, '\\\\')
                .replace(/\\n/g, '\\n')
                .replace(/\\r/g, '\\r')
                .replace(/\\t/g, '\\t');
              return '"' + key + '":\"' + escaped + '\"';
            }
          );
          candidate = JSON.parse(loose);
        } catch {}
      }
      // Step 3: 极宽松正则 fallback — 从 markdown 字符串里提取所有 case 块
      if (!Array.isArray(candidate) || candidate.length < 2) {
        candidate = extractCasesByRegex(trimmed);
      }
      if (Array.isArray(candidate) && candidate.length >= 2 && candidate[0]?.case_index !== undefined) {
        console.log('\u2705 FORMAT: MULTI-CASE JSON array (' + candidate.length + ' cases)');
        return { format: 'multi-case', data: { ...data, _multiCase: candidate } };
      }
    }
  }

  // Priority 2: Markdown format
  if (data.markdown && typeof data.markdown === 'string') {
    console.log('✅ FORMAT: MARKDOWN (will parse to BlockNote)');
    return { format: 'markdown', data };
  }

  // Priority 3: Legacy JSON
  const hasLegacyStructure = data.MeetingName || Object.keys(data).some(key =>
    typeof data[key] === 'object' && data[key]?.title && data[key]?.blocks
  );

  if (hasLegacyStructure) {
    console.log('✅ FORMAT: LEGACY (custom JSON)');
    return { format: 'legacy', data };
  }

  return { format: 'legacy', data: null };
}

export const BlockNoteSummaryView = forwardRef<BlockNoteSummaryViewRef, BlockNoteSummaryViewProps>(({
  summaryData,
  onSave,
  onSummaryChange,
  status = 'idle',
  error = null,
  onRegenerateSummary,
  meeting,
  onDirtyChange
}, ref) => {
  const { format, data } = detectSummaryFormat(summaryData);
  const [isDirty, setIsDirty] = useState(false);
  const [currentBlocks, setCurrentBlocks] = useState<Block[]>([]);
  const [isSaving, setIsSaving] = useState(false);
  const isContentLoaded = useRef(false);
  const lastLoadedMarkdownRef = useRef<string>('');

  // Create BlockNote editor for markdown parsing
  const editor = useCreateBlockNote({
    initialContent: undefined
  });

  // Parse markdown to blocks when format is markdown
  useEffect(() => {
    if (format === 'markdown' && data?.markdown && editor) {
      const loadMarkdown = async () => {
        try {
          console.log('📝 Parsing markdown to BlockNote blocks...');
          // v0.6.22: 防 #321 死循环 — 在 replaceBlocks 前先关掉 isContentLoaded,
          // 让 onChange 期间的 setCurrentBlocks/setIsDirty 不触发 parent re-render
          isContentLoaded.current = false;
          // §141 D 方案: 把 fact_guard 报警的 unexpected_dates/numbers 用 ==包裹==
          // 让 BlockNote markdown parser 渲染成高亮 (BlockNote 默认支持 ==highlight== 语法)
          const highlighted = highlightUnexpectedFacts(data.markdown, data.fact_guard);
          const blocks = await editor.tryParseMarkdownToBlocks(highlighted);
          editor.replaceBlocks(editor.document, blocks);
          console.log('✅ Markdown parsed successfully', { highlighted: highlighted !== data.markdown });

          // Delay to ensure editor has finished rendering before allowing onChange
          setTimeout(() => {
            isContentLoaded.current = true;
          }, 100);
        } catch (err) {
          console.error('❌ Failed to parse markdown:', err);
          // 失败也要恢复,避免 dirty state 永久僵住
          isContentLoaded.current = true;
        }
      };
      loadMarkdown();
    }
  }, [format, data?.markdown, editor]);

  // Set content loaded flag for blocknote format
  useEffect(() => {
    if (format === 'blocknote' && data?.summary_json) {
      // Delay to ensure editor has finished rendering
      setTimeout(() => {
        isContentLoaded.current = true;
      }, 100);
    }
  }, [format, data?.summary_json]);

  // v0.6.21+: 双重触发条件
  // - status 从其他状态 → completed (polling 完成时)
  // - markdown 字符串变化 (切走切回, 父组件换了新 prop)
  // 防同 markdown 重复触发: 用 `${len}:${prefix64}` 做 key
  useEffect(() => {
    if (status !== 'completed') return;
    if (format !== 'markdown' || !data?.markdown || !editor) return;
    const md = data.markdown;
    const markdownKey = `${md.length}:${md.slice(0, 64)}`;
    if (lastLoadedMarkdownRef.current === markdownKey) return;
    lastLoadedMarkdownRef.current = markdownKey;
    (async () => {
      try {
        // §141 D 方案: 高亮 fact_guard 报警项
        const highlighted = highlightUnexpectedFacts(md, data.fact_guard);
        const blocks = await editor.tryParseMarkdownToBlocks(highlighted);
        editor.replaceBlocks(editor.document, blocks);
        isContentLoaded.current = true;
        console.log('✅ [BlockNote] force reload on status=completed, len=', md.length, { highlighted: highlighted !== md });
      } catch (err) {
        console.error('❌ [BlockNote] force reload failed', err);
      }
    })();
  }, [status, data?.markdown, editor, format]);

  const handleEditorChange = useCallback((blocks: Block[]) => {
    // Only set dirty flag if content has finished loading
    if (isContentLoaded.current) {
      setCurrentBlocks(blocks);
      setIsDirty(true);
    }
  }, []);

  // Notify parent of dirty state changes
  useEffect(() => {
    if (onDirtyChange) {
      onDirtyChange(isDirty);
    }
  }, [isDirty, onDirtyChange]);

  const handleSave = useCallback(async () => {
    if (!onSave || !isDirty) return;

    setIsSaving(true);
    try {
      console.log('💾 Saving BlockNote content...');

      // Generate markdown from current blocks; preserve BlockNote JSON even if markdown conversion fails.
      const markdownResult = await blocksToMarkdownSafely(editor, currentBlocks, {
        source: 'BlockNoteSummaryView.handleSave',
      });

      const saveData: { markdown?: string; summary_json?: BlockNoteBlock[] } = {
        summary_json: currentBlocks as unknown as BlockNoteBlock[]
      };

      if (markdownResult.markdown !== undefined) {
        saveData.markdown = markdownResult.markdown;
      }

      onSave(saveData);

      setIsDirty(false);
      console.log('✅ Save successful');
    } catch (err) {
      console.error('❌ Save failed:', err);
      alert('Failed to save changes. Please try again.');
    } finally {
      setIsSaving(false);
    }
  }, [onSave, isDirty, currentBlocks, editor]);

  // Expose methods to parent via ref
  useImperativeHandle(ref, () => ({
    saveSummary: handleSave,
    getMarkdown: async () => {
      try {
        console.log('🔍 getMarkdown called, format:', format);
        console.log('🔍 currentBlocks length:', currentBlocks.length);
        console.log('🔍 data:', data);

        // For markdown format - use the main editor
        if (format === 'markdown' && editor) {
          console.log('📝 Using markdown editor, blocks:', editor.document.length);
          const markdownResult = await blocksToMarkdownSafely(editor, editor.document, {
            source: 'BlockNoteSummaryView.getMarkdown.markdown',
            fallbackMarkdown: data?.markdown,
          });
          console.log('📝 Generated markdown length:', markdownResult.markdown?.length || 0);
          return markdownResult.markdown || '';
        }

        // For blocknote format - use currentBlocks state
        if (format === 'blocknote') {
          console.log('📝 BlockNote format, currentBlocks:', currentBlocks.length);
          const blocks = currentBlocks.length > 0
            ? currentBlocks
            : (data?.summary_json as unknown as Block[] | undefined) || [];

          if (blocks.length > 0 && editor) {
            const markdownResult = await blocksToMarkdownSafely(editor, blocks, {
              source: 'BlockNoteSummaryView.getMarkdown.blocknote',
              fallbackMarkdown: data?.markdown,
            });
            console.log('📝 Generated markdown from blocks, length:', markdownResult.markdown?.length || 0);
            return markdownResult.markdown || '';
          }
          // Fallback: if we have the original data with markdown
          if (data?.markdown) {
            console.log('📝 Using fallback markdown from data');
            return data.markdown;
          }
        }

        // For legacy format - return empty (handled by parent)
        console.warn('⚠️ Cannot generate markdown for legacy format, returning empty');
        return '';
      } catch (err) {
        console.error('❌ Failed to generate markdown:', err);
        return '';
      }
    },
    isDirty
  }), [handleSave, isDirty, editor, format, currentBlocks, data]);

  // Render legacy format
  if (format === 'legacy') {
    console.log('🎨 Rendering LEGACY format');
    return (
      <AISummary
        summary={summaryData as Summary}
        status={status}
        error={error}
        onSummaryChange={onSummaryChange || (() => { })}
        onRegenerateSummary={onRegenerateSummary || (() => { })}
        meeting={meeting}
      />
    );
  }

  // §170: Parse multi-case JSON for real (memoized)
  const multiCases = useMemo(() => {
    if (format !== 'multi-case') return [];
    return Array.isArray(data?._multiCase) ? (data._multiCase as any[]) : [];
  }, [format, data?._multiCase]);

  // §170: Render multi-case JSON array as Card list
  // §170.9: Load case[0].content into editor for multi-case render
  useEffect(() => {
    if (format !== 'multi-case' || !editor) return;
    const caseContent = String(multiCases[0]?.content || '');
    if (!caseContent) return;
    (async () => {
      try {
        isContentLoaded.current = false;
        const highlighted = highlightUnexpectedFacts(caseContent, data?.fact_guard);
        const blocks = await editor.tryParseMarkdownToBlocks(highlighted);
        editor.replaceBlocks(editor.document, blocks);
        setTimeout(() => { isContentLoaded.current = true; }, 100);
      } catch (err) {
        console.error('\u274c Failed to parse multi-case content:', err);
        isContentLoaded.current = true;
      }
    })();
  }, [format, multiCases, editor, data?.fact_guard]);

  if (format === 'multi-case') {
    console.log('\ud83d\udca5 Rendering MULTI-CASE warning + case[0] content as markdown (' + multiCases.length + ' cases)');
    const caseContent = String(multiCases[0]?.content || '');
    const multiCaseWarning: string | null = multiCases.length >= 2
      ? (multiCases.find((c: any) => c.warning)?.warning as string) || null
      : null;
    const parseMarkdownToBlocks = useCallback(async (md: string): Promise<Block[]> => {
      if (!editor) return [];
      try {
        const highlighted = highlightUnexpectedFacts(md, data?.fact_guard);
        return await editor.tryParseMarkdownToBlocks(highlighted);
      } catch {
        return [];
      }
    }, [editor, data?.fact_guard]);
    return (
      <div className="flex flex-col w-full">
        {data?.fact_guard_legal_critical && (
          <FactGuardBanner report={data.fact_guard} legalCritical={true} />
        )}
        {/* §182 + §183: 数字一致性 / 模板错配 / 待查明事项真伪过滤 / 时间线冲突 / 立场标注 / 时间线覆盖度 — 6 个 banner */}
        <NumberGuardBanner report={data?.number_consistency} />
        <TemplateMismatchBanner report={data?.template_mismatch} />
        <PendingFilterBanner report={data?.pending_filter} />
        <TimelineConflictBanner report={data?.timeline_conflict} />
        <PartyRoleBanner report={data?.party_role} />
        <TimelineCoverageBanner report={data?.timeline_coverage} />
        {multiCaseWarning && (
          <div className="px-4 py-3 bg-amber-50 border-b border-amber-200 text-sm text-amber-900">
            <div className="font-semibold mb-1">{'\u26a0\ufe0f'} {'\u68c0\u6d4b\u5230\u591a\u6bb5\u72ec\u7acb\u5185\u5bb9'} ({multiCases.length} {'\u4e2a\u6848\u4ef6'})</div>
            <div className="text-xs leading-5 mt-1">{multiCaseWarning}</div>
            <div className="text-xs mt-2 text-amber-700">{'\u5efa\u8bae\uff1a\u4ec5\u4ee5\u672c\u6848'} ({multiCases[0]?.defendant || '\u4e3b\u6848\u4ef6'}) {'\u5185\u5bb9\u4e3a\u51c6\uff0c\u5176\u4ed6\u6848\u4ef6\u9700\u91cd\u65b0\u751f\u6210\u3002'}</div>
          </div>
        )}
        {caseContent && (
          <div className="w-full">
            <BlockNoteView editor={editor} editable={true} theme="light" />
          </div>
        )}
      </div>
    );
  }

  // Render BlockNote format (has summary_json)
  if (format === 'blocknote') {
    console.log('🎨 Rendering BLOCKNOTE format (direct)');
    return (
      <div className="flex flex-col w-full">
        {/* §148: 法律 critical (人名漂移/角色混淆/判决编造) — 显式警告,避免直接转发 */}
        {data?.fact_guard_legal_critical && (
          <FactGuardBanner
            report={data.fact_guard}
            legalCritical={true}
          />
        )}
        {/* §182: 数字一致性 / 模板错配 / 待查明事项真伪过滤 / 时间线冲突 — 4 个 banner */}
        <NumberGuardBanner report={data?.number_consistency} />
        <TemplateMismatchBanner report={data?.template_mismatch} />
        <PendingFilterBanner report={data?.pending_filter} />
        <TimelineConflictBanner report={data?.timeline_conflict} />
        {/* §141.7: 隐藏 banner — 用户 8/20 反馈"华而不实",只保留 D 方案黄底高亮 (highlightUnexpectedFacts) */}
        {/* 恢复: git log -p 取回这一行 + i18n banner_* keys 都保留在 zh.ts/en.ts */}
        <div className="w-full">
          <Editor
            initialContent={data.summary_json}
            onChange={(blocks) => {
              console.log('📝 Editor blocks changed:', blocks.length);
              handleEditorChange(blocks);
            }}
            editable={true}
          />
        </div>
      </div>
    );
  }

  // Render Markdown format (parse and display in BlockNote)
  if (format === 'markdown') {
    console.log('🎨 Rendering MARKDOWN format (parsed to BlockNote)');
    return (
      <div className="flex flex-col w-full">
        {/* §148: 法律 critical 警告 (markdown 路径) */}
        {data?.fact_guard_legal_critical && (
          <FactGuardBanner
            report={data.fact_guard}
            legalCritical={true}
          />
        )}
        {/* §182: 数字一致性 / 模板错配 / 待查明事项真伪过滤 / 时间线冲突 — 4 个 banner */}
        <NumberGuardBanner report={data?.number_consistency} />
        <TemplateMismatchBanner report={data?.template_mismatch} />
        <PendingFilterBanner report={data?.pending_filter} />
        <TimelineConflictBanner report={data?.timeline_conflict} />
        {/* §141.7: 同上 — 隐藏 banner,只保留黄底高亮 D 方案 */}
        <div className="w-full">
          <BlockNoteView
            editor={editor}
            editable={true}
            onChange={() => {
              if (isContentLoaded.current) {
                handleEditorChange(editor.document);
              }
            }}
            theme="light"
          />
        </div>
      </div>
    );
  }

  return null;
});

BlockNoteSummaryView.displayName = 'BlockNoteSummaryView';
