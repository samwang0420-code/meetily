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

// Format detection helper
function detectSummaryFormat(data: any): { format: SummaryFormat; data: any } {
  if (!data) {
    return { format: 'legacy', data: null };
  }

  // Priority 1: BlockNote format (has summary_json)
  if (data.summary_json && Array.isArray(data.summary_json)) {
    console.log('✅ FORMAT: BLOCKNOTE (summary_json exists)');
    return { format: 'blocknote', data };
  }

  // §170.6: Priority 1.5 - Multi-case JSON array (§165 wrap_summary_as_multi_case_array)
  // 必须在 Markdown 检测之前 — 后端 LLM 输出多案件时, markdown 字段是 JSON 数组字符串
  // `[{ "case_index": 1, "defendant": "...", "content": "...", "warning": "..." }]`,
  // 如果先走 Markdown, 会被 tryParseMarkdownToBlocks 当成普通文本, [{ 显示成原样.
  if (typeof data?.markdown === 'string') {
    const trimmed = data.markdown.trimStart();
    if (trimmed.startsWith('[{')) {
      try {
        const candidate = JSON.parse(trimmed);
        if (Array.isArray(candidate) && candidate.length >= 2 && candidate[0]?.case_index !== undefined) {
          console.log('\u2705 FORMAT: MULTI-CASE JSON array (' + candidate.length + ' cases)');
          return { format: 'multi-case', data: { ...data, _multiCase: candidate } };
        }
      } catch {
        // not JSON, fall through to markdown
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

  // §170: Helper - parse markdown content for one case (skip parsed JSON to avoid recursion)
  const parseCaseMarkdown = useCallback(async (md: string): Promise<Block[]> => {
    if (!editor) return [];
    try {
      const highlighted = highlightUnexpectedFacts(md, data?.fact_guard);
      return await editor.tryParseMarkdownToBlocks(highlighted);
    } catch (err) {
      console.warn('\u274c Failed to parse multi-case markdown, falling back to plain text:', err);
      // Fallback: 把整段 markdown 包成一个 paragraph block (BlockNote 接受混合 markdown)
      try {
        return await editor.tryParseMarkdownToBlocks(md);
      } catch {
        return [];
      }
    }
  }, [editor, data?.fact_guard]);

  // §170: Multi-case editors keyed per case_index (独立 BlockNote 实例)
  const [caseBlocks, setCaseBlocks] = useState<Record<number, Block[]>>({});
  const caseEditorsRef = useRef<Record<number, ReturnType<typeof useCreateBlockNote>>>({});

  // 初始化 / 更新每个 case 的 blocks
  useEffect(() => {
    if (format !== 'multi-case') return;
    let cancelled = false;
    (async () => {
      const next: Record<number, Block[]> = {};
      for (const c of multiCases) {
        const idx = typeof c.case_index === 'number' ? c.case_index : 0;
        const content = String(c.content || '');
        if (content) {
          const blocks = await parseCaseMarkdown(content);
          if (!cancelled) next[idx] = blocks;
        }
      }
      if (!cancelled) setCaseBlocks(next);
    })();
    return () => { cancelled = true; };
  }, [multiCases, parseCaseMarkdown, format]);

  // §170: Render multi-case JSON array as Card list
  if (format === 'multi-case') {
    console.log('\ud83c\udfa8 Rendering MULTI-CASE format (' + multiCases.length + ' cases)');
    return (
      <div className="flex flex-col gap-4 w-full p-4">
        {data?.fact_guard_legal_critical && (
          <FactGuardBanner report={data.fact_guard} legalCritical={true} />
        )}
        {multiCases.map((c, i) => (
          <MultiCaseCard
            key={'case-' + String(c.case_index ?? i) + '-' + i}
            caseData={c}
            index={i}
            factGuard={data?.fact_guard}
          />
        ))}
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


// §170: 多案件单 Card component — 独立 useCreateBlockNote 实例
function MultiCaseCard({ caseData, index, factGuard }: { caseData: any; index: number; factGuard: any }) {
  const { t } = useTranslation();
  const caseIdx = typeof caseData?.case_index === 'number' ? caseData.case_index : (index + 1);
  const defendant = String(caseData?.defendant || t('meeting.case_unknown') || ('案件 ' + caseIdx));
  const content = String(caseData?.content || '');
  const warning = caseData?.warning ? String(caseData.warning) : null;
  const editor = useCreateBlockNote({ initialContent: undefined });
  const [blocks, setBlocks] = useState<Block[]>([]);
  const isContentLoaded = useRef(false);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        isContentLoaded.current = false;
        const highlighted = highlightUnexpectedFacts(content, factGuard);
        const parsed = await editor.tryParseMarkdownToBlocks(highlighted);
        if (!cancelled) {
          editor.replaceBlocks(editor.document, parsed);
          setBlocks(parsed);
          setTimeout(() => { isContentLoaded.current = true; }, 100);
        }
      } catch (err) {
        console.warn('§170 MultiCaseCard parse failed for case', caseIdx, err);
      }
    })();
    return () => { cancelled = true; };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [content, caseIdx]);

  return (
    <div
      data-testid={'multi-case-card-' + caseIdx}
      className="rounded-xl border border-orange-200 bg-white shadow-sm overflow-hidden"
    >
      <div className="px-4 py-3 bg-gradient-to-r from-orange-50 to-amber-50 border-b border-orange-200 flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <span className="inline-flex items-center justify-center w-7 h-7 rounded-full bg-orange-500 text-white text-xs font-bold">
            {caseIdx}
          </span>
          <span className="font-semibold text-gray-800 text-sm">
            {defendant}
          </span>
        </div>
        <span className="text-[10px] uppercase tracking-wide text-orange-700 font-medium">
          {t('meeting.case_label')} #{caseIdx}
        </span>
      </div>
      {warning && (
        <div className="px-4 py-2 bg-amber-50 border-b border-amber-200 text-xs text-amber-800">
          <span className="font-semibold">⚠️ </span>{warning}
        </div>
      )}
      <div className="px-4 py-3">
        {blocks.length > 0 ? (
          <BlockNoteView editor={editor} editable={true} theme="light" />
        ) : (
          <pre className="whitespace-pre-wrap break-words font-sans text-sm leading-7 text-gray-700">
            {content}
          </pre>
        )}
      </div>
    </div>
  );
}
