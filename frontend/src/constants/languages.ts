// 离线会记 W2.5: 精简到实际可用语言
// SenseVoice-zh 支持: zh / yue / en / ja / ko
// Paraformer-zh 仅支持 zh
// Whisper 已删除 (不需要 36 语言列表)
// 中英标签: 方便中英用户使用

export type LanguageOption = {
  code: string;
  name: string;        // 英文名 (兼容老代码)
  nameZh: string;      // 中文名
};

export const LANGUAGES: LanguageOption[] = [
  { code: 'auto', name: 'Auto Detect',          nameZh: '自动检测 (推荐)' },
  { code: 'zh',   name: 'Chinese (Mandarin)',   nameZh: '中文 (普通话)' },
  { code: 'yue',  name: 'Chinese (Cantonese)',  nameZh: '粤语' },
  { code: 'en',   name: 'English',              nameZh: '英语' },
  { code: 'ja',   name: 'Japanese',             nameZh: '日语' },
  { code: 'ko',   name: 'Korean',               nameZh: '韩语' },
];

/**
 * UI 显示用: 优先中文, fallback 英文
 */
export function displayLanguage(code: string, locale: 'zh' | 'en' = 'zh'): string {
  const opt = LANGUAGES.find(l => l.code === code);
  if (!opt) return code;
  return locale === 'zh' ? opt.nameZh : opt.name;
}
