'use client';

import React, { useState, useMemo, useEffect, useCallback } from 'react';
import {
  ChevronDown, ChevronRight, File, Settings, ChevronLeftCircle,
  BookOpen, ChevronRightCircle, Calendar, StickyNote, Home, Trash2, Mic, Square, Plus, Search, Pencil, NotebookPen, SearchIcon, X, Upload
} from 'lucide-react';
import { FeedbackDialog } from '@/components/FeedbackDialog';
import { useRouter, usePathname } from 'next/navigation';
import { useSidebar } from './SidebarProvider';
import { useTranslation } from '@/i18n';
import { openExternalUrl } from '@/lib/openExternalUrl';
import { useAuth } from '@/contexts/AuthContext';
import type { CurrentMeeting } from '@/components/Sidebar/SidebarProvider';
import { ConfirmationModal } from '../ConfirmationModel/confirmation-modal';
import { ModelConfig } from '@/components/ModelSettingsModal';
import { SettingTabs } from '../SettingTabs';
import Image from 'next/image';
import { TranscriptModelProps } from '@/components/TranscriptSettings';
import Analytics from '@/lib/analytics';
import { invoke } from '@tauri-apps/api/core';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useImportDialog } from '@/contexts/ImportDialogContext';
import { useConfig } from '@/contexts/ConfigContext';

import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogTitle,
} from "@/components/ui/dialog"
import { VisuallyHidden } from "@/components/ui/visually-hidden"

import { MessageToast } from '../MessageToast';
import Info from '../Info';
import { ComplianceNotification } from '../ComplianceNotification';
import { Input } from '../ui/input';
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput } from '../ui/input-group';

interface SidebarItem {
  id: string;
  title: string;
  type: 'folder' | 'file';
  children?: SidebarItem[];
}

