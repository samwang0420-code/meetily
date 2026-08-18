"use client";

import { Summary, SummaryResponse, Transcript } from '@/types';
import { useTranslation } from '@/i18n';
import { EditableTitle } from '@/components/EditableTitle';
import { BlockNoteSummaryView, BlockNoteSummaryViewRef } from '@/components/AISummary/BlockNoteSummaryView';
import { EmptyStateSummary } from '@/components/EmptyStateSummary';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { ActionItemsList } from './ActionItemsList';
import { SummaryHistoryPanel } from './SummaryHistoryPanel';
// §124 dead-code-elimination: SummaryGeneratorButtonGroup import removed (was at line 9 in §123 baseline)
// §124 dead-code-elimination: SummaryUpdaterButtonGroup import removed (was at line 12 in §123 baseline)
import { SpeakerRosterDrawer } from '@/components/SpeakerRoster/SpeakerRosterDrawer';
import Analytics from '@/lib/analytics';
import { useEffect, useRef, useState, RefObject } from 'react';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import { Languages, ChevronDown, Users, History } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubTrigger,
  DropdownMenuSubContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
} from '@/components/ui/dropdown-menu';
import { Dialog, DialogContent, DialogTrigger, DialogTitle } from '@/components/ui/dialog';
import { Settings, Download, Sparkles, Save, Copy, FileCode, FileText, FolderOpen, FileType, Square, Check, Loader2 } from 'lucide-react';
import { ModelSettingsModal } from '@/components/ModelSettingsModal';
import { VisuallyHidden } from '@radix-ui/react-visually-hidden';
import { LanguagePickerPopover } from '@/components/LanguagePickerPopover';
import { useRecentLanguages } from '@/hooks/useRecentLanguages';
import { labelForCode } from '@/lib/summary-languages';
import {
  readMeetingSummaryLanguage,
  saveMeetingSummaryLanguage,
  SummaryLanguageStorage,
} from '@/lib/summary-language-preferences';

