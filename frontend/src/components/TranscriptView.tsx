'use client';

import { Transcript } from '@/types';
import { useEffect, useRef, useState } from 'react';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { ConfidenceIndicator } from './ConfidenceIndicator';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/tooltip';
import { RecordingStatusBar } from './RecordingStatusBar';
import { motion, AnimatePresence } from 'framer-motion';

// P1-G: speaker label helper. 把 "speaker_00" / "speaker_01" 解析成数字 index,
// 然后映射成人类可读标签 + 8 色调色板 (稳定, 按 index mod 8 取色).
const SPEAKER_PALETTE = [
  { bg: 'bg-blue-100',   text: 'text-blue-700',   ring: 'ring-blue-200',   bar: 'bg-blue-500'   },
  { bg: 'bg-emerald-100',text: 'text-emerald-700',ring: 'ring-emerald-200',bar: 'bg-emerald-500'},
  { bg: 'bg-amber-100',  text: 'text-amber-700',  ring: 'ring-amber-200',  bar: 'bg-amber-500'  },
  { bg: 'bg-rose-100',   text: 'text-rose-700',   ring: 'ring-rose-200',   bar: 'bg-rose-500'   },
  { bg: 'bg-violet-100', text: 'text-violet-700', ring: 'ring-violet-200', bar: 'bg-violet-500' },
  { bg: 'bg-cyan-100',   text: 'text-cyan-700',   ring: 'ring-cyan-200',   bar: 'bg-cyan-500'   },
  { bg: 'bg-orange-100', text: 'text-orange-700', ring: 'ring-orange-200', bar: 'bg-orange-500' },
  { bg: 'bg-fuchsia-100',text: 'text-fuchsia-700',ring: 'ring-fuchsia-200',bar: 'bg-fuchsia-500'},
];

// "speaker_00" -> 0, "speaker_03" -> 3, 其它 -> null
function parseSpeakerIndex(label?: string | null): number | null {
  if (!label) return null;
  const m = /^speaker_(\d+)$/i.exec(label.trim());
  if (!m) return null;
  const n = parseInt(m[1], 10);
  return Number.isFinite(n) && n >= 0 ? n : null;
}

// 把 speaker index 转成 UI 显示的短标签 (不用 i18n, 跟当前 TranscriptView 风格一致).
// P1-G: zh "说话人 1" / en "Speaker 1". 用浏览器 language 嗅探, 避免给 TranscriptView 引入 i18n 上下文.
function speakerLabel(idx: number): string {
  const isZh = typeof navigator !== 'undefined' && /^zh/i.test(navigator.language || '');
  return isZh ? `说话人 ${idx + 1}` : `Speaker ${idx + 1}`;
}

interface TranscriptViewProps {
  transcripts: Transcript[];
  isRecording?: boolean;
  isPaused?: boolean; // Is recording paused (affects UI indicators)
  isProcessing?: boolean; // Is processing/finalizing transcription (hides "正在聆听..." indicator)
  isStopping?: boolean; // Is recording being stopped (provides immediate UI feedback)
  enableStreaming?: boolean; // Enable streaming effect for live transcription UX
}

interface SpeechDetectedEvent {
  message: string;
}

// Helper function to format seconds as recording-relative time [MM:SS]
function formatRecordingTime(seconds: number | undefined): string {
  if (seconds === undefined) return '[--:--]';

  const totalSeconds = Math.floor(seconds);
  const minutes = Math.floor(totalSeconds / 60);
  const secs = totalSeconds % 60;

  return `[${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}]`;
}

// Helper function to remove consecutive word repetitions (especially short words ≤2 letters)
function cleanRepetitions(text: string): string {
  if (!text || text.trim().length === 0) return text;

  const words = text.split(/\s+/);
  const cleanedWords: string[] = [];

  let i = 0;
  while (i < words.length) {
    const currentWord = words[i];
    const currentWordLower = currentWord.toLowerCase();

    // Count consecutive repetitions of the same word
    let repeatCount = 1;
    while (
      i + repeatCount < words.length &&
      words[i + repeatCount].toLowerCase() === currentWordLower
    ) {
      repeatCount++;
    }

    // For short words (≤2 letters), be aggressive: if repeated 2+ times, keep only 1
    // For longer words, keep 1 if repeated 3+ times (less aggressive)
    if (currentWord.length <= 2) {
      // Short words: "I I I I" → "I", "Tu Tu Tu" → "Tu"
      if (repeatCount >= 2) {
        cleanedWords.push(currentWord);
        i += repeatCount;
      } else {
        cleanedWords.push(currentWord);
        i += 1;
      }
    } else {
      // Longer words: keep original unless heavily repeated
      if (repeatCount >= 3) {
        cleanedWords.push(currentWord);
        i += repeatCount;
      } else {
        cleanedWords.push(currentWord);
        i += 1;
      }
    }
  }

  return cleanedWords.join(' ');
}

