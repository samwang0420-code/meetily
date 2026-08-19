// 工具: 把 fact_guard 报出的 unexpected_dates/numbers 在 markdown 中用 == 包裹, 让 BlockNote 渲染成高亮

export interface HighlightableFactGuard {
  unexpected_dates?: string[];
  unexpected_numbers?: string[];
}

// 转义 markdown 特殊字符, 避免破坏现有语法
function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

// 在 markdown 文本中查找并包装 unexpected 项
// - 用 ==xxx== 包裹 (BlockNote markdown highlight 语法)
// - 已存在 ==xxx== 包裹的不重复
// - 跳过代码块 (```) 和行内 code (`...`)
export function highlightUnexpectedFacts(
  markdown: string,
  report: HighlightableFactGuard | null | undefined
): string {
  if (!markdown || !report) return markdown;
  const items: string[] = [];
  if (report.unexpected_numbers?.length) items.push(...report.unexpected_numbers);
  if (report.unexpected_dates?.length) items.push(...report.unexpected_dates);
  if (items.length === 0) return markdown;

  // 按长度倒序, 先匹配长的 (避免 "2017 年" 抢在 "2017 年 8 月 26 日" 之前)
  const sorted = [...new Set(items)]
    .filter(s => s && s.length >= 2)
    .sort((a, b) => b.length - a.length);

  // 按行处理, 跳过 code block / inline code
  const lines = markdown.split('\n');
  let inCodeBlock = false;
  const processed: string[] = [];
  for (const line of lines) {
    if (line.startsWith('```')) {
      inCodeBlock = !inCodeBlock;
      processed.push(line);
      continue;
    }
    if (inCodeBlock) {
      processed.push(line);
      continue;
    }
    // 跳过纯 inline code 行 (e.g. `code only`)
    // 用临时占位符保护 inline code 内容
    const codeSpans: string[] = [];
    let masked = line.replace(/`[^`\n]+`/g, (m) => {
      const i = codeSpans.length;
      codeSpans.push(m);
      return `\u0000CODE${i}\u0000`;
    });
    for (const item of sorted) {
      const re = new RegExp(escapeRegex(item), 'g');
      // 已 ==xxx== 包裹的, 跳过
      masked = masked.replace(re, (match, offset, full) => {
        // 检查前后是否已经在 highlight 块里
        const before = full.slice(Math.max(0, offset - 2), offset);
        const after = full.slice(offset + match.length, offset + match.length + 2);
        if (before.endsWith('==') || after.startsWith('==')) return match;
        return `==${match}==`;
      });
    }
    // 还原 inline code
    masked = masked.replace(/\u0000CODE(\d+)\u0000/g, (_, i) => codeSpans[Number(i)]);
    processed.push(masked);
  }
  return processed.join('\n');
}
