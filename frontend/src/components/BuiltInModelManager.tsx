'use client';

import { useState, useEffect } from 'react';
import { useTranslation } from '@/i18n';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { cn } from '@/lib/utils';
import { Download, RefreshCw, BadgeAlert, Trash2 } from 'lucide-react';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import { formatSummaryModelSizeLabelFromMb } from '@/lib/onboarding-summary-model';

interface ModelInfo {
  name: string;
  display_name: string;
  status: {
    type: 'not_downloaded' | 'downloading' | 'available' | 'corrupted' | 'error';
    progress?: number;
  };
  size_mb: number;
  context_size: number;
  description: string;
  gguf_file: string;
}

interface DownloadProgressInfo {
  downloadedMb: number;
  totalMb: number;
  speedMbps: number;
}

interface BuiltInModelManagerProps {
  selectedModel: string;
  onModelSelect: (model: string) => void;
  layout?: 'inline' | 'dialog';
}

export function BuiltInModelManager({
  selectedModel,
  onModelSelect,
  layout = 'inline',
}: BuiltInModelManagerProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [hasFetched, setHasFetched] = useState<boolean>(false);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, number>>({});
  const [downloadProgressInfo, setDownloadProgressInfo] = useState<Record<string, DownloadProgressInfo>>({});
  const { t } = useTranslation();
  const [downloadingModels, setDownloadingModels] = useState<Set<string>>(new Set());
  // §90: 默认隐藏未下载模型, 用户要"显示所有"才展开下载列表
  const [showAllModels, setShowAllModels] = useState<boolean>(false);

  // §202 (2026-08-31): RAM-aware recommendation banner.
  // When the system has <16GB RAM, recommend qwen3.5:2b over qwen2.5:3b for better
  // performance. Show "当前机器 8GB · 推荐 qwen3.5:2b (更快)" so user can choose
  // consciously instead of getting the default 3B and watching it crawl at 5 tok/s.
  const [deviceRamGb, setDeviceRamGb] = useState<number | null>(null);
  const [deviceTier, setDeviceTier] = useState<string | null>(null);
  const [isAppleSilicon, setIsAppleSilicon] = useState<boolean | null>(null);
  const [cpuBrand, setCpuBrand] = useState<string | null>(null);

  const recommendedModelForRam = (ramGb: number, appleSilicon: boolean): string => {
    // §205.1 (2026-09-02): 跟 Rust 端 recommend_summary_model 完全对齐 — 单一真实源
    //   ≥16GB                  → qwen2.5:3b (stable, 已实测)
    //   9-15GB Apple Silicon   → spark-x2.5:1.7b (中文 benchmark +20.8, 法律场景高级选项)
    //   8GB / <8GB / Intel     → qwen3.5:2b (保稳, 8GB 边界仍 qwen 不走 spark)
    if (ramGb >= 16) return 'qwen2.5:3b';
    if (ramGb > 8 && ramGb < 16 && appleSilicon) return 'spark-x2.5:1.7b';
    if (ramGb >= 10 && appleSilicon) return 'spark-x2.5:1.7b';
    return 'qwen3.5:2b';
  };

  const fetchDeviceProfile = async () => {
    try {
      const profile = await invoke<{
        total_memory_gb?: number;
        total_memory_mb?: number;
        cpu_brand?: string;
        is_apple_silicon?: boolean;
        tier?: string;
      }>('device_detect_profile');
      const ramGb = profile.total_memory_gb
        ?? (profile.total_memory_mb ? Math.round(profile.total_memory_mb / 1024) : null)
        ?? null;
      setDeviceRamGb(ramGb);
      setDeviceTier(profile.tier ?? null);
      setIsAppleSilicon(profile.is_apple_silicon ?? null);
      setCpuBrand(profile.cpu_brand ?? null);
    } catch (error) {
      console.warn('§202: failed to detect device profile', error);
    }
  };

  const fetchModels = async () => {
    try {
      setIsLoading(true);
      const data = (await invoke('builtin_ai_list_models')) as ModelInfo[];
      setModels(data);

      // Auto-select first available model if none selected
      if (data.length > 0 && !selectedModel) {
        const firstAvailable = data.find((m) => m.status.type === 'available');
        if (firstAvailable) {
          onModelSelect(firstAvailable.name);
        }
      }
    } catch (error) {
      console.error('Failed to fetch built-in AI models:', error);
      safeToast.error('加载模型失败');
    } finally {
      setIsLoading(false);
      setHasFetched(true);
    }
  };

  useEffect(() => {
    fetchModels();
    fetchDeviceProfile();
  }, []);

  // Listen for download progress events
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await listen('builtin-ai-download-progress', (event: any) => {
        const { model, progress, downloaded_mb, total_mb, speed_mbps, status } = event.payload;

        // Update percentage progress
        setDownloadProgress((prev) => ({
          ...prev,
          [model]: progress,
        }));

        // Update detailed progress info (MB, speed)
        setDownloadProgressInfo((prev) => ({
          ...prev,
          [model]: {
            downloadedMb: downloaded_mb ?? 0,
            totalMb: total_mb ?? 0,
            speedMbps: speed_mbps ?? 0,
          },
        }));

        // Handle downloading status - restore downloadingModels state on modal reopen
        if (status === 'downloading') {
          setDownloadingModels((prev) => {
            if (!prev.has(model)) {
              const newSet = new Set(prev);
              newSet.add(model);
              return newSet;
            }
            return prev;
          });
        }

        // Handle completed status
        if (status === 'completed') {
          setDownloadingModels((prev) => {
            const newSet = new Set(prev);
            newSet.delete(model);
            return newSet;
          });
          // Clean up progress state
          setDownloadProgress((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });
          setDownloadProgressInfo((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });
          // Refresh models list
          fetchModels();
          safeToast.success(`模型 ${model} downloaded successfully`);
        }

        // Handle cancelled status
        if (status === 'cancelled') {
          setDownloadingModels((prev) => {
            const newSet = new Set(prev);
            newSet.delete(model);
            return newSet;
          });
          // Clean up progress state
          setDownloadProgress((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });
          setDownloadProgressInfo((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });
          // Refresh models list
          fetchModels();
        }

        // Handle error status
        if (status === 'error') {
          setDownloadingModels((prev) => {
            const newSet = new Set(prev);
            newSet.delete(model);
            return newSet;
          });
          // Clean up progress state
          setDownloadProgress((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });
          setDownloadProgressInfo((prev) => {
            const { [model]: _, ...rest } = prev;
            return rest;
          });

          // Update model status to error locally instead of fetching from backend
          // Backend doesn't persist error status, so fetchModels() would return not_downloaded
          setModels((prevModels) =>
            prevModels.map((m) =>
              m.name === model
                ? {
                    ...m,
                    status: {
                      type: 'error',
                      progress: 0,
                    } as any,
                  }
                : m
            )
          );

          // Don't show error toast here - DownloadProgressToast already handles it
          // Don't call fetchModels() - it would overwrite error status with not_downloaded
        }
      });
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const downloadModel = async (modelName: string) => {
    try {
      // Optimistically add to downloadingModels for immediate UI feedback
      setDownloadingModels((prev) => new Set([...prev, modelName]));

      await invoke('builtin_ai_download_model', { modelName });
    } catch (error) {
      console.error('Failed to download model:', error);

      // Check if this is a cancellation error (starts with "CANCELLED:")
      const errorMsg = String(error);
      if (errorMsg.startsWith('CANCELLED:')) {
        // Cancel handler already removed from downloadingModels
        // Don't show error toast for cancellations - cancel function already shows info toast
        return;
      }

      // For real errors, show toast and remove from downloading
      safeToast.error(`Failed to download ${modelName}`);

      setDownloadingModels((prev) => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });

      // Refresh model list to get updated Error status from backend
      fetchModels();
    }
  };

  const cancelDownload = async (modelName: string) => {
    try {
      await invoke('builtin_ai_cancel_download', { modelName });
      safeToast.info(`下载 of ${modelName} cancelled`);
      setDownloadingModels((prev) => {
        const newSet = new Set(prev);
        newSet.delete(modelName);
        return newSet;
      });
    } catch (error) {
      console.error('Failed to cancel download:', error);
    }
  };

  const deleteModel = async (modelName: string) => {
    try {
      await invoke('builtin_ai_delete_model', { modelName });
      safeToast.success(`模型 ${modelName} deleted`);
      fetchModels();
    } catch (error) {
      console.error('Failed to delete model:', error);
      safeToast.error(`Failed to delete ${modelName}`);
    }
  };

  // Don't show loading spinner if we have downloads in progress - show the model list instead
  if (isLoading && downloadingModels.size === 0) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        <RefreshCw className="mx-auto h-8 w-8 animate-spin mb-2" />
        Loading models...
      </div>
    );
  }

  // Only show "no models" message after fetch has completed
  if (hasFetched && models.length === 0) {
    return (
      <Alert>
        <AlertDescription>
          No models found. Download a model to get started with Built-in AI.
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <div>
      <div className="flex items-center justify-between mb-4">
        <h4 className="text-sm font-bold">{t('models.title')}</h4>
      </div>

      {/* §202: RAM-aware recommendation banner. Helps 8GB users avoid the 3B default
         trap where they get a 5 tok/s crawl. Renders only after device profile loads. */}
      {deviceRamGb !== null && (
        <Alert className="mb-4 border-blue-200 bg-blue-50/50 dark:bg-blue-950/20">
          <AlertDescription className="text-xs">
            <span className="font-medium">{t('models.ram_detected', { ram: deviceRamGb, cpu: cpuBrand ?? '' })}</span>
            {(() => {
              const recommended = recommendedModelForRam(deviceRamGb, isAppleSilicon ?? false);
              const isCurrentRecommended = selectedModel === recommended;
              if (isCurrentRecommended) {
                return (
                  <span className="ml-2 text-green-700 dark:text-green-400">
                    ✓ {t('models.ram_match', { model: recommended })}
                  </span>
                );
              }
              const selectedMeta = models.find(m => m.name === selectedModel);
              const recommendedMeta = models.find(m => m.name === recommended);
              if (!recommendedMeta) return null; // 没装推荐模型, 不提示切换
              return (
                <button
                  type="button"
                  className="ml-2 text-blue-700 hover:underline dark:text-blue-400"
                  onClick={() => onModelSelect(recommended)}
                  data-testid="ram-recommendation-cta"
                >
                  → {t('models.ram_recommend', {
                    current: selectedMeta?.display_name ?? selectedModel,
                    recommended: recommendedMeta.display_name ?? recommended,
                  })}
                </button>
              );
            })()}
          </AlertDescription>
        </Alert>
      )}

      <div className="mb-3 flex items-center justify-between text-xs text-neutral-500">
        <span>
          {showAllModels
            ? t('models.showing_all')
            : t('models.showing_available')}
        </span>
        <button
          onClick={() => setShowAllModels((v) => !v)}
          className="text-blue-600 hover:underline"
          data-testid="toggle-show-all-models"
        >
          {showAllModels
            ? t('models.hide_undownloaded')
            : t('models.show_undownloaded')}
        </button>
      </div>
      <div
        className={cn(
          'grid gap-4',
          layout === 'dialog' && 'max-h-[50vh] overflow-y-auto pr-2 pb-2'
        )}
      >
        {models
          .filter((m) => {
            // §90: 默认只显示已下载 (available / downloading / corrupted / error)
            // 不显示 not_downloaded 直到用户点 "显示所有"
            if (showAllModels) return true;
            return m.status.type !== 'not_downloaded';
          })
          .map((model) => {
          const progress = downloadProgress[model.name];
          const progressInfo = downloadProgressInfo[model.name];
          const modelIsDownloading = downloadingModels.has(model.name);
          const isAvailable = model.status.type === 'available';
          const isNotDownloaded = model.status.type === 'not_downloaded';
          const isCorrupted = model.status.type === 'corrupted';
          const isError = model.status.type === 'error';

          return (
            <div
              key={model.name}
              className={cn(
                'p-4 rounded-lg border transition-colors',
                modelIsDownloading
                  ? 'bg-white border-gray-200'
                  : 'bg-card',
                selectedModel === model.name
                  ? 'ring-2 ring-gray-800 border-gray-800'
                  : 'border-gray-200 hover:border-gray-300',
                isAvailable && !modelIsDownloading && 'cursor-pointer'
              )}
              onClick={() => {
                if (isAvailable && !modelIsDownloading) {
                  onModelSelect(model.name);
                }
              }}
            >
            <div className="space-y-3">
              <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                <div className="min-w-0 flex-1">
                  <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                    <span className="min-w-0 break-words text-base font-bold leading-snug text-gray-900">{model.display_name || model.name}</span>
                    {isAvailable && (
                      <>
                        <span className="flex shrink-0 items-center gap-1 text-xs font-medium text-green-600">
                          <span className="h-2 w-2 rounded-full bg-green-600"></span>
                          {t('models.status.ready')}
                        </span>
                        {selectedModel === model.name && (
                          <span className="shrink-0 rounded bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700">
                            {t('models.status.selected')}
                          </span>
                        )}
                      </>
                    )}
                    {isCorrupted && (
                      <span className="flex shrink-0 items-center gap-1 rounded bg-red-100 px-2 py-0.5 text-xs font-medium text-red-700">
                        <BadgeAlert className="h-3 w-3" />
                        {t('models.status.corrupted')}
                      </span>
                    )}
                    {isError && (
                      <span className="shrink-0 rounded bg-red-100 px-2 py-0.5 text-xs font-medium text-red-700">
                        {t('models.status.error')}
                      </span>
                    )}
                  </div>
                </div>
                <div className="flex w-full shrink-0 flex-wrap items-center gap-2 sm:ml-4 sm:w-auto sm:justify-end">
                  {/* Not Downloaded - Show Download button */}
                  {isNotDownloaded && !modelIsDownloading && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="min-w-[100px]"
                      onClick={(e) => {
                        e.stopPropagation();
                        downloadModel(model.name);
                      }}
                    >
                      <Download className="mr-2 h-4 w-4" />
                      {t('models.action.download')}
                    </Button>
                  )}
                  {/* Downloading - Show Cancel button */}
                  {modelIsDownloading && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="min-w-[100px]"
                      onClick={(e) => {
                        e.stopPropagation();
                        cancelDownload(model.name);
                      }}
                    >
                      {t('models.action.cancel')}
                    </Button>
                  )}
                  {/* Error - Show Retry button */}
                  {isError && !modelIsDownloading && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="min-w-[100px]"
                      onClick={(e) => {
                        e.stopPropagation();
                        downloadModel(model.name);
                      }}
                    >
                      <RefreshCw className="mr-2 h-4 w-4" />
                      Retry
                    </Button>
                  )}
                  {/* Corrupted - Show both Retry and Delete buttons */}
                  {isCorrupted && !modelIsDownloading && (
                    <>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          downloadModel(model.name);
                        }}
                      >
                        <RefreshCw className="mr-2 h-4 w-4" />
                        {t('models.action.retry')}
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          deleteModel(model.name);
                        }}
                      >
                        <Trash2 className="mr-2 h-4 w-4" />
                        {t('models.action.delete')}
                      </Button>
                    </>
                  )}
                  {/* Available - Show small trash icon (only if not currently selected) */}
                  {isAvailable && !modelIsDownloading && selectedModel !== model.name && (
                    <button
                      className="p-2 rounded hover:bg-gray-100 transition-colors text-gray-500 hover:text-red-600"
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteModel(model.name);
                      }}
                      title={t('models.action.delete_model')}
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                  )}
                </div>
              </div>
              <div className="text-sm text-gray-600">
                {model.description && (
                  <p className="mb-1">{model.description}</p>
                )}
                {(isError || isCorrupted) && (
                  <p className="mb-1 text-xs text-red-600">
                    {isError && typeof model.status === 'object' && 'Error' in model.status
                      ? (model.status as any).Error
                      : isCorrupted
                      ? t('model_dialog.file_corrupted')
                      : t('errors.unknown')}
                  </p>
                )}
                <div className="text-xs text-gray-500">
                  <span>{formatSummaryModelSizeLabelFromMb(model.size_mb)}{t('models.size.unit_separator')}{model.context_size} {t('models.size.tokens')}</span>
                </div>
                </div>
              </div>

              {/* Download progress bar */}
              {modelIsDownloading && progress !== undefined && (
                <div className="mt-3 pt-3 border-t border-gray-200">
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-sm font-medium text-gray-900">{t('models.status.downloading')}</span>
                    <span className="text-sm font-semibold text-gray-900">
                      {Math.round(progress)}%
                    </span>
                  </div>
                  <div className="text-sm text-gray-600 mb-2">
                    {progressInfo?.totalMb > 0 ? (
                      <>
                        {progressInfo.downloadedMb.toFixed(1)} MiB / {progressInfo.totalMb.toFixed(1)} MiB
                        {progressInfo.speedMbps > 0 && (
                          <span className="ml-2 text-gray-500">
                            ({progressInfo.speedMbps.toFixed(1)} MiB/s)
                          </span>
                        )}
                      </>
                    ) : (
                      <span>{formatSummaryModelSizeLabelFromMb(model.size_mb)}</span>
                    )}
                  </div>
                  <div className="w-full h-2.5 bg-gray-200 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-gray-800 to-gray-900 rounded-full transition-all duration-300"
                      style={{ width: `${progress}%` }}
                    />
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
