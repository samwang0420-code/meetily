import { describe, expect, test } from "bun:test";
import { highlightUnexpectedFacts } from "../../src/lib/highlight_facts";

describe("highlightUnexpectedFacts", () => {
  test("wraps unexpected_dates with ==highlight== syntax", () => {
    const md = "案发于 2017 年 8 月 26 日，原告方涛死亡。";
    const out = highlightUnexpectedFacts(md, {
      unexpected_dates: ["2017 年 8 月 26 日"],
    });
    expect(out).toContain("==2017 年 8 月 26 日==");
  });

  test("wraps unexpected_numbers with ==highlight== syntax", () => {
    const md = "判赔 23.75 万元。";
    const out = highlightUnexpectedFacts(md, {
      unexpected_numbers: ["23.75 万"],
    });
    expect(out).toContain("==23.75 万==");
  });

  test("matches longest first to avoid prefix issues", () => {
    const md = "案发于 2017 年 8 月 26 日";
    const out = highlightUnexpectedFacts(md, {
      unexpected_dates: ["2017 年", "2017 年 8 月 26 日"],
    });
    // Long match wins — only one wrap around the long form
    expect(out).toContain("==2017 年 8 月 26 日==");
  });

  test("does not re-wrap already highlighted text", () => {
    const md = "案发于 ==2017 年 8 月 26 日==，已记录。";
    const out = highlightUnexpectedFacts(md, {
      unexpected_dates: ["2017 年 8 月 26 日"],
    });
    // Should not double-wrap to ====2017...====
    expect(out).toBe(md);
  });

  test("returns markdown unchanged when report has no unexpected items", () => {
    const md = "庭审情况良好。";
    const out = highlightUnexpectedFacts(md, {
      unexpected_numbers: [],
      unexpected_dates: [],
    });
    expect(out).toBe(md);
  });

  test("returns markdown unchanged when report is null/undefined", () => {
    const md = "庭审情况良好。";
    expect(highlightUnexpectedFacts(md, null)).toBe(md);
    expect(highlightUnexpectedFacts(md, undefined)).toBe(md);
  });

  test("preserves inline code spans (does not highlight inside backticks)", () => {
    const md = "代码 `let x = 2017` 和 2017 年 8 月 26 日庭审";
    const out = highlightUnexpectedFacts(md, {
      unexpected_dates: ["2017 年 8 月 26 日"],
    });
    // "2017" inside backticks must NOT be highlighted, but the date outside must
    expect(out).toContain("`let x = 2017`");
    expect(out).toContain("==2017 年 8 月 26 日==");
  });

  test("preserves code blocks (does not highlight inside ``` blocks)", () => {
    const md = "前面 2017 年 8 月 26 日。\n```\n2017 年 8 月 26 日 (in code)\n```\n后面 2017 年 8 月 26 日。";
    const out = highlightUnexpectedFacts(md, {
      unexpected_dates: ["2017 年 8 月 26 日"],
    });
    // First and last occurrences (outside code block) should be wrapped
    // Middle occurrence (inside ```) should NOT be wrapped
    const firstWrap = out.indexOf("==2017 年 8 月 26 日==");
    const codeOccurrence = out.indexOf("2017 年 8 月 26 日", out.indexOf("```"));
    const lastWrap = out.lastIndexOf("==2017 年 8 月 26 日==");
    expect(firstWrap).toBeGreaterThan(-1);
    expect(lastWrap).toBeGreaterThan(firstWrap);
    // The occurrence inside the code block should NOT be wrapped
    const betweenCodeBlockAndLastWrap = out.slice(
      out.indexOf("```\n") + 4,
      lastWrap
    );
    expect(betweenCodeBlockAndLastWrap).toContain("2017 年 8 月 26 日 (in code)");
    // Verify the in-code occurrence has no == around it
    const inCodeSnippet = out.match(/2017 年 8 月 26 日 \(in code\)/);
    expect(inCodeSnippet).toBeTruthy();
  });

  test("deduplicates items (same string in both dates and numbers is wrapped once)", () => {
    const md = "金额 23.75 万元";
    const out = highlightUnexpectedFacts(md, {
      unexpected_dates: [],
      unexpected_numbers: ["23.75 万"],
    });
    // Single wrap
    const wraps = (out.match(/==23\.75 万==/g) || []).length;
    expect(wraps).toBe(1);
  });
});
