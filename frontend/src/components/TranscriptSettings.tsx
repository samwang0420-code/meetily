import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { Input } from './ui/input';
import { Button } from './ui/button';
import { Label } from './ui/label';
import { Eye, EyeOff, Lock, Unlock } from 'lucide-react';
import { useTranslation } from '@/i18n';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';

// W2.5: WhisperModelManager 已不需要 — Whisper 完全删除
// import { ModelManager } from './WhisperModelManager';
import { ParakeetModelManager } from './ParakeetModelManager';


export interface TranscriptModelProps {
    provider: 'localWhisper' | 'parakeet' | 'deepgram' | 'elevenLabs' | 'groq' | 'openai' | 'sherpa_paraformer' | 'sherpa_funasr_nano';  // 离线会记 W2.5: 加 sherpa
    model: string;
    apiKey?: string | null;
}

export interface TranscriptSettingsProps {
    transcriptModelConfig: TranscriptModelProps;
    setTranscriptModelConfig: (config: TranscriptModelProps) => void;
    onModelSelect?: () => void;
}

export function TranscriptSettings({
    transcriptModelConfig,
    setTranscriptModelConfig,
    onModelSelect,
  }: TranscriptSettingsProps) {
  const { t } = useTranslation();
    const [apiKey, setApiKey] = useState<string | null>(transcriptModelConfig.apiKey || null);
    const [showApiKey, setShowApiKey] = useState<boolean>(false);
    const [isApiKeyLocked, setIsApiKeyLocked] = useState<boolean>(true);
    const [isLockButtonVibrating, setIsLockButtonVibrating] = useState<boolean>(false);
    const [uiProvider, setUiProvider] = useState<TranscriptModelProps['provider']>(transcriptModelConfig.provider);

    // Sync uiProvider when backend config changes (e.g., after model selection or initial load)
    useEffect(() => {
        setUiProvider(transcriptModelConfig.provider);
    }, [transcriptModelConfig.provider]);

    useEffect(() => {
        if (transcriptModelConfig.provider === 'localWhisper' || transcriptModelConfig.provider === 'parakeet') {
            setApiKey(null);
        }
    }, [transcriptModelConfig.provider]);

    const fetchApiKey = async (provider: string) => {
        try {

            const data = await invoke('api_get_transcript_api_key', { provider }) as string;

            setApiKey(data || '');
        } catch (err) {
            console.error('Error fetching API key:', err);
            setApiKey(null);
        }
    };
    const modelOptions: Record<string, string[]> = {
        localWhisper: [], // Model selection handled by ModelManager component
        // §94.1 fix: parakeet v0.8+ 禁用 (实测不如 SenseVoice, §38), 删选项
        deepgram: ['nova-2-phonecall'],
        elevenLabs: ['eleven_multilingual_v2'],
        groq: ['llama-3.3-70b-versatile'],
        openai: ['gpt-4o'],
        // W2.5: sherpa 模型选择由 WhisperModelManager / sherpa daemon 处理, 不需要这里列
        sherpa_paraformer: [],
        // v0.6.10+: 加 funasr-nano-zh 作为可选 (实验性), 用户切时弹窗告知评测数据不达标
        sherpa_funasr_nano: ['sense-voice-zh-int8', 'funasr-nano-zh'],
    };
    const requiresApiKey = transcriptModelConfig.provider === 'deepgram' || transcriptModelConfig.provider === 'elevenLabs' || transcriptModelConfig.provider === 'openai' || transcriptModelConfig.provider === 'groq';

    const handleInputClick = () => {
        if (isApiKeyLocked) {
            setIsLockButtonVibrating(true);
            setTimeout(() => setIsLockButtonVibrating(false), 500);
        }
    };

    const handleWhisperModelSelect = (modelName: string) => {
        // Always update config when model is selected, regardless of current provider
        // This ensures the model is set when user switches back
        setTranscriptModelConfig({
            ...transcriptModelConfig,
            provider: 'localWhisper', // Ensure provider is set correctly
            model: modelName
        });
        // Close modal after selection
        if (onModelSelect) {
            onModelSelect();
        }
    };

    const handleParakeetModelSelect = (modelName: string) => {
        // Always update config when model is selected, regardless of current provider
        // This ensures the model is set when user switches back
        setTranscriptModelConfig({
            ...transcriptModelConfig,
            provider: 'parakeet', // Ensure provider is set correctly
            model: modelName
        });
        // Close modal after selection
        if (onModelSelect) {
            onModelSelect();
        }
    };

    return (
        <div>
            <div>
                {/* <div className="flex justify-between items-center mb-4">
                    <h3 className="text-lg font-semibold text-gray-900">转录设置</h3>
                </div> */}
                <div className="space-y-4 pb-6">
                    <div>
                        <Label className="block text-sm font-medium text-gray-700 mb-1">
                            Transcript Model
                        </Label>
                        <div className="flex space-x-2 mx-1">
                            <Select
                                value={uiProvider}
                                onValueChange={(value) => {
                                    const provider = value as TranscriptModelProps['provider'];
                                    setUiProvider(provider);
                                    if (provider !== 'localWhisper' && provider !== 'parakeet') {
                                        fetchApiKey(provider);
                                    }
                                }}
                            >
                                <SelectTrigger className='focus:ring-1 focus:ring-blue-500 focus:border-blue-500'>
                                    <SelectValue placeholder="选择 provider" />
                                </SelectTrigger>
                                <SelectContent>
                                    {/* §94.1 fix: §90 决策 - 2 个 provider, 第 2 个 Select 选具体 model name. §29 FunASR-Nano Pro gate 见 §94 P1. */}
                                    <SelectItem value="sherpa_funasr_nano">✨ 本地 ASR (SenseVoice 228MB + FunASR-Nano 947MB Pro)</SelectItem>
                                    <SelectItem value="sherpa_paraformer">🐉 Paraformer-zh 备选 · 216MB</SelectItem>
                                </SelectContent>
                            </Select>

                            {/* model list 为空时 (sherpa/无 cloud key) 不渲染空 select, 改用提示卡片 */}
                            {uiProvider !== 'localWhisper' && modelOptions[uiProvider]?.length > 0 && (
                                <Select
                                    value={transcriptModelConfig.model}
                                    onValueChange={(value) => {
                                        // v0.6.10+: 切到 FunASR-Nano (实验性) 时弹窗告知评测数据
                                        if (value === 'funasr-nano-zh') {
                                            // 来自 /benchmarks/asr/reports/model-decision.json (5 段法律 + 5 段医疗标准文本)
                                            safeToast.warning('切换到 FunASR-Nano (实验性)?', {
                                                description: '我们的内部评测显示: Nano CER 相对 +2.36% (不达 10% 准入), 术语召回 89.46% (低于 90% 门槛), 解码慢 6.04 倍 (远超 3 倍门槛). 切到这个模型不会让识别更好, 反而更慢, 仅供你技术验证.',
                                                duration: 10000,
                                                action: {
                                                    label: '我已知晓, 切到 Nano',
                                                    onClick: () => {
                                                        setTranscriptModelConfig({ ...transcriptModelConfig, provider: uiProvider, model: 'funasr-nano-zh' });
                                                    }
                                                },
                                                cancel: {
                                                    label: '取消',
                                                    onClick: () => {}
                                                }
                                            });
                                            return;
                                        }
                                        const model = value as TranscriptModelProps['model'];
                                        setTranscriptModelConfig({ ...transcriptModelConfig, provider: uiProvider, model });
                                    }}
                                >
                                    <SelectTrigger className='focus:ring-1 focus:ring-blue-500 focus:border-blue-500'>
                                        <SelectValue placeholder="选择 model" />
                                    </SelectTrigger>
                                    <SelectContent>
                                        {/* §94.1 fix: 用 §90 决策文案, 不直接显示 model name */}
                                        {modelOptions[uiProvider]?.map((model) => {
                                            const prettyName =
                                                model === 'funasr-nano-zh' ? '🧪 FunASR-Nano 高精度 · 947MB (Pro)' :
                                                model === 'sense-voice-zh-int8' ? '✨ SenseVoice-zh 推荐 · 228MB' :
                                                model === 'paraformer-zh-int8' ? '🐉 Paraformer-zh 备选 · 216MB' :
                                                model;
                                            return <SelectItem key={model} value={model}>{prettyName}</SelectItem>;
                                        })}
                                    </SelectContent>
                                </Select>
                            )}

                        </div>
                    </div>

                    {/* W2.5: Whisper 已删除, WhisperModelManager 不再渲染 */}
                    {uiProvider === 'localWhisper' && (
                        <div className="mt-6 p-4 bg-gray-50 rounded-lg">
                            <p className="text-sm text-gray-700">
                                ⚠️ Whisper 已在 v0.5 中移除, 完全被 SenseVoice-zh INT8 (228MB) + FunASR-Nano (947MB, Pro) 替代, 中文 SOTA。
                                请切换到 ✨ SenseVoice-zh INT8 (上方选项)。
                            </p>
                        </div>
                    )}

                    {(uiProvider === 'sherpa_funasr_nano' || uiProvider === 'sherpa_paraformer') && (
                        <div className="mt-6 p-4 bg-blue-50 rounded-lg">
                            <p className="text-sm text-blue-900">
                                ✅ sherpa-onnx 模型自动从 <code className="bg-white px-1 rounded">~/Library/Application Support/cn.lixianhuiji.app/models/sherpa/</code> 加载,
                                无需额外下载。当前已安装: SenseVoice-zh INT8 (228MB) + Paraformer-zh INT8 (216MB) + FunASR-Nano (947MB, Pro 专属)。
                            </p>
                            <p className="text-xs text-blue-700 mt-2">
                                v0.8+ 默认推荐 ✨ SenseVoice-zh (按句切 + 字级 timestamp), 中文 SOTA 体验最好。
                            </p>
                        </div>
                    )}

                    {/* v0.6.10+: FunASR-Nano 评测数据透明化 - 让用户知道为什么 Nano 不是默认 */}
                    {uiProvider === 'sherpa_funasr_nano' && (
                        <details className="mt-4 text-xs">
                            <summary className="cursor-pointer text-neutral-600 hover:text-neutral-900 font-medium">
                                📊 FunASR-Nano 评测数据 (5 段法律 + 5 段医疗标准文本)
                            </summary>
                            <div className="mt-3 p-3 bg-neutral-50 rounded-lg border border-neutral-200 space-y-2 text-neutral-700">
                                <div className="flex items-center justify-between">
                                    <span>CER (字错率):</span>
                                    <span className="font-mono">
                                        <span className="text-blue-700">SenseVoice 5.67%</span>
                                        {' vs '}
                                        <span className="text-neutral-500">Nano 5.54% (+2.36%)</span>
                                    </span>
                                </div>
                                <div className="flex items-center justify-between">
                                    <span>行业术语召回:</span>
                                    <span className="font-mono">
                                        <span className="text-blue-700">SenseVoice 81.4%</span>
                                        {' vs '}
                                        <span className="text-amber-700">Nano 89.5% (↑但低于 90% 门槛)</span>
                                    </span>
                                </div>
                                <div className="flex items-center justify-between">
                                    <span>解码耗时 (avg):</span>
                                    <span className="font-mono">
                                        <span className="text-blue-700">SenseVoice 375ms</span>
                                        {' vs '}
                                        <span className="text-red-700">Nano 2264ms (6.04x ↑)</span>
                                    </span>
                                </div>
                                <p className="pt-2 border-t border-neutral-200 text-neutral-600">
                                    准入门槛: CER ≥10% 改善, 术语召回 ≥90%, 解码 ≤3x 慢。
                                    三个指标 Nano <strong>均未达标</strong>, 故保持 SenseVoice 默认。
                                    评测脚本: <code>node benchmarks/asr/compare.mjs</code>。
                                </p>
                            </div>
                        </details>
                    )}

                    {uiProvider === 'parakeet' && (
                        <div className="mt-6">
                            <ParakeetModelManager
                                selectedModel={transcriptModelConfig.provider === 'parakeet' ? transcriptModelConfig.model : undefined}
                                onModelSelect={handleParakeetModelSelect}
                                autoSave={true}
                            />
                        </div>
                    )}


                    {requiresApiKey && (
                        <div>
                            <Label className="block text-sm font-medium text-gray-700 mb-1">
                                API Key
                            </Label>
                            <div className="relative mx-1">
                                <Input
                                    type={showApiKey ? "text" : "password"}
                                    className={`pr-24 focus:ring-1 focus:ring-blue-500 focus:border-blue-500 ${isApiKeyLocked ? 'bg-gray-100 cursor-not-allowed' : ''
                                        }`}
                                    value={apiKey || ''}
                                    onChange={(e) => setApiKey(e.target.value)}
                                    disabled={isApiKeyLocked}
                                    onClick={handleInputClick}
                                    placeholder={t('settings.api_key_placeholder')}
                                />
                                {isApiKeyLocked && (
                                    <div
                                        onClick={handleInputClick}
                                        className="absolute inset-0 flex items-center justify-center bg-gray-100 bg-opacity-50 rounded-md cursor-not-allowed"
                                    />
                                )}
                                <div className="absolute inset-y-0 right-0 pr-1 flex items-center">
                                    <Button
                                        type="button"
                                        variant="ghost"
                                        size="icon"
                                        onClick={() => setIsApiKeyLocked(!isApiKeyLocked)}
                                        className={`transition-colors duration-200 ${isLockButtonVibrating ? 'animate-vibrate text-red-500' : ''
                                            }`}
                                        title={isApiKeyLocked ? t('settings.unlock_to_edit') : t('settings.lock_to_edit')}
                                    >
                                        {isApiKeyLocked ? <Lock className="h-4 w-4" /> : <Unlock className="h-4 w-4" />}
                                    </Button>
                                    <Button
                                        type="button"
                                        variant="ghost"
                                        size="icon"
                                        onClick={() => setShowApiKey(!showApiKey)}
                                    >
                                        {showApiKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                                    </Button>
                                </div>
                            </div>
                        </div>
                    )}
                </div>
            </div>
        </div >
    )
}