// Helper function to remove filler words and stop words from transcripts
function cleanStopWords(text: string): string {
  // FIRST: Clean repetitions (especially short words)
  let cleanedText = cleanRepetitions(text);

  // THEN: Remove filler words
  const stopWords = [
    'uh', 'um', 'er', 'ah', 'hmm', 'hm', 'eh', 'oh',
    // 'like', 'you know', 'i mean', 'sort of', 'kind of',
    // 'basically', 'actually', 'literally', 'right',
    // 'thank you', 'thanks'
  ];

  // Remove each stop word (case-insensitive, with word boundaries)
  stopWords.forEach(word => {
    // Match the stop word at word boundaries, with optional punctuation
    const pattern = new RegExp(`\\b${word}\\b[,\\s]*`, 'gi');
    cleanedText = cleanedText.replace(pattern, ' ');
  });

  // Clean up extra whitespace and trim
  cleanedText = cleanedText.replace(/\s+/g, ' ').trim();

  return cleanedText;
}

export const TranscriptView: React.FC<TranscriptViewProps> = ({ transcripts, isRecording = false, isPaused = false, isProcessing = false, isStopping = false, enableStreaming = false }) => {
  const [speechDetected, setSpeechDetected] = useState(false);

  // Debug: Log the props to understand what's happening
  console.log('TranscriptView render:', {
    isRecording,
    isPaused,
    isProcessing,
    isStopping,
    transcriptCount: transcripts.length,
    shouldShowListening: !isStopping && isRecording && !isPaused && !isProcessing && transcripts.length > 0
  });

  // v0.6.11+ bug fix: 读 livePartialText (灰色 preview 流式显示)
  const { livePartialText, isPartialEndpoint, lastDecodeMs, lastBufferAgeMs } = useTranscripts();

    // Streaming effect state
  const [streamingTranscript, setStreamingTranscript] = useState<{
    id: string;
    visibleText: string;
    fullText: string;
  } | null>(null);
  const streamingIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const lastStreamedIdRef = useRef<string | null>(null); // Track which transcript we've streamed

  // Load preference for showing confidence indicator
  const [showConfidence, setShowConfidence] = useState<boolean>(() => {
    if (typeof window !== 'undefined') {
      const saved = localStorage.getItem('showConfidenceIndicator');
      return saved !== null ? saved === 'true' : true; // Default to true
    }
    return true;
  });

  // Listen for preference changes from settings
  useEffect(() => {
    const handleConfidenceChange = (e: Event) => {
      const customEvent = e as CustomEvent<boolean>;
      setShowConfidence(customEvent.detail);
    };

    window.addEventListener('confidenceIndicatorChanged', handleConfidenceChange);
    return () => window.removeEventListener('confidenceIndicatorChanged', handleConfidenceChange);
  }, []);

  // Listen for speech-detected event
  useEffect(() => {
    let unsubscribe: (() => void) | undefined;

    const setupListener = async () => {
      const { listen } = await import('@tauri-apps/api/event');
      unsubscribe = await listen<SpeechDetectedEvent>('speech-detected', () => {
        setSpeechDetected(true);
      });
    };

    if (isRecording) {
      setupListener();
    } else {
      // Reset when not recording
      setSpeechDetected(false);
    }

    return () => {
      if (unsubscribe) {
        unsubscribe();
      }
    };
  }, [isRecording]);

  // Streaming effect: animate new transcripts character-by-character
  useEffect(() => {
    if (!enableStreaming || !isRecording) {
      // Clean up if streaming is disabled
      if (streamingIntervalRef.current) {
        clearInterval(streamingIntervalRef.current);
        streamingIntervalRef.current = null;
      }
      setStreamingTranscript(null);
      lastStreamedIdRef.current = null;
      return;
    }

    // Find the latest non-partial transcript
    const latestTranscript = transcripts
      .slice(-1)[0];

    if (!latestTranscript) return;

    // Check if this is a new transcript we haven't streamed yet (using ref to avoid dependency issues)
    if (lastStreamedIdRef.current !== latestTranscript.id) {
      // Clear any existing streaming interval
      if (streamingIntervalRef.current) {
        clearInterval(streamingIntervalRef.current);
        streamingIntervalRef.current = null;
      }

      // Mark this transcript as being streamed
      lastStreamedIdRef.current = latestTranscript.id;

      const fullText = latestTranscript.text;

      // Fast typewriter effect - complete in 0.8 seconds for snappy feel
      const TOTAL_DURATION_MS = 800; // 0.8 seconds total - fast and snappy!
      const INTERVAL_MS = 15; // Update every 15ms for smooth animation
      const totalTicks = TOTAL_DURATION_MS / INTERVAL_MS; // ~53 ticks
      const charsPerTick = Math.max(2, Math.ceil(fullText.length / totalTicks)); // At least 2 chars per tick for speed
      const INITIAL_CHARS = Math.min(5, fullText.length); // Start with first 5 chars visible
      let charIndex = INITIAL_CHARS;

      setStreamingTranscript({
        id: latestTranscript.id,
        visibleText: fullText.substring(0, INITIAL_CHARS),
        fullText: fullText
      });

      streamingIntervalRef.current = setInterval(() => {
        charIndex += charsPerTick;

        if (charIndex >= fullText.length) {
          // Streaming complete
          clearInterval(streamingIntervalRef.current!);
          streamingIntervalRef.current = null;
          setStreamingTranscript(null);
        } else {
          setStreamingTranscript(prev => {
            if (!prev) return null;
            return {
              ...prev,
              visibleText: fullText.substring(0, charIndex)
            };
          });
        }
      }, INTERVAL_MS);
    }
  }, [transcripts, enableStreaming, isRecording]);

  // Cleanup streaming interval on unmount
  useEffect(() => {
    return () => {
      if (streamingIntervalRef.current) {
        clearInterval(streamingIntervalRef.current);
        streamingIntervalRef.current = null;
      }
      lastStreamedIdRef.current = null;
    };
  }, []);

  return (
    <div className="px-4 py-2">
      {/* Recording Status Bar - Sticky at top, always visible when recording */}
      <AnimatePresence>
        {isRecording && (
          <div className="sticky top-4 z-10 bg-white pb-2">
            <RecordingStatusBar isPaused={isPaused} />
          </div>
        )}
      </AnimatePresence>

      {transcripts?.map((transcript, index) => {
        const isStreaming = streamingTranscript?.id === transcript.id;
        const textToShow = isStreaming ? streamingTranscript.visibleText : transcript.text;
        // Clean up text for display - remove repetitions and filler words
        const filteredText = cleanStopWords(textToShow);
        // Show [Silence] ONLY if the ORIGINAL transcript was empty (not just after filtering)
        const originalWasEmpty = transcript.text.trim() === '';
        const displayText = originalWasEmpty && !isStreaming ? '[Silence]' : filteredText;

        // Sizer text: use cleaned version for proper sizing, fallback to [Silence] only if original was empty
        const sizerText = cleanStopWords(isStreaming ? streamingTranscript.fullText : transcript.text)
          || (originalWasEmpty && !isStreaming ? '[Silence]' : '');

        // P1-G: speaker 标签 (仅在 transcript.speaker 已落库时显示)
        const speakerIdx = parseSpeakerIndex(transcript.speaker);
        const speakerStyle = speakerIdx !== null ? SPEAKER_PALETTE[speakerIdx % SPEAKER_PALETTE.length] : null;

        return (
          <motion.div
            key={transcript.id ? `${transcript.id}-${index}` : `transcript-${index}`}
            initial={{ opacity: 0, y: 5 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.15 }}
            className="mb-3"
          >
            <div className="flex items-start gap-2">
              <Tooltip>
                <TooltipTrigger>
                  <span className="text-xs text-gray-400 mt-1 flex-shrink-0 min-w-[50px]">
                    {transcript.audio_start_time !== undefined
                      ? formatRecordingTime(transcript.audio_start_time)
                      : transcript.timestamp}
                  </span>
                </TooltipTrigger>
                <TooltipContent>
                  {transcript.duration !== undefined && (
                    <span className="text-xs text-gray-400">
                      {transcript.duration.toFixed(1)}s
                      {transcript.confidence !== undefined && (
                        <ConfidenceIndicator
                          confidence={transcript.confidence}
                          showIndicator={showConfidence}
                        />
                      )}
                    </span>
                  )}
                </TooltipContent>
              </Tooltip>
              <div className="flex-1 min-w-0">
                {speakerStyle && (
                  // P1-G: speaker 标签 (S1 / S2 ...), 颜色按 speaker index 选
                  <span
                    className={`inline-block text-[10px] font-medium px-1.5 py-0.5 rounded mb-1 mr-2 align-middle ${speakerStyle.bg} ${speakerStyle.text} ring-1 ${speakerStyle.ring}`}
                    title={`${transcript.speaker}`}
                  >
                    {speakerLabel(speakerIdx!)}
                  </span>
                )}
                {isStreaming ? (
                  // Streaming transcript - show in bubble (full width)
                  <div className={`bg-gray-100 border border-gray-200 rounded-lg px-3 py-2 ${speakerStyle ? `border-l-4 ${speakerStyle.bar.replace('bg-', 'border-l-')}` : ''}`}>
                    <div className="relative">
                      <p className="text-base text-gray-800 leading-relaxed" style={{ visibility: 'hidden' }}>
                        {sizerText}
                      </p>
                      <p className="text-base text-gray-800 leading-relaxed absolute top-0 left-0">
                        {displayText}
                      </p>
                    </div>
                  </div>
                ) : (
                  // Regular transcript - simple text with optional left color bar
                  <div className={`relative ${speakerStyle ? `pl-2 border-l-4 ${speakerStyle.bar.replace('bg-', 'border-l-')}` : ''}`}>
                    <p className="text-base text-gray-800 leading-relaxed" style={{ visibility: 'hidden' }}>
                      {sizerText}
                    </p>
                    <p className="text-base text-gray-800 leading-relaxed absolute top-0 left-0">
                      {displayText}
                    </p>
                  </div>
                )}
              </div>
            </div>
          </motion.div>
        );
      })}

      {/* v0.6.11+ bug fix: 灰色 partial 实时浮现 (解决 27s 空白问题) */}
      {/* v0.6.12+: 右侧加 decode_ms / buffer_age 灰字 chip, 让你直观看到延迟 */}
      {isRecording && !isStopping && !isPaused && !isProcessing && livePartialText && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.15 }}
          className="mb-3 ml-12"
        >
          <p className="text-sm text-gray-400 italic leading-relaxed">
            {livePartialText}
            <span className="inline-block w-1.5 h-3.5 bg-gray-400 ml-0.5 align-middle animate-pulse" />
          </p>
          <div className="flex items-center gap-2 mt-1">
            {typeof lastDecodeMs === 'number' && (
              <span
                data-testid="transcript-decode-ms"
                className={
                  "text-[10px] font-mono px-1.5 py-0.5 rounded " +
                  (lastDecodeMs > 200
                    ? "text-orange-700 bg-orange-100"
                    : lastDecodeMs > 100
                      ? "text-amber-700 bg-amber-50"
                      : "text-gray-400 bg-gray-100")
                }
                title="sherpa-onnx 本次 decode 耗时"
              >
                decode {lastDecodeMs}ms
              </span>
            )}
            {typeof lastBufferAgeMs === 'number' && lastBufferAgeMs > 100 && (
              <span
                className="text-[10px] font-mono text-gray-400"
                title="音频 buffer 已积累多久"
              >
                buf {lastBufferAgeMs}ms
              </span>
            )}
            {isPartialEndpoint && (
              <span className="text-xs text-gray-300">正在切句…</span>
            )}
          </div>
        </motion.div>
      )}

      {/* Show listening indicator when recording and has transcripts */}
      {!isStopping && isRecording && !isPaused && !isProcessing && transcripts.length > 0 && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="flex items-center gap-2 mt-4 text-gray-500"
        >
          <div className="w-2 h-2 bg-blue-500 rounded-full animate-pulse"></div>
          <span className="text-sm">正在聆听...</span>
        </motion.div>
      )}

      {/* Empty state when no transcripts */}
      {transcripts.length === 0 && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className="text-center text-gray-500 mt-8"
        >
          {isRecording ? (
            <>
              <div className="flex items-center justify-center mb-3">
                <div className={`w-3 h-3 rounded-full ${isPaused ? 'bg-orange-500' : 'bg-blue-500 animate-pulse'}`}></div>
              </div>
              <p className="text-sm text-gray-600">
                {isPaused ? '录音已暂停' : '正在聆听语音...'}
              </p>
              <p className="text-xs mt-1 text-gray-400">
                {isPaused
                  ? '点击继续录音'
                  : '点击开始录音, 即可看到实时转录文字'}
              </p>
            </>
          ) : (
            <>
              <p className="text-lg font-semibold">欢迎使用离线会记!</p>
              <p className="text-xs mt-1">开始录音, 即可看到实时转录文字</p>
            </>
          )}
        </motion.div>
      )}
    </div>
  );
};
