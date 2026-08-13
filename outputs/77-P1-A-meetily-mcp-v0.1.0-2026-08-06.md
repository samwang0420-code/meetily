# §77 P1-A meetily-mcp v0.1.0 — stdio JSON-RPC MCP server (2026-08-06)

## 触发
71 报告 P1-A: Granola / Otter 都出 MCP server 抢 AI 生态入口, 3-5 天工作量, ROI 高 (Claude/Cursor 用户可调我们数据).

## 交付
- **新 Cargo workspace member** `meetily-mcp` (独立 binary, 3.6 MB release)
- **stdio JSON-RPC 2.0** transport 实现 MCP 协议 2024-11-05
- **read-only rusqlite** 直接读 `~/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite` (零写权限, 用户数据安全)
- **3 个 tool v0.1.0**:
  - `search_meetings(query, limit)` — LIKE 模糊搜 title/id
  - `get_meeting_summary(meeting_id)` — 返 summary_markdown + action_items + key_points (从 transcripts 字段 + summary_processes.result JSON)
  - `get_action_items(date_from, date_to, limit)` — 跨会议 action items
- **4 个单测** 全过 (parse_csv_field / jsonrpc format × 2 / db_path)
- **实测用真 DB**: search_meetings 返 56 会议中的 3 真实 ID (f2b73add / 2a0dea87 / 8bffd804), get_meeting_summary 返 f2b73add 完整 828 段

## 部署文档
`meetily-mcp/README.md` 含完整配置:
- **Claude Desktop** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Cursor** `~/.cursor/mcp.json`
- **Continue.dev** `~/.continue/config.json` (experimental.modelContextProtocolServers)

## Roadmap (后续版本)
- v0.2.0: `get_topic_status` (接 P0-A 知识图谱)
- v0.3.0: `create_meeting_note` (写入草稿, Pro 权限)
- v0.4.0: SSE transport (远程调用 — §18 后置)

## 协议合规
- JSON-RPC 2.0 spec
- 错误码: -32700 (parse) / -32600 (invalid) / -32601 (method not found) / -32602 (invalid params) / -32603 (internal) / -32004 (meeting not found)
- protocolVersion "2024-11-05" 兼容 Claude Desktop / Cursor / Cline / Continue.dev
- 4 method: `initialize` / `ping` / `tools/list` / `tools/call` + 2 notification (silenced)

## 关联
- 71 报告 P1-A
- §28 决策迁移铁律 (3 处同日落)
- [[项目/3-离线会记/76-P1-C-官网对比表-2026-08-06]] (P1-C 完成)
- [[项目/3-离线会记/75-P0-B-Obsidian-vault-写入-Phase2-2026-08-06]] (P0-B 完成)
- commit 2c9ab52
