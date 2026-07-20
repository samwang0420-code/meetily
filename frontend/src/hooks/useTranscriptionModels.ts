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

    // 离线会记 W2.5: Whisper 从 enhance 选项删除 (recording 也改为 sherpa, 不再需要 Whisper)
    // Whisper 体验差 (幻觉多 / 中文漏字), 完全被 SenseVoice-zh 替代
    // 模型文件 ggml-large-v3-turbo-q5_0.bin 547MB 也即将删除

    // 离线会记 W2.3: 默认推荐 SenseVoice-zh (23 段按句切 + 字级 timestamp + 中文 SOTA)
    // Paraformer 排在第二位 (W2.2 验证可用, 但无 timestamps, 退回 VAD 段循环)
    // 用户反馈: "Paraformer 不能看, SenseVoice 默认体验最好"
    allModels.push({
      provider: 'sherpa_funasr_nano' as const,
      name: 'sense-voice-zh-int8',
      displayName: '✨ SenseVoice-zh (推荐 · 23 段)',
      size_mb: 228,
    });
    allModels.push({
      provider: 'sherpa_funasr_nano' as const,
      name: 'funasr-nano-zh',
      displayName: '🧪 FunASR-Nano (实验性离线精转 · 较慢)',
      size_mb: 948,
    });
    allModels.push({
      provider: 'sherpa_paraformer' as const,
      name: 'paraformer-zh-int8',
      displayName: '🐉 Paraformer-zh (备选 · 10 段)',
      size_mb: 217,
    });

    // Fetch Parakeet models
    try {
      const parakeetModels = await invoke<RawModelInfo[]>('parakeet_get_available_models');
      const availableParakeet = parakeetModels
        .filter((m) => m.status === 'Available')
        .map((m) => ({
          provider: 'parakeet' as const,
          name: m.name,
          displayName: `⚡ Parakeet: ${m.name}`,
          size_mb: m.size_mb,
        }));
      allModels.push(...availableParakeet);
    } catch (err) {
      console.error('Failed to fetch Parakeet models:', err);
    }

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
