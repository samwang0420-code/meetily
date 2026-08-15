# §122 action_items parser 兼容多模板 (2026-08-15 立)

## 触发

§121 修完后审计发现:
- `action_items` 表 0 行
- `topic_node` / `topic_dossier` 表也是 0 行
- §121 修了 topic_graph 链路 (Ollama 替代 BuiltInAI)
- action_items 链路还有 bug: parser 只认 `**行动事项**` / `## 行动事项`, 漏抓法律/电商模板的标记

## 根因

`frontend/src-tauri/src/action_items/mod.rs::parse_markdown_action_items` 早期只识别 2 个 marker:

```rust
for marker in &["**行动事项**", "## 行动事项"] { ... }
```

但 §91 P2-A 完整化收尾时加了 8 个模板 (含 `legal_consultation` / `cross_border_ecommerce` / `medical_consultation`):

| 模板 | 行动事项 marker |
|---|---|
| `standard_meeting` | `**行动事项**` / `## 行动事项` |
| `legal_consultation` | `**待办事项**` / `## 待办事项` |
| `cross_border_ecommerce` | `**下周重点事项**` / `## 下周重点事项` |
| `medical_consultation` | (无对应, 用 `**待确认信息**` 而非行动项) |

→ 用户如果用法律/电商模板生成摘要, parser 全跳过, `action_items` 表 0 行.

## 修复

`frontend/src-tauri/src/action_items/mod.rs` parser marker 列表扩展为 6 个:

```rust
for marker in &[
    "**行动事项**", "## 行动事项",         // standard_meeting
    "**待办事项**", "## 待办事项",         // legal_consultation
    "**下周重点事项**", "## 下周重点事项",  // cross_border_ecommerce
] { ... }
```

表格解析逻辑不变 (兼容 markdown 表格 `| 事项 | 责任人 | 截止时间 |`).

## 新增测试 (2 个)

```rust
#[test]
fn test_parse_legal_template_todo_marker() {
    let md = "**基本事实**\n...\n\n**待办事项**\n\n| 事项 | 责任人 | 截止时间 |\n| --- | --- | --- |\n| 准备起诉状 | 张律师 | 6 月 15 日 |\n| 调查取证 | 王律师 | 未明确 |\n\n**遗留问题**\n无";
    let items = parse_markdown_action_items(md);
    assert_eq!(items.len(), 2);
    assert!(items[0].contains("准备起诉状"));
    assert!(items[0].contains("张律师"));
}

#[test]
fn test_parse_ecommerce_template_marker() {
    let md = "**风险与卡点**\n...\n\n**下周重点事项**\n\n| 事项 | 负责人 | 截止时间 |\n| --- | --- | --- |\n| 投放 TikTok | 王伟 | 6 月 20 日 |\n";
    let items = parse_markdown_action_items(md);
    assert_eq!(items.len(), 1);
    assert!(items[0].contains("投放 TikTok"));
}
```

## §37 6 步硬闸门

- ✅ cargo check --lib: 0 errors (28 §18 warnings 不动)
- ✅ cargo test --lib action_items: **7/7 PASS** (含 2 个 §122 新测试)
- ✅ check_historical_fixes.py: 176 → **180/180 PASS** (+4 §122 anchor)

## §122 铁律

1. **新增任何模板必须在 §X commit 同步更新 action_items parser marker** — 不允许 "模板加了一节但 parser 没接"
2. **占位 marker 必须随模板加**: 新模板若 marker 段落允许空 (`本次无...`), 必须同步加占位字符串到 `ACTION_ITEMS_PLACEHOLDERS`
3. **parser 测试矩阵**: 新 marker 必须有一个独立 `#[test]` 覆盖, 防止回归

## §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
bash /Users/wangwei/Documents/离线会记/scripts/sync_app_bundle.sh
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'

# 1) 设置 → 模型设置 → 模板选"法律咨询"或"跨境电商"
# 2) 重生成 566fe7a9 (1h50m 科普音频, 实际上无法律内容) 摘要, 验证不报错
# 3) 或新录一段会议选法律模板 → 看 action_items 表格是否抓到 待办事项
```

## 已知边界

- `medical_consultation` 没有"行动事项"对应 (用 `**待确认信息**`), 不应被 parser 误抓
- 已存在的 completed summaries (28a6c63c 等) 不会自动重解析 — 用户必须主动"重新生成摘要"才生效
- 老 action_items 数据保留 (不动)

## 关联

- §91 (P2-A 完整化收尾, §122 是其 parser 漏兼容补丁)
- §121 (同 session 修复 topic_graph silent fail)
- §85 (MVP 起点) / §18 / §37 / §15
- [[122-action-items-parser-兼容多模板-2026-08-15]] (Obsidian)
