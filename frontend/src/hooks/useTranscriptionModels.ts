import { useState, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface RawModelInfo {
  name: string;
  size_mb: number;
  status: 'Available' | 'Missing' | { Downloading: { progress: number } } | { Error: string };
}

export interface ModelOption {
  provider: 'parakeet' | 'sherpa_paraformer' | 'sherpa_funasr_nano';
  name: string;
  displayName: string;
  size_mb: number;
}

interface TranscriptModelConfig {
  provider?: string;
  model?: string;
}

/**
 * Custom hook for fetching and managing transcription models (Whisper and Parakeet).
 *
 * This hook centralizes the model fetching logic that was previously duplicated
 * in ImportAudioDialog and RetranscribeDialog components.
 *
 * @param transcriptModelConfig - User's saved model configuration from context
 * @returns Object containing available models, selected model key, loading state, and fetch function
 */
export function useTranscriptionModels(transcriptModelConfig: TranscriptModelConfig | undefined) {
  const [availableModels, setAvailableModels] = useState<ModelOption[]>([]);
  const [selectedModelKey, setSelectedModelKey] = useState<string>('');
  const [loadingModels, setLoadingModels] = useState(false);
  // Track whether the user has manually changed the model selection
  const userSelectedRef = useRef(false);

  // Wrap setSelectedModelKey to track user-initiated changes
  const setSelectedModelKeyWithTracking = useCallback((key: string) => {
    userSelectedRef.current = true;
    setSelectedModelKey(key);
  }, []);

  const fetchModels = useCallback(async () => {
    setLoadingModels(true);
    const allModels: ModelOption[] = [];

    // §90 v0.8+ 转录模型列表 (用户决策 2026-08-07):
    // 1) FunASR-Nano 947MB (高精度 / Pro 专属 per §29)
    // 2) SenseVoice-zh 228MB (默认推荐, 用户实测已装 per §38 续)
    // 3) Paraformer-zh 216MB (备选, 中文 SOTA 但无 timestamp)
    // 删除: Parakeet (实测不如 SenseVoice, §38 禁用清单)
    // 删除: Whisper (W2.5 已删)

    allModels.push({
      provider: 'sherpa_funasr_nano' as const,
      name: 'funasr-nano-zh',
      displayName: '🧪 FunASR-Nano 高精度 · 947MB (Pro)',
      size_mb: 947,
    });
    allModels.push({
      provider: 'sherpa_funasr_nano' as const,
      name: 'sense-voice-zh-int8',
      displayName: '✨ SenseVoice-zh 推荐 · 228MB',
      size_mb: 228,
    });
    allModels.push({
      provider: 'sherpa_paraformer' as const,
      name: 'paraformer-zh-int8',
      displayName: '🐉 Paraformer-zh 备选 · 216MB',
      size_mb: 216,
    });

    setAvailableModels(allModels);

    // Set default model based on user's saved configuration
    const configuredProvider = transcriptModelConfig?.provider || '';
    const configuredModel = transcriptModelConfig?.model || '';

    // Try to match the configured model
    // W2.5: 已移除 whisper, 不再处理 localWhisper 配置
    const configuredMatch = allModels.find(
      (m) =>
        (configuredProvider === 'parakeet' && m.provider === 'parakeet' && m.name === configuredModel) ||
        ((configuredProvider === 'sherpa_paraformer' || configuredProvider === 'sherpa_funasr_nano') &&
         (m.provider === configuredProvider))
    );

    // Only set default model if user hasn't manually selected one
    if (!userSelectedRef.current) {
      if (configuredMatch) {
        // Use the configured model if available
        setSelectedModelKey(`${configuredMatch.provider}:${configuredMatch.name}`);
      } else {
        // Fall back: 优先 SenseVoice-zh (默认推荐), 然后任意 sherpa, 最后第一个
        const senseVoice = allModels.find(
          (m) => m.provider === 'sherpa_funasr_nano' && m.name === 'sense-voice-zh-int8'
        );
        const anySherpa = allModels.find(
          (m) => m.provider === 'sherpa_funasr_nano' || m.provider === 'sherpa_paraformer'
        );
        const fallback = senseVoice || anySherpa || allModels[0];
        if (fallback) {
          setSelectedModelKey(`${fallback.provider}:${fallback.name}`);
        }
      }
    }

    setLoadingModels(false);
  }, [transcriptModelConfig]);

  // Reset user selection tracking (call when dialog opens fresh)
  const resetSelection = useCallback(() => {
    userSelectedRef.current = false;
  }, []);

  return {
    availableModels,
    selectedModelKey,
    setSelectedModelKey: setSelectedModelKeyWithTracking,
    loadingModels,
    fetchModels,
    resetSelection,
  };
}