const Sidebar: React.FC = () => {
  const router = useRouter();
  const pathname = usePathname();
  const {
    currentMeeting,
    setCurrentMeeting,
    sidebarItems,
    isCollapsed,
    toggleCollapse,
    handleRecordingToggle,
    searchTranscripts,
    searchResults,
    isSearching,
    meetings,
    setMeetings,
    serverAddress
  } = useSidebar();
  const { user, logout } = useAuth();
  const { locale, t } = useTranslation();
  // §105: 旧 DB 数据 title 是 "Recording in progress (Untitled)" / "Untitled" / 空, 渲染时本地化
  const meetingTitle = (raw: string) => {
    if (!raw || raw.trim() === '' || raw === 'Untitled' || raw.startsWith('Recording in progress (')) {
      return t('meeting.untitled') || '未命名会议';
    }
    return raw;
  };

  // Get recording state from RecordingStateContext (single source of truth)
  const { isRecording } = useRecordingState();

  // v0.6.7: 监听 Topbar 派发的事件
  useEffect(() => {
    const recHandler = () => { handleRecordingToggle(); };
    const searchHandler = (e: Event) => {
      const q = (e as CustomEvent<string>).detail || '';
      setSearchQuery(q);
      if (q) searchTranscripts(q);
    };
    window.addEventListener('lixianhuiji:toggle-recording', recHandler);
    window.addEventListener('lixianhuiji:search-query', searchHandler);
    return () => {
      window.removeEventListener('lixianhuiji:toggle-recording', recHandler);
      window.removeEventListener('lixianhuiji:search-query', searchHandler);
    };
  }, [handleRecordingToggle, searchTranscripts]);
  const { openImportDialog } = useImportDialog();
  const { betaFeatures } = useConfig();
  const [expandedFolders, setExpandedFolders] = useState<Set<string>>(new Set(['meetings']));
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [showModelSettings, setShowModelSettings] = useState(false);
  const [feedbackOpen, setFeedbackOpen] = useState(false);
  const [modelConfig, setModelConfig] = useState<ModelConfig>({
    provider: 'ollama',
    model: '',
    whisperModel: '',
    apiKey: null,
    ollamaEndpoint: null
  });
  const [transcriptModelConfig, setTranscriptModelConfig] = useState<TranscriptModelProps>({
    provider: 'parakeet',
    model: 'parakeet-tdt-0.6b-v3-int8',
  });
  const [settingsSaveSuccess, setSettingsSaveSuccess] = useState<boolean | null>(null);
  const [totalTopics, setTotalTopics] = useState(0);

  // State for edit modal
  const [editModalState, setEditModalState] = useState<{ isOpen: boolean; meetingId: string | null; currentTitle: string }>({
    isOpen: false,
    meetingId: null,
    currentTitle: ''
  });
  const [editingTitle, setEditingTitle] = useState<string>('');

  // Ensure 'meetings' folder is always expanded
  useEffect(() => {
    if (!expandedFolders.has('meetings')) {
      const newExpanded = new Set(expandedFolders);
      newExpanded.add('meetings');
      setExpandedFolders(newExpanded);
    }
  }, [expandedFolders]);

  // useEffect(() => {
  //   if (settingsSaveSuccess !== null) {
  //     const timer = setTimeout(() => {
  //       setSettingsSaveSuccess(null);
  //     }, 3000);
  //   }
  // }, [settingsSaveSuccess]);


  const [deleteModalState, setDeleteModalState] = useState<{ isOpen: boolean; itemId: string | null }>({ isOpen: false, itemId: null });

  // P0-A: load topic count for sidebar badge
  useEffect(() => {
    const loadTopics = async () => {
      try {
        const list = await invoke('api_topic_recent', { limit: 50 });
        if (Array.isArray(list)) setTotalTopics(list.length);
      } catch {
        /* no-op */
      }
    };
    void loadTopics();
    const interval = setInterval(loadTopics, 60000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    // Note: Don't set hardcoded defaults - let DB be the source of truth
    const fetchModelConfig = async () => {
      // Only make API call if serverAddress is loaded
      if (!serverAddress) {
        console.log('Waiting for server address to load before fetching model config');
        return;
      }

      try {
        const data = await invoke('api_get_model_config') as any;
        if (data && data.provider !== null) {
          // Fetch API key if not included and provider requires it
          if (data.provider !== 'ollama' && !data.apiKey) {
            try {
              const apiKeyData = await invoke('api_get_api_key', {
                provider: data.provider
              }) as string;
              data.apiKey = apiKeyData;
            } catch (err) {
              console.error('Failed to fetch API key:', err);
            }
          }
          setModelConfig(data);
        }
      } catch (error) {
        console.error('Failed to fetch model config:', error);
      }
    };

    fetchModelConfig();
  }, [serverAddress]);


  useEffect(() => {
    // Note: Don't set hardcoded defaults - let DB be the source of truth
    const fetchTranscriptSettings = async () => {
      // Only make API call if serverAddress is loaded
      if (!serverAddress) {
        console.log('Waiting for server address to load before fetching transcript settings');
        return;
      }

      try {
        const data = await invoke('api_get_transcript_config') as any;
        if (data && data.provider !== null) {
          setTranscriptModelConfig(data);
        }
      } catch (error) {
        console.error('Failed to fetch transcript settings:', error);
      }
    };
    fetchTranscriptSettings();
  }, [serverAddress]);

  // Listen for model config updates from other components
  useEffect(() => {
    const setupListener = async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen<ModelConfig>('model-config-updated', (event) => {
        console.log('Sidebar received model-config-updated event:', event.payload);
        setModelConfig(event.payload);
      });

      return unlisten;
    };

    let cleanup: (() => void) | undefined;
    setupListener().then(fn => cleanup = fn);

    return () => {
      cleanup?.();
    };
  }, []);



  // Handle model config save
  const handleSaveModelConfig = async (config: ModelConfig) => {
    try {
      await invoke('api_save_model_config', {
        provider: config.provider,
        model: config.model,
        whisperModel: config.whisperModel,
        apiKey: config.apiKey,
        ollamaEndpoint: config.ollamaEndpoint,
      });

      setModelConfig(config);
      console.log('Model config saved successfully');
      setSettingsSaveSuccess(true);

      // Emit event to sync other components
      const { emit } = await import('@tauri-apps/api/event');
      await emit('model-config-updated', config);

      // Track settings change
      await Analytics.trackSettingsChanged('model_config', `${config.provider}_${config.model}`);
    } catch (error) {
      console.error('Error saving model config:', error);
      setSettingsSaveSuccess(false);
    }
  };

  const handleSaveTranscriptConfig = async (updatedConfig?: TranscriptModelProps) => {
    try {
      const configToSave = updatedConfig || transcriptModelConfig;
      const payload = {
        provider: configToSave.provider,
        model: configToSave.model,
        apiKey: configToSave.apiKey ?? null
      };
      console.log('Saving transcript config with payload:', payload);

      await invoke('api_save_transcript_config', {
        provider: payload.provider,
        model: payload.model,
        apiKey: payload.apiKey,
      });


      setSettingsSaveSuccess(true);

      // Track settings change
      const transcriptConfigToSave = updatedConfig || transcriptModelConfig;
      await Analytics.trackSettingsChanged('transcript_config', `${transcriptConfigToSave.provider}_${transcriptConfigToSave.model}`);
    } catch (error) {
      console.error('Failed to save transcript config:', error);
      setSettingsSaveSuccess(false);
    }
  };

  // Handle search input changes
  const handleSearchChange = useCallback(async (value: string) => {
    setSearchQuery(value);

    // If search query is empty, just return to normal view
    if (!value.trim()) return;

    // Search through transcripts
    await searchTranscripts(value);

    // Make sure the meetings folder is expanded when searching
    if (!expandedFolders.has('meetings')) {
      const newExpanded = new Set(expandedFolders);
      newExpanded.add('meetings');
      setExpandedFolders(newExpanded);
    }
  }, [expandedFolders, searchTranscripts]);

  // Combine search results with sidebar items
  const filteredSidebarItems = useMemo(() => {
    if (!searchQuery.trim()) return sidebarItems;

    // If we have search results, highlight matching meetings
    if (searchResults.length > 0) {
      // Get the IDs of meetings that matched in transcripts
      const matchedMeetingIds = new Set(searchResults.map(result => result.id));

      return sidebarItems
        .map(folder => {
          // Always include folders in the results
          if (folder.type === 'folder') {
            if (!folder.children) return folder;

            // Filter children based on search results or title match
            const filteredChildren = folder.children.filter(item => {
              // Include if the meeting ID is in our search results
              if (matchedMeetingIds.has(item.id)) return true;

              // Or if the title matches the search query
              return item.title.toLowerCase().includes(searchQuery.toLowerCase());
            });

            return {
              ...folder,
              children: filteredChildren
            };
          }

          // For non-folder items, check if they match the search
          return (matchedMeetingIds.has(folder.id) ||
            folder.title.toLowerCase().includes(searchQuery.toLowerCase()))
            ? folder : undefined;
        })
        .filter((item): item is SidebarItem => item !== undefined); // Type-safe filter
    } else {
      // Fall back to title-only filtering if no transcript results
      return sidebarItems
        .map(folder => {
          // Always include folders in the results
          if (folder.type === 'folder') {
            if (!folder.children) return folder;

            // Filter children based on search query
            const filteredChildren = folder.children.filter(item =>
              item.title.toLowerCase().includes(searchQuery.toLowerCase())
            );

            return {
              ...folder,
              children: filteredChildren
            };
          }

          // For non-folder items, check if they match the search
          return folder.title.toLowerCase().includes(searchQuery.toLowerCase()) ? folder : undefined;
        })
        .filter((item): item is SidebarItem => item !== undefined); // Type-safe filter
    }
  }, [sidebarItems, searchQuery, searchResults, expandedFolders]);


  const handleDelete = async (itemId: string) => {
    console.log('Deleting item:', itemId);
    const payload = {
      meetingId: itemId
    };

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('api_delete_meeting', {
        meetingId: itemId,
      });
      console.log('Meeting deleted successfully');
      const updatedMeetings = meetings.filter((m: CurrentMeeting) => m.id !== itemId);
      setMeetings(updatedMeetings);

      // Track meeting deletion
      Analytics.trackMeetingDeleted(itemId);

      // Show success toast
      safeToast.success("会议 deleted successfully", {
        description: "关联数据已全部删除"
      });

      // If deleting the active meeting, navigate to Home
      if (currentMeeting?.id === itemId) {
        setCurrentMeeting({ id: 'intro-call', title: '+ New Call' });
        router.push('/');
      }
    } catch (error) {
      console.error('删除会议失败:', error);
      safeToast.error("删除会议失败", {
        description: error instanceof Error ? error.message : String(error)
      });
    }
  };

  const handleDeleteConfirm = () => {
    if (deleteModalState.itemId) {
      handleDelete(deleteModalState.itemId);
    }
    setDeleteModalState({ isOpen: false, itemId: null });
  };

  // Handle modal editing of meeting names
  const handleEditStart = (meetingId: string, currentTitle: string) => {
    setEditModalState({
      isOpen: true,
      meetingId: meetingId,
      currentTitle: currentTitle
    });
    setEditingTitle(currentTitle);
  };

  const handleEditConfirm = async () => {
    const newTitle = editingTitle.trim();
    const meetingId = editModalState.meetingId;

    if (!meetingId) return;

    // Prevent empty titles
    if (!newTitle) {
      safeToast.error("会议 title cannot be empty");
      return;
    }

    try {
      await invoke('api_save_meeting_title', {
        meetingId: meetingId,
        title: newTitle,
      });

      // Update local state
      const updatedMeetings = meetings.map((m: CurrentMeeting) =>
        m.id === meetingId ? { ...m, title: newTitle } : m
      );
      setMeetings(updatedMeetings);

      // Update current meeting if it's the one being edited
      if (currentMeeting?.id === meetingId) {
        setCurrentMeeting({ id: meetingId, title: newTitle });
      }

      // Track the edit
      Analytics.trackButtonClick('edit_meeting_title', 'sidebar');

      safeToast.success("会议 title updated successfully");

      // Close modal and reset state
      setEditModalState({ isOpen: false, meetingId: null, currentTitle: '' });
      setEditingTitle('');
    } catch (error) {
      console.error('更新会议标题失败:', error);
      safeToast.error("更新会议标题失败", {
        description: error instanceof Error ? error.message : String(error)
      });
    }
  };

  const handleEditCancel = () => {
    setEditModalState({ isOpen: false, meetingId: null, currentTitle: '' });
    setEditingTitle('');
  };

  const toggleFolder = (folderId: string) => {
    // Normal toggle behavior for all folders
    const newExpanded = new Set(expandedFolders);
    if (newExpanded.has(folderId)) {
      newExpanded.delete(folderId);
    } else {
      newExpanded.add(folderId);
    }
    setExpandedFolders(newExpanded);
  };

  // Expose setShowModelSettings to window for Rust tray to call
  useEffect(() => {
    (window as any).openSettings = () => {
      setShowModelSettings(true);
    };

    // Cleanup on unmount
    return () => {
      delete (window as any).openSettings;
    };
  }, []);

  const renderCollapsedIcons = () => {
    if (!isCollapsed) return null;

    const isHomePage = pathname === '/';
    const isMeetingPage = pathname?.includes('/meeting-details');
    const isSettingsPage = pathname === '/settings';

    return (
      <TooltipProvider>
        <div className="flex flex-col items-center space-y-4 mt-4">
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={() => router.push('/')}
                className={`p-2 rounded-lg transition-colors duration-150 ${isHomePage ? 'bg-gray-100' : 'hover:bg-gray-100'
                  }`}
              >
                <Home className="w-5 h-5 text-gray-600" />
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              {t('nav.home')}
            </TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={handleRecordingToggle}
                disabled={isRecording}
                className={`p-2 ${isRecording ? 'bg-red-500 cursor-not-allowed' : 'bg-red-500 hover:bg-red-600'} rounded-full transition-colors duration-150 shadow-sm`}
              >
                {isRecording ? (
                  <Square className="w-5 h-5 text-white" />
                ) : (
                  <Mic className="w-5 h-5 text-white" />
                )}
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              {isRecording ? t('nav.recording_in_progress') : t('nav.start_recording') }
            </TooltipContent>
          </Tooltip>

          {betaFeatures.importAndRetranscribe && (
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  onClick={() => openImportDialog()}
                  className="p-2 rounded-lg transition-colors duration-150 hover:bg-blue-100 bg-blue-50"
                >
                  <Upload className="w-5 h-5 text-blue-600" />
                </button>
              </TooltipTrigger>
              <TooltipContent side="right">
                {t('nav.import_audio') }
              </TooltipContent>
            </Tooltip>
          )}

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={() => {
                  if (isCollapsed) toggleCollapse();
                  toggleFolder('meetings');
                }}
                className={`p-2 rounded-lg transition-colors duration-150 ${isMeetingPage ? 'bg-gray-100' : 'hover:bg-gray-100'
                  }`}
              >
                <NotebookPen className="w-5 h-5 text-gray-600" />
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              {t('nav.notes') }
            </TooltipContent>
          </Tooltip>

          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={() => router.push('/settings')}
                className={`p-2 rounded-lg transition-colors duration-150 ${isSettingsPage ? 'bg-gray-100' : 'hover:bg-gray-100'
                  }`}
              >
                <Settings className="w-5 h-5 text-gray-600" />
              </button>
            </TooltipTrigger>
            <TooltipContent side="right">
              {t('nav.settings') }
            </TooltipContent>
          </Tooltip>

          <Info isCollapsed={isCollapsed} />
        </div>
      </TooltipProvider>
    );
  };

  // Find matching transcript snippet for a meeting item
  const findMatchingSnippet = (itemId: string) => {
    if (!searchQuery.trim() || !searchResults.length) return null;
    return searchResults.find(result => result.id === itemId);
  };

  const renderItem = (item: SidebarItem, depth = 0) => {
    const isExpanded = expandedFolders.has(item.id);
    const paddingLeft = `${depth * 12 + 12}px`;
    const isActive = item.type === 'file' && currentMeeting?.id === item.id;
    const isMeetingItem = item.id.includes('-') && !item.id.startsWith('intro-call');

    // Check if this item has a matching transcript snippet
    const matchingResult = isMeetingItem ? findMatchingSnippet(item.id) : null;
    const hasTranscriptMatch = !!matchingResult;

    if (isCollapsed) return null;

    return (
      <div key={item.id}>
        <div
          className={`flex items-center transition-all duration-150 group ${item.type === 'folder' && depth === 0
            ? 'p-3 text-lg font-semibold h-10 mx-3 mt-3 rounded-lg'
            : `px-3 py-2 my-0.5 rounded-md text-sm ${isActive ? 'bg-blue-100 text-blue-700 font-medium' :
              hasTranscriptMatch ? 'bg-yellow-50' : 'hover:bg-gray-50'
            } cursor-pointer`
            }`}
          style={item.type === 'folder' && depth === 0 ? {} : { paddingLeft }}
          onClick={() => {
            if (item.type === 'folder') {
              toggleFolder(item.id);
            } else {
              setCurrentMeeting({ id: item.id, title: item.title });
              const basePath = item.id.startsWith('intro-call') ? '/' :
                item.id.includes('-') ? `/meeting-details?id=${item.id}` : `/notes/${item.id}`;
              router.push(basePath);
            }
          }}
        >
          {item.type === 'folder' ? (
            <>
              {item.id === 'meetings' ? (
                <Calendar className="w-4 h-4 mr-2" />
              ) : item.id === 'notes' ? (
                <Calendar className="w-4 h-4 mr-2" />
              ) : null}
              <span className={depth === 0 ? "" : "font-medium"}>{meetingTitle(item.title)}</span>
              <div className="ml-auto">
                {isExpanded ? (
                  <ChevronDown className="w-4 h-4 text-gray-500" />
                ) : (
                  <ChevronRight className="w-4 h-4 text-gray-500" />
                )}
              </div>
              {searchQuery && item.id === 'meetings' && isSearching && (
                <span className="ml-2 text-xs text-blue-500 animate-pulse">Searching...</span>
              )}
            </>
          ) : (
            <div className="flex flex-col w-full">
              <div className="flex items-center w-full">
                {isMeetingItem ? (
                  <div className="flex-shrink-0 flex items-center justify-center w-6 h-6 rounded-full mr-2 bg-gray-100">
                    <File className="w-3.5 h-3.5 text-gray-600" />
                  </div>
                ) : (
                  <div className="flex-shrink-0 flex items-center justify-center w-6 h-6 rounded-full mr-2 bg-blue-100">
                    <Plus className="w-3.5 h-3.5 text-blue-600" />
                  </div>
                )}
                <span className="flex-1 break-words">{meetingTitle(item.title)}</span>
                {isMeetingItem && (
                  <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity duration-150">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleEditStart(item.id, item.title);
                      }}
                      className="hover:text-blue-600 p-1 rounded-md hover:bg-blue-50 flex-shrink-0"
                      aria-label="编辑 meeting title"
                    >
                      <Pencil className="w-4 h-4" />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setDeleteModalState({ isOpen: true, itemId: item.id });
                      }}
                      className="hover:text-red-600 p-1 rounded-md hover:bg-red-50 flex-shrink-0"
                      aria-label="删除 meeting"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                )}
              </div>

              {/* Show transcript match snippet if available */}
              {hasTranscriptMatch && (
                <div className="mt-1 ml-8 text-xs text-gray-500 bg-yellow-50 p-1.5 rounded border border-yellow-100 line-clamp-2">
                  <span className="font-medium text-yellow-600">匹配片段:</span> {matchingResult.matchContext}
                </div>
              )}
            </div>
          )}
        </div>
        {item.type === 'folder' && isExpanded && item.children && (
          <div className="ml-1">
            {item.children.map(child => renderItem(child, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  // 离线会记 v0.5.0: auth route 不显示 sidebar, 全屏
  if (pathname === '/login' || pathname === '/register') return null;
  return (
    <>
      <aside
      className={`fixed top-0 left-0 h-screen z-40 flex flex-col border-r border-neutral-200/80 bg-white transition-[width] duration-300 ease-out dark:border-neutral-800 dark:bg-neutral-950 ${isCollapsed ? 'w-[68px]' : 'w-[252px]'}`}
    >
      {/* Collapse / expand handle */}
      <button
        onClick={toggleCollapse}
        aria-label={isCollapsed ? '展开侧栏' : '收起侧栏'}
        className="absolute -right-3 top-20 z-50 flex h-6 w-6 items-center justify-center rounded-full border border-neutral-200 bg-white text-neutral-500 shadow-sm transition-colors hover:bg-neutral-50 hover:text-neutral-800"
      >
        {isCollapsed ? (
          <ChevronRightCircle className="h-5 w-5" />
        ) : (
          <ChevronLeftCircle className="h-5 w-5" />
        )}
      </button>

      {/* Brand: 盾牌 + 文字 + v0.6.10 chip (展开态) / 单盾 (折叠态) */}
      <div className="flex h-14 items-center border-b border-neutral-200/70 px-3.5">
        {!isCollapsed ? (
          <div className="flex min-w-0 items-center gap-2.5">
            <button
              onClick={() => router.push('/')}
              aria-label="言镜 AI"
              className="shrink-0 rounded-md transition-opacity hover:opacity-80"
            >
                              <Image src="/logo.png" alt="言镜 AI" width={28} height={28} className="h-7 w-7 rounded-md" />
            </button>
            <div className="flex min-w-0 items-baseline gap-1.5">
              <span className="truncate text-[14px] font-semibold tracking-[-0.01em] text-neutral-900 dark:text-neutral-50">
                言镜 AI
              </span>
              <span className="shrink-0 rounded border border-neutral-200 px-1 py-px font-mono text-[9.5px] uppercase tracking-wider text-neutral-500 dark:border-neutral-700 dark:text-neutral-500">
                v0.9.2
              </span>
            </div>
          </div>
        ) : (
          <button
            onClick={() => router.push('/')}
            aria-label="言镜 AI"
            className="mx-auto rounded-md p-1 transition-opacity hover:opacity-80"
          >
                          <Image src="/logo.png" alt="言镜 AI" width={28} height={28} className="h-7 w-7 rounded-md" />
          </button>
        )}
      </div>

      {/* Primary nav */}
      <nav className="flex-1 overflow-y-auto px-2.5 py-3 space-y-0.5">
        {!isCollapsed && (
          <div className="px-2 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-neutral-400">
            {t('sidebar.workspace')}
          </div>
        )}

        {/* Home */}
        {(() => {
          const active = pathname === '/';
          return (
            <button onClick={() => router.push('/')} title={isCollapsed ? t('nav.home') : undefined}
              className={`relative flex w-full items-center gap-3 rounded-lg px-2.5 py-2 text-[13.5px] font-medium transition-colors ${active ? 'bg-blue-50/80 text-blue-700' : 'text-neutral-600 hover:bg-neutral-50 hover:text-neutral-900'} ${isCollapsed ? 'justify-center' : ''}`}>
              {active && <span className="absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-blue-600" />}
              <Home className={`h-[18px] w-[18px] ${active ? 'text-blue-600' : 'text-neutral-500'}`} />
              {!isCollapsed && <span className="truncate">{t('nav.home')}</span>}
            </button>
          );
        })()}

        {/* §141.7: 隐藏"会议脉络"入口 — 用户 8/20 反馈"看不懂,不知道在做什么",P0-A 知识图谱/Sidebar nav 都隐藏,代码保留便于恢复 */}
        {false && (() => {
          const active = pathname.startsWith('/knowledge');
          return (
            <button onClick={() => router.push('/knowledge')} title={isCollapsed ? t('nav.knowledge') : undefined}
              data-testid="sidebar-knowledge"
              className={`relative flex w-full items-center gap-3 rounded-lg px-2.5 py-2 text-[13.5px] font-medium transition-colors ${active ? 'bg-violet-50/80 text-violet-700' : 'text-neutral-600 hover:bg-neutral-50 hover:text-neutral-900'} ${isCollapsed ? 'justify-center' : ''}`}>
              {active && <span className="absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-violet-600" />}
              <BookOpen className={`h-[18px] w-[18px] ${active ? 'text-violet-600' : 'text-neutral-500'}`} />
              {!isCollapsed && (
                <>
                  <span className="truncate">{t('nav.knowledge')}</span>
                  {totalTopics > 0 && (
                    <span className="ml-auto rounded-full bg-violet-100 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-violet-700">
                      {totalTopics}
                    </span>
                  )}
                </>
              )}
            </button>
          );
        })()}

        {/* Recording CTA */}
        {(() => {
          const active = isRecording;
          return (
            <button onClick={handleRecordingToggle} disabled={isRecording}
              title={isCollapsed ? t('nav.start_recording') : undefined}
              className={`relative flex w-full items-center gap-3 rounded-lg px-2.5 py-2 text-[13.5px] font-medium transition-colors ${active ? 'bg-red-50 text-red-700' : 'text-neutral-600 hover:bg-neutral-50 hover:text-neutral-900'} ${isCollapsed ? 'justify-center' : ''}`}>
              {active ? <Square className="h-[18px] w-[18px] text-red-600" /> : <Mic className="h-[18px] w-[18px] text-neutral-500" />}
              {!isCollapsed && <span className="truncate">{active ? t('nav.recording_in_progress') : t('nav.start_recording')}</span>}
            </button>
          );
        })()}

        {/* Library: meetings folder */}
        {!isCollapsed && filteredSidebarItems.filter(i => i.type === 'folder').map(item => (
          <div key={item.id} className="mt-2">
            <div
              onClick={() => toggleFolder(item.id)}
              className="mb-1 flex cursor-pointer items-center gap-2 px-2.5"
            >
              {expandedFolders.has(item.id)
                ? <ChevronDown className="h-3.5 w-3.5 text-neutral-400" />
                : <ChevronRight className="h-3.5 w-3.5 text-neutral-400" />}
              <NotebookPen className="h-3.5 w-3.5 text-neutral-500" />
              <span className="text-[11px] font-semibold uppercase tracking-wider text-neutral-500">{item.title}</span>
              {searchQuery && item.id === 'meetings' && isSearching && (
                <span className="text-[10.5px] text-blue-500 animate-pulse">…</span>
              )}
            </div>
            {expandedFolders.has(item.id) && item.children && item.children.length > 0 && (
              <div className="space-y-0.5">
                {item.children.map((child: any) => renderItem(child, 1))}
              </div>
            )}
            {/* Empty search state for meetings folder */}
            {expandedFolders.has(item.id) && searchQuery.trim() && (!item.children || item.children.length === 0) && (
              <div className="mt-1 ml-2 px-2.5 py-2 text-[12px] text-neutral-400">
                <span>{t('home.no_match')}</span>
                <button
                  onClick={() => {
                    window.dispatchEvent(new CustomEvent('lixianhuiji:search-query', { detail: '' }));
                  }}
                  className="ml-2 text-blue-500 hover:underline"
                >
                  {t('common.clear')}
                </button>
              </div>
            )}
          </div>
        ))}

        {/* Configure */}
        <div className="pt-4" />
        {!isCollapsed && (
          <div className="px-2 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-neutral-400">{t('sidebar.configure')}</div>
        )}
        {(() => {
          const active = pathname.startsWith('/settings');
          return (
            <button onClick={() => router.push('/settings')} title={isCollapsed ? t('nav.settings') : undefined}
              className={`relative flex w-full items-center gap-3 rounded-lg px-2.5 py-2 text-[13.5px] font-medium transition-colors ${active ? 'bg-blue-50/80 text-blue-700' : 'text-neutral-600 hover:bg-neutral-50 hover:text-neutral-900'} ${isCollapsed ? 'justify-center' : ''}`}>
              {active && <span className="absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-blue-600" />}
              <Settings className={`h-[18px] w-[18px] ${active ? 'text-blue-600' : 'text-neutral-500'}`} />
              {!isCollapsed && <span className="truncate">{t('nav.settings')}</span>}
            </button>
          );
        })()}
        {/* About */}
        <div className="pt-2">
          <Info isCollapsed={isCollapsed} />
        </div>
      </nav>

      {/* Footer */}
      <div className="border-t border-neutral-200/70 px-3 py-2.5 text-[10.5px] text-neutral-400">
        <div className="flex items-center justify-between">
          {!isCollapsed ? (
            <>
              <span>v0.9.2 · MIT</span>
              <span className="inline-flex items-center gap-1 text-emerald-600">
                <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />offline
              </span>
            </>
          ) : (
            <span className="mx-auto">v0.9.2</span>
          )}
        </div>
        {!isCollapsed && (
          <a
            href="mailto:sam.wang01@icloud.com?subject=言镜 AI - 反馈&body=版本 v0.9.2 · macOS"
            onClick={(e) => { e.preventDefault(); openExternalUrl('mailto:sam.wang01@icloud.com?subject=言镜 AI - 反馈&body=版本 v0.9.2 · macOS'); }}
            className="mt-1.5 flex items-center gap-1 text-[10px] text-neutral-500 hover:text-blue-600 transition-colors truncate cursor-pointer"
            title="联系客服: sam.wang01@icloud.com"
          >
            <svg className="h-2.5 w-2.5 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2"><rect x="2" y="4" width="20" height="16" rx="2"/><path d="m22 7-10 5L2 7"/></svg>
            <span className="truncate">客服: sam.wang01@icloud.com</span>
          </a>
        )}
      </div>
    <FeedbackDialog
        open={feedbackOpen}
        onOpenChange={setFeedbackOpen}
      />
    </aside>

      {/* Confirmation Modal for Delete */}
      <ConfirmationModal
        isOpen={deleteModalState.isOpen}
        text="确认删除该会议? 此操作不可撤销。"
        onConfirm={handleDeleteConfirm}
        onCancel={() => setDeleteModalState({ isOpen: false, itemId: null })}
      />

      {/* 编辑会议标题 Modal */}
      <Dialog open={editModalState.isOpen} onOpenChange={(open) => {
        if (!open) handleEditCancel();
      }}>
        <DialogContent className="sm:max-w-[425px]">
          <VisuallyHidden>
            <DialogTitle>编辑会议标题</DialogTitle>
          </VisuallyHidden>
          <div className="py-4">
            <h3 className="text-lg font-semibold mb-4">编辑会议标题</h3>
            <div className="space-y-4">
              <div>
                <label htmlFor="meeting-title" className="block text-sm font-medium text-gray-700 mb-2">
                  Meeting Title
                </label>
                <input
                  id="meeting-title"
                  type="text"
                  value={editingTitle}
                  onChange={(e) => setEditingTitle(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      handleEditConfirm();
                    } else if (e.key === 'Escape') {
                      handleEditCancel();
                    }
                  }}
                  className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  placeholder="输入会议标题"
                  autoFocus
                />
              </div>
            </div>
          </div>
          <DialogFooter>
            <button
              onClick={handleEditCancel}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors"
            >
              Cancel
            </button>
            <button
              onClick={handleEditConfirm}
              className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md transition-colors"
            >
              Save
            </button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};

export default Sidebar;
