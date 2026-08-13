# meetily-mcp — 言镜 AI 的 MCP server

> **§P1-A** — 让 Claude / Cursor / Continue.dev / Cline 等 AI assistant 直接读取你的会议纪要、行动项、关键决议。

## 是什么

[Model Context Protocol (MCP)](https://modelcontextprotocol.io) 是 Anthropic 推动的开放协议，让 AI assistant 通过 stdio JSON-RPC 调用本地工具。Granola / Otter 都已经出 MCP server 抢 AI 生态入口。言镜 AI v0.8.6 起跟进 — 用户在 Claude Desktop 里直接问"上周的会议有哪些行动项"，AI 就能调用这个 server 从本机 SQLite 查出来。

## 暴露的 3 个 tool

| Tool | 作用 | 输入 |
|---|---|---|
| `search_meetings` | 按 title/id 搜会议 | `query` (必填) + `limit` (默认 20, max 100) |
| `get_meeting_summary` | 取单个会议完整摘要 + 行动项 + 关键决议 | `meeting_id` (必填) |
| `get_action_items` | 跨会议拉所有 action items | `date_from` / `date_to` (可选) + `limit` (默认 100, max 500) |

## 数据库位置

默认读取：
- macOS: `~/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite`
- Windows: `%APPDATA%/cn.lixianhuiji.app/meeting_minutes.sqlite`
- Linux: `~/.local/share/cn.lixianhuiji.app/meeting_minutes.sqlite`

可通过 `MEETILY_DB_PATH` 环境变量覆盖。

**权限**: 整个 server 是 **read-only** (用 `SQLITE_OPEN_READ_ONLY` 打开 DB)，永远不会写你的会议数据。

## 编译

```bash
cd /Users/wangwei/Documents/离线会记
cargo build -p meetily-mcp --release
# binary: target/release/meetily-mcp (3.6 MB)
```

## Claude Desktop 配置

`~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "meetily": {
      "command": "/Users/wangwei/Documents/离线会记/target/release/meetily-mcp",
      "args": []
    }
  }
}
```

重启 Claude Desktop 即可在工具栏看到 🔨 3 个工具（`search_meetings` / `get_meeting_summary` / `get_action_items`）。

## Cursor 配置

`~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "meetily": {
      "command": "/Users/wangwei/Documents/离线会记/target/release/meetily-mcp"
    }
  }
}
```

## Continue.dev 配置

`~/.continue/config.json` (`.models` 同级):

```json
{
  "experimental": {
    "modelContextProtocolServers": [
      {
        "name": "meetily",
        "transport": {
          "type": "stdio",
          "command": "/Users/wangwei/Documents/离线会记/target/release/meetily-mcp"
        }
      }
    ]
  }
}
```

## 使用示例

对话中（Claude Desktop 任意 chat）：

> "用 meetily 搜一下 8 月初关于'API 限流'的会议，给我摘要和行动项"

Claude 会自动：
1. 调 `search_meetings(query="API 限流", limit=5)` 拿候选
2. 找到匹配的 meeting_id 后调 `get_meeting_summary(meeting_id=...)` 拿完整内容
3. 把 markdown 摘要 + 行动项呈现给你

## 协议实现

- MCP 协议版本: `2024-11-05`
- Transport: stdio JSON-RPC 2.0
- 错误码: `-32700` (parse) / `-32600` (invalid) / `-32601` (method not found) / `-32602` (invalid params) / `-32603` (internal) / `-32004` (meeting not found)
- Rust 1.77+ / 无运行时依赖 (rusqlite bundled SQLite)

## 关联

- 71 报告 P1-A (Granola / Otter MCP 对标)
- §P0-B Obsidian 写入 (共享 DB)
- §P0-A 跨会议知识图谱 (路线图 — `get_topic_status` 是第 5 个 tool 候选)