interface SummaryPanelProps {
  meeting: {
    id: string;
    title: string;
    created_at: string;
  };
  meetingTitle: string;
  onTitleChange: (title: string) => void;
  isEditingTitle: boolean;
  onStartEditTitle: () => void;
  onFinishEditTitle: () => void;
  isTitleDirty: boolean;
  summaryRef: RefObject<BlockNoteSummaryViewRef>;
  isSaving: boolean;
  onSaveAll: () => Promise<void>;
  onCopySummary: () => Promise<void>;
  onExportSummaryMarkdown?: () => void;
  onExportSummaryTxt?: () => void;
  onOpenFolder: () => Promise<void>;
  aiSummary: Summary | null;
  summaryStatus: 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error';
  transcripts: Transcript[];
  modelConfig: ModelConfig;
  setModelConfig: (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => void;
  onSaveModelConfig: (config?: ModelConfig) => Promise<void>;
  onGenerateSummary: (customPrompt: string) => Promise<void>;
  onStopGeneration: () => void;
  customPrompt: string;
  summaryResponse: SummaryResponse | null;
  onSaveSummary: (summary: Summary | { markdown?: string; summary_json?: any[] }) => Promise<void>;
  onSummaryChange: (summary: Summary) => void;
  onDirtyChange: (isDirty: boolean) => void;
  summaryError: string | null;
  onRegenerateSummary: () => Promise<void>;
  getSummaryStatusMessage: (status: 'idle' | 'processing' | 'summarizing' | 'regenerating' | 'completed' | 'error') => string;
  availableTemplates: Array<{ id: string; name: string; description: string; required_tier?: 'free' | 'member' }>;
  selectedTemplate: string;
  /// §123: 当前选中模板的显示名 (按钮里展示)
  selectedTemplateName?: string;
  onTemplateSelect: (templateId: string, templateName: string) => void;
  isModelConfigLoading?: boolean;
  onOpenModelSettings?: (openFn: () => void) => void;
}

export function SummaryPanel({
  meeting,
  meetingTitle,
  onTitleChange,
  isEditingTitle,
  onStartEditTitle,
  onFinishEditTitle,
  isTitleDirty,
  summaryRef,
  isSaving,
  onSaveAll,
  onCopySummary,
  onExportSummaryMarkdown,
  onExportSummaryTxt,
  onOpenFolder,
  aiSummary,
  summaryStatus,
  transcripts,
  modelConfig,
  setModelConfig,
  onSaveModelConfig,
  onGenerateSummary,
  onStopGeneration,
  customPrompt,
  summaryResponse,
  onSaveSummary,
  onSummaryChange,
  onDirtyChange,
  summaryError,
  onRegenerateSummary,
  getSummaryStatusMessage,
  availableTemplates,
  selectedTemplate,
  selectedTemplateName,
  onTemplateSelect,
  isModelConfigLoading = false,
  onOpenModelSettings
}: SummaryPanelProps) {
  const { t, locale } = useTranslation();
  const [summaryLang, setSummaryLang] = useState<string | null>(null);
  const [summaryLangStorage, setSummaryLangStorage] = useState<SummaryLanguageStorage>('metadata');
  const [speakerDrawerOpen, setSpeakerDrawerOpen] = useState(false);
  const [langPickerOpen, setLangPickerOpen] = useState(false);
  const [streamedMarkdown, setStreamedMarkdown] = useState('');
  // v0.7.0+ P0-1: Map-Reduce 阶段显示
  const [summaryPhase, setSummaryPhase] = useState<'idle'|'single'|'map'|'reduce'|'final'>('idle');
  // §135: 历史摘要弹窗
  const [historyOpen, setHistoryOpen] = useState(false);
  // §128: 让"摘要设置 → AI 模型" 真正打开 ModelSettingsModal 对话框 (而不是空回调)
  const [modelSettingsDialogOpen, setModelSettingsDialogOpen] = useState(false);
  const languageLoadVersionRef = useRef(0);
  const activeMeetingIdRef = useRef(meeting.id);
  const languageSaveVersionRef = useRef(0);
  const languageSaveLoopRunningRef = useRef(false);

  useEffect(() => {
    setStreamedMarkdown('');
    let unlisten: (() => void) | undefined;
    void listen<{ meeting_id: string; delta: string }>('summary-stream', (event) => {
      if (event.payload.meeting_id !== meeting.id) return;
      setStreamedMarkdown((current) => current + event.payload.delta);
    }).then((dispose) => {
      unlisten = dispose;
    });
    return () => unlisten?.();
  }, [meeting.id]);

  useEffect(() => {
    setSummaryPhase('idle');
    let unlisten: (() => void) | undefined;
    void listen<{ meeting_id: string; phase: 'single'|'map'|'reduce'|'final'; progress: number }>('summary-phase', (event) => {
      if (event.payload.meeting_id !== meeting.id) return;
      setSummaryPhase(event.payload.phase);
    }).then((dispose) => {
      unlisten = dispose;
    });
    return () => unlisten?.();
  }, [meeting.id]);

  useEffect(() => {
    if (summaryStatus === 'processing' || summaryStatus === 'regenerating') {
      setStreamedMarkdown('');
    }
  }, [summaryStatus]);

  useEffect(() => {
    if (summaryStatus === 'processing' || summaryStatus === 'regenerating') {
      setSummaryPhase('single');
    } else if (summaryStatus === 'idle' || summaryStatus === 'completed') {
      setSummaryPhase('idle');
    }
  }, [summaryStatus]);
  const latestLanguageSaveRequestRef = useRef<{
    version: number;
    meetingId: string;
    language: string | null;
    rollback: {
      language: string | null;
      storage: SummaryLanguageStorage;
    };
  } | null>(null);
  activeMeetingIdRef.current = meeting.id;
  const { addRecent } = useRecentLanguages();

  const effectiveLangLabel = summaryLang
    ? (locale === 'zh' && summaryLang === 'zh' ? '中文' : labelForCode(summaryLang))
    : t('language_picker.auto_detect');
  const isLocalFallbackLanguage = summaryLangStorage === 'local_fallback';
  const autoSubtitle = isLocalFallbackLanguage
    ? t('language_picker.auto_saved_local')
    : t('language_picker.auto_dominant_transcript');

  useEffect(() => {
    let cancelled = false;
    const loadVersion = languageLoadVersionRef.current + 1;
    languageLoadVersionRef.current = loadVersion;

    const loadSummaryLanguage = async () => {
      try {
        const stored = await readMeetingSummaryLanguage(meeting.id);
        if (!cancelled && languageLoadVersionRef.current === loadVersion) {
          setSummaryLang(stored.language);
          setSummaryLangStorage(stored.storage);
        }
      } catch (err) {
        console.error('Failed to load summary language:', err);
        safeToast.warning('Could not load saved summary language', {
          description: 'Using Auto until meeting metadata can be read.',
        });
        if (!cancelled && languageLoadVersionRef.current === loadVersion) setSummaryLang(null);
      }
    };

    loadSummaryLanguage();

    return () => {
      cancelled = true;
    };
  }, [meeting.id]);

  const persistLatestLanguageSelection = async () => {
    if (languageSaveLoopRunningRef.current) return;
    languageSaveLoopRunningRef.current = true;

    try {
      while (true) {
        const request = latestLanguageSaveRequestRef.current;
        if (!request) return;

        try {
          const saved = await saveMeetingSummaryLanguage(request.meetingId, request.language);
          const latest = latestLanguageSaveRequestRef.current;
          if (
            latest?.version === request.version &&
            activeMeetingIdRef.current === request.meetingId
          ) {
            setSummaryLang(saved.language);
            setSummaryLangStorage(saved.storage);
            if (saved.storage === 'local_fallback') {
              safeToast.info(t('language_picker.saved_local'), {
                description: t('language_picker.saved_local_desc'),
              });
            }
            if (request.language) {
              addRecent(request.language);
            }
            return;
          }

          if (latest?.version === request.version) return;
        } catch (err) {
          const latest = latestLanguageSaveRequestRef.current;
          if (
            latest?.version === request.version &&
            activeMeetingIdRef.current === request.meetingId
          ) {
            console.error('Failed to persist summary language:', err);
            safeToast.error(t('language_picker.save_failed'));
            setSummaryLang(request.rollback.language);
            setSummaryLangStorage(request.rollback.storage);
            return;
          }

          console.warn('Ignoring failed stale summary language save:', err);
          if (latest?.version === request.version) return;
        }
      }
    } finally {
      languageSaveLoopRunningRef.current = false;
    }
  };

  const handleLangChange = (code: string | null) => {
    const previous = summaryLang;
    const previousStorage = summaryLangStorage;
    const nextStored = code;
    languageLoadVersionRef.current += 1;
    latestLanguageSaveRequestRef.current = {
      version: languageSaveVersionRef.current + 1,
      meetingId: meeting.id,
      language: nextStored,
      rollback: {
        language: previous,
        storage: previousStorage,
      },
    };
    languageSaveVersionRef.current += 1;
    setSummaryLang(nextStored);
    setLangPickerOpen(false);
    void persistLatestLanguageSelection();
  };

  const isSummaryLoading = summaryStatus === 'processing' || summaryStatus === 'summarizing' || summaryStatus === 'regenerating';

  const languageSlot = (
    <Popover open={langPickerOpen} onOpenChange={setLangPickerOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          size="sm"
          title={t('language_picker.summary_language_title', { language: effectiveLangLabel })}
          aria-label={t('language_picker.aria_label')}
        >
          <Languages size={18} />
          <span className="hidden lg:inline">{effectiveLangLabel}</span>
          <ChevronDown size={14} className="text-gray-400" />
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        className="w-auto p-0 border-0 shadow-none bg-transparent"
      >
        <LanguagePickerPopover
          value={summaryLang}
          onChange={handleLangChange}
          onClose={() => setLangPickerOpen(false)}
          autoSubtitle={autoSubtitle}
        />
      </PopoverContent>
    </Popover>
  );

  return (
    <div className="flex-1 min-w-0 flex flex-col bg-white overflow-hidden">
      {/* Title area */}
      <div className="p-4 border-b border-gray-200">
        {/* <EditableTitle
          title={meetingTitle}
          isEditing={isEditingTitle}
          onStartEditing={onStartEditTitle}
          onFinishEditing={onFinishEditTitle}
          onChange={onTitleChange}
        /> */}

        {/* §110: 9 按钮 → 4 元素 (说话人/重新生成/⚙️ 设置下拉/📤 导出下拉) */}
        {!isSummaryLoading && (
          <div className="flex items-center justify-center w-full pt-0 gap-2">
            {/* 1. 说话人名单 — 仅在已有摘要时显示 (没摘要就没对话过说话人) */}
            {aiSummary && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setSpeakerDrawerOpen(true)}
                className="flex items-center gap-2 ml-2"
                data-testid="open-speaker-roster"
              >
                <Users className="w-4 h-4" />
                {t('speaker.title')}
              </Button>
            )}
            {/* 2. 主操作 — 生成 / 重新生成 / 停止 (按状态切换). §124 统一按钮文本 */}
            {isSummaryLoading ? (
              <Button
                variant="outline"
                size="sm"
                className="bg-gradient-to-r from-red-50 to-orange-50 hover:from-red-100 hover:to-orange-100 border-red-200 xl:px-4"
                onClick={() => {
                  Analytics.trackButtonClick('stop_summary_generation', 'meeting_details');
                  onStopGeneration();
                }}
                title={t('summary.stop_generation')}
              >
                <Square className="w-4 h-4" fill="currentColor" />
                <span className="hidden lg:inline">{t('summary.stop')}</span>
              </Button>
            ) : aiSummary ? (
              <Button
                variant="outline"
                size="sm"
                className="bg-gradient-to-r from-blue-50 to-purple-50 hover:from-blue-100 hover:to-purple-100 border-blue-200 xl:px-4"
                onClick={() => {
                  Analytics.trackButtonClick('regenerate_summary_header', 'meeting_details');
                  onRegenerateSummary();
                }}
                title={t('summary.regenerate')}
              >
                <Sparkles className="w-4 h-4" />
                <span className="hidden lg:inline">{t('summary.regenerate')}</span>
              </Button>
            ) : (
              <Button
                variant="outline"
                size="sm"
                className="bg-gradient-to-r from-blue-50 to-purple-50 hover:from-blue-100 hover:to-purple-100 border-blue-200 xl:px-4"
                onClick={() => {
                  Analytics.trackButtonClick('generate_summary', 'meeting_details');
                  onGenerateSummary(customPrompt);
                }}
                disabled={!transcripts || transcripts.length === 0}
                title={t('summary.generate')}
              >
                <Sparkles className="w-4 h-4" />
                <span className="hidden lg:inline">{t('summary.generate')}</span>
              </Button>
            )}

            {/* 3. ⚙️ 设置下拉 — 自动检测 / AI 模型 / 模板 (§124 加 disabled 状态) */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="outline"
                  size="sm"
                  title={t('summary.settings_title')}
                  disabled={isSummaryLoading}
                >
                  <Settings className="w-4 h-4" />
                  <span className="hidden lg:inline">{t('summary.settings_title')}</span>
                  <ChevronDown className="w-3 h-3 ml-1 opacity-50" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-56">
                <DropdownMenuItem onSelect={(e) => e.preventDefault()} className="cursor-default p-0">
                  <div className="flex items-center w-full">
                    <Languages className="w-4 h-4 mr-2" />
                    {languageSlot}
                  </div>
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                {/* §128: AI 模型 → 真正弹出 ModelSettingsModal Dialog */}
                <DropdownMenuItem onSelect={(e) => { e.preventDefault(); setModelSettingsDialogOpen(true); }}>
                  <Sparkles className="w-4 h-4 mr-2" />
                  {t('summary.ai_model')}
                  <span className="ml-auto text-[10px] text-neutral-400 truncate max-w-[100px]">
                    {modelConfig?.model || '—'}
                  </span>
                </DropdownMenuItem>
                {/* §128: 模板 → SubMenu 列出全部 availableTemplates 让用户逐一选择 (之前硬编 templates[0]) */}
                <DropdownMenuSub>
                  <DropdownMenuSubTrigger>
                    <FileType className="w-4 h-4 mr-2" />
                    <span className="truncate max-w-[140px]">
                      {selectedTemplateName || t('summary.template')}
                    </span>
                  </DropdownMenuSubTrigger>
                  <DropdownMenuSubContent className="w-64">
                    <DropdownMenuRadioGroup
                      value={selectedTemplate}
                      onValueChange={(v) => {
                        const tmpl = availableTemplates.find(t => t.id === v);
                        if (tmpl) onTemplateSelect(tmpl.id, tmpl.name);
                      }}
                    >
                      {availableTemplates.length === 0 && (
                        <DropdownMenuItem disabled>
                          <Loader2 className="w-3.5 h-3.5 mr-2 animate-spin" />
                          {t('summary.loading_templates')}
                        </DropdownMenuItem>
                      )}
                      {availableTemplates.map((template) => (
                        <DropdownMenuRadioItem
                          key={template.id}
                          value={template.id}
                          title={template.description}
                          className="text-[12.5px]"
                        >
                          <span className="flex items-center gap-1.5 truncate">
                            {template.name}
                            {template.required_tier === 'member' && (
                              <span className="text-[10px] font-medium px-1.5 py-0.5 rounded bg-amber-100 text-amber-700 border border-amber-200">
                                PRO
                              </span>
                            )}
                          </span>
                        </DropdownMenuRadioItem>
                      ))}
                    </DropdownMenuRadioGroup>
                  </DropdownMenuSubContent>
                </DropdownMenuSub>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* §128: model settings dialog — 直接挂 Modal 在按钮组下方, 避免空回调 */}
            <Dialog open={modelSettingsDialogOpen} onOpenChange={setModelSettingsDialogOpen}>
              <DialogContent aria-describedby={undefined} className="max-w-3xl max-h-[88vh] overflow-y-auto">
                <VisuallyHidden>
                  <DialogTitle>{t('summary.model_settings')}</DialogTitle>
                </VisuallyHidden>
                <ModelSettingsModal
                  modelConfig={modelConfig}
                  setModelConfig={setModelConfig}
                  onSave={async (config) => {
                    await onSaveModelConfig(config);
                    setModelSettingsDialogOpen(false);
                  }}
                  skipInitialFetch={true}
                  layout="dialog"
                />
              </DialogContent>
            </Dialog>

            {/* 4. 📤 导出下拉 — 保存 / 复制 / MD / TXT / 打开文件夹 (§124: 没摘要时 disabled 各项) */}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="outline"
                  size="sm"
                  title={t('summary.export_md_title')}
                  disabled={isSummaryLoading}
                >
                  <Download className="w-4 h-4" />
                  <span className="hidden lg:inline">{t('meeting_details.export_md')}</span>
                  <ChevronDown className="w-3 h-3 ml-1 opacity-50" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-44">
                <DropdownMenuItem onClick={onSaveAll} disabled={isSaving}>
                  <Save className="w-4 h-4 mr-2" />
                  {t('summary.save')}
                  {(isTitleDirty || (summaryRef.current?.isDirty || false)) && (
                    <span className="ml-auto w-2 h-2 rounded-full bg-green-500" />
                  )}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={onCopySummary} disabled={!aiSummary}>
                  <Copy className="w-4 h-4 mr-2" />
                  {t('summary.copy')}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={onExportSummaryMarkdown} disabled={!aiSummary}>
                  <FileCode className="w-4 h-4 mr-2" />
                  {t('summary.export_md')}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={onExportSummaryTxt} disabled={!aiSummary}>
                  <FileText className="w-4 h-4 mr-2" />
                  {t('summary.export_txt')}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={onOpenFolder}>
                  <FolderOpen className="w-4 h-4 mr-2" />
                  {t('meeting_details.open_folder')}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        )}
      </div>

      {isSummaryLoading ? (
        <div className="flex-1 min-h-0 overflow-y-auto px-6 pb-6">
          {/* §124: Loading 状态下, 主区只显示 streaming markdown / spinner (按钮已统一在顶部) */}
            {streamedMarkdown ? (
              <div className="mx-auto max-w-4xl rounded-xl border border-blue-100 bg-blue-50/40 p-5">
                <div className="mb-3 flex items-center gap-2 text-xs font-medium text-blue-700">
                  <span className="h-2 w-2 animate-pulse rounded-full bg-blue-500" />
                  {locale === 'zh' ? '本地模型正在生成' : 'Local model is generating'}
                </div>
                <pre className="whitespace-pre-wrap break-words font-sans text-sm leading-7 text-gray-800">{streamedMarkdown}</pre>
              </div>
            ) : (
            <div className="flex h-full items-center justify-center">
            <div className="text-center">
              <div className="inline-block animate-spin rounded-full h-12 w-12 border-t-2 border-b-2 border-blue-500 mb-4"></div>
              <p className="text-gray-600">{t('summary.generating')}</p>
            </div>
            </div>
            )}
          </div>
      ) : !aiSummary ? (
        <div className="flex-1 min-h-0 overflow-y-auto px-6 pb-6 pt-8">
          {/* §124: 首次生成状态下, 按钮已统一在顶部. 主区只显示 EmptyState 引导用户去顶部按钮 */}
          <EmptyStateSummary
            onGenerate={() => onGenerateSummary(customPrompt)}
            hasModel={modelConfig.provider !== null && modelConfig.model !== null}
            isGenerating={isSummaryLoading}
          />
        </div>
      ) : transcripts?.length > 0 && (
        <div className="flex-1 overflow-y-auto min-h-0">
          {/* §135: 当前模板徽章 + 历史摘要按钮 (用户一眼看到本次生成用的什么模板) */}
          <div className="mx-6 mt-4 flex items-center justify-between gap-2">
            <div className="inline-flex items-center gap-1.5 rounded-full bg-violet-50 px-3 py-1 text-xs font-medium text-violet-700 border border-violet-200">
              <FileType className="h-3.5 w-3.5" />
              <span>{t('summary.current_template_badge')}: {selectedTemplateName || t('summary.template')}</span>
            </div>
            <button
              onClick={() => setHistoryOpen(true)}
              className="inline-flex items-center gap-1.5 rounded-full border border-neutral-200 bg-white px-3 py-1 text-xs font-medium text-neutral-600 hover:bg-neutral-50"
            >
              <History className="h-3.5 w-3.5" />
              {t('summary.history_button')}
            </button>
          </div>
          <SummaryHistoryPanel
            meetingId={meeting.id}
            currentTemplateName={selectedTemplateName}
            open={historyOpen}
            onClose={() => setHistoryOpen(false)}
            onLoadHistory={(historyId) => {
              window.dispatchEvent(new CustomEvent('summary-history-load', { detail: { historyId, meetingId: meeting.id } }));
              setHistoryOpen(false);
            }}
          />
          {summaryResponse && (
            <div className="fixed bottom-0 left-0 right-0 bg-white shadow-lg p-4 max-h-1/3 overflow-y-auto">
              <h3 className="text-lg font-semibold mb-2">{t('summary.title')}</h3>
              <div className="grid grid-cols-2 gap-4">
                <div className="bg-white p-4 rounded-lg shadow-sm">
                  <h4 className="font-medium mb-1">{t('summary.key_points')}</h4>
                  <ul className="list-disc pl-4">
                    {summaryResponse.summary.key_points.blocks.map((block, i) => (
                      <li key={i} className="text-sm">{block.content}</li>
                    ))}
                  </ul>
                </div>
                <div className="bg-white p-4 rounded-lg shadow-sm mt-4">
                  <h4 className="font-medium mb-1">{t('summary.action_items')}</h4>
                  <ActionItemsList meetingId={meeting.id} />
                </div>
                <div className="bg-white p-4 rounded-lg shadow-sm mt-4">
                  <h4 className="font-medium mb-1">{t('summary.decisions')}</h4>
                  <ul className="list-disc pl-4">
                    {summaryResponse.summary.decisions.blocks.map((block, i) => (
                      <li key={i} className="text-sm">{block.content}</li>
                    ))}
                  </ul>
                </div>
                <div className="bg-white p-4 rounded-lg shadow-sm mt-4">
                  <h4 className="font-medium mb-1">{t('summary.topics')}</h4>
                  <ul className="list-disc pl-4">
                    {summaryResponse.summary.main_topics.blocks.map((block, i) => (
                      <li key={i} className="text-sm">{block.content}</li>
                    ))}
                  </ul>
                </div>
              </div>
              {summaryResponse.raw_summary ? (
                <div className="mt-4">
                  <h4 className="font-medium mb-1">{t('summary.full_summary')}</h4>
                  <p className="text-sm whitespace-pre-wrap">{summaryResponse.raw_summary}</p>
                </div>
              ) : null}
            </div>
          )}
          <div className="p-6 w-full">
            <BlockNoteSummaryView
              ref={summaryRef}
              summaryData={aiSummary}
              onSave={onSaveSummary}
              onSummaryChange={onSummaryChange}
              onDirtyChange={onDirtyChange}
              status={summaryStatus}
              error={summaryError}
              onRegenerateSummary={() => {
                Analytics.trackButtonClick('regenerate_summary', 'meeting_details');
                onRegenerateSummary();
              }}
              meeting={{
                id: meeting.id,
                title: meetingTitle,
                created_at: meeting.created_at
              }}
            />
          </div>
          {summaryStatus !== 'idle' && (
            <div className={`mt-4 p-4 rounded-lg ${summaryStatus === 'error' ? 'bg-red-100 text-red-700' :
              summaryStatus === 'completed' ? 'bg-green-100 text-green-700' :
                'bg-blue-100 text-blue-700'
              }`}>
              <p className="text-sm font-medium">{getSummaryStatusMessage(summaryStatus)}</p>
            </div>
          )}
        </div>
      )}
      <SpeakerRosterDrawer
        open={speakerDrawerOpen}
        onOpenChange={setSpeakerDrawerOpen}
        meetingId={meeting.id}
      />
    </div>
  );
}
