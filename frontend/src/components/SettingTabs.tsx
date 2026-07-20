import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ModelConfig, ModelSettingsModal } from "./ModelSettingsModal"
import { TranscriptModelProps, TranscriptSettings } from "./TranscriptSettings"
import { RecordingSettings, Recording偏好设置 } from "./RecordingSettings"
import { About } from "./About";

interface SettingTabsProps {
    modelConfig: ModelConfig;
    setModelConfig: (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => void;
    onSave: (config: ModelConfig) => void;
    transcriptModelConfig: TranscriptModelProps;
    setTranscriptModelConfig: (config: TranscriptModelProps) => void;
    onSaveTranscript: (config: TranscriptModelProps) => void;
    setSaveSuccess: (success: boolean | null) => void;
    defaultTab?: string;
}

export function SettingTabs({ 
    modelConfig, 
    setModelConfig, 
    onSave, 
    setSaveSuccess,
    defaultTab = "transcriptSettings",
    transcriptModelConfig,
    setTranscriptModelConfig,
    onSaveTranscript,
}: SettingTabsProps) {

    const handleTabChange = () => {
        setSaveSuccess(null); // Reset save success when tab changes
    };

    return (
        <Tabs defaultValue={defaultTab} className="w-full max-h-[calc(100vh-10rem)] overflow-y-auto" onValueChange={handleTabChange}>
  <TabsList>
    <TabsTrigger value="transcriptSettings">转录文本</TabsTrigger>
    <TabsTrigger value="modelSettings">AI 总结</TabsTrigger>
    <TabsTrigger value="recordingSettings">偏好设置</TabsTrigger>
    <TabsTrigger value="hotwords">热词</TabsTrigger>
    <TabsTrigger value="about">关于</TabsTrigger>
  </TabsList>

  <TabsContent value="hotwords">
    <div className="p-4 bg-white border border-gray-200 rounded-lg">
      <p className="text-sm text-gray-700 mb-3">启用热词词库, 让 ASR 更准确识别专业术语。</p>
      <a href="/settings/hotwords" className="text-blue-600 hover:underline text-sm">打开热词设置 →</a>
    </div>
  </TabsContent>

  <TabsContent value="modelSettings">
    <ModelSettingsModal

modelConfig={modelConfig}
setModelConfig={setModelConfig}
onSave={onSave}
/>
  </TabsContent>
<TabsContent value="transcriptSettings">
    <TranscriptSettings
    transcriptModelConfig={transcriptModelConfig}
    setTranscriptModelConfig={setTranscriptModelConfig}
    // onSave={onSaveTranscript}
  />
  </TabsContent>
  <TabsContent value="recordingSettings">
    <RecordingSettings />
  </TabsContent>
  <TabsContent value="about">
    <About />
  </TabsContent>
</Tabs>
    )
}


