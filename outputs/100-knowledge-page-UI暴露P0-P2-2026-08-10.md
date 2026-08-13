# §100 UI 暴露 P0-P2 大改造 (2026-08-10)

## 触发
用户原话: "我完全没有感受到它的变化" — 71 报告 P0-P2 功能代码都在, 但 UI 上没有任何提示/入口, 用户感知不到。

## 实现

### 1. Sidebar 新增 "📚 知识" nav item
`frontend/src/components/Sidebar/index.tsx`:
- Home 和 Recording CTA 之间插入新 nav button
- 主题紫色 (violet) 高亮, 跟其它 nav item 区分 (蓝=home, 紫=knowledge)
- 折叠态: BookOpen icon + 计数 badge; 展开态: 同样 + 中文"知识图谱"
- 60s interval refresh topic count (P0-A 后端 `api_topic_recent`)

### 2. 新页 `/knowledge` (600 行, 全 P0-P2 集中展示)
`frontend/src/app/knowledge/page.tsx`:
- 顶部 4 stat cards: 主题总数 / 决议数 / 项目数 / 待办行动项
- 左主区: 主题列表 (按 type 过滤: 全部/项目/决议/人物/话题)
  - 每条主题显示: 类型 chip + canonical_name + status + mention_count + 时间戳 + "已决议" 标记
- 右侧 4 panels:
  1. **Topic Dossier** (P0-A): 点击主题 → 弹出 dossier 详情, 含 summary / last_decided / open_questions / episodes + "重建档案" 按钮
  2. **行动项** (P2-A): checkbox toggle 持久化, 显示会议标题
  3. **Obsidian 同步状态** (P0-B): vault_path + enabled 状态 + 入口到 Settings
  4. **MCP Server** (P1-A): 5 tools 列表 + 配置入口
- 底栏: **夜间重建** (P2-B) 状态: 0:00-6:00 + idle 30min + 默认 3 个/晚

### 3. i18n 新增
- `nav.knowledge` zh "知识图谱" / en "Knowledge Graph" (Sidebar nav 用)

## Tauri commands 复用
全部已存在, 直接调:
- `api_topic_recent` (P0-A)
- `api_topic_search` (P0-A)
- `api_topic_get_dossier` (P0-A)
- `api_topic_rebuild_dossier` (P0-A)
- `api_action_item_list` / `api_action_item_toggle` (P2-A) 
- `api_obsidian_get_settings` (P0-B)

## §37 硬闸门
- ✅ tsc --noEmit: 0 errors (1 §18 bun:test 不动)
- ⚠️ next build: 需要网络 (Google Fonts Source_Sans_3), sandbox 跑不了
  → 用户终端跑 (有网络)
- ⏳ cargo build --release: 用户终端跑
- ⏳ sync_app_bundle.sh: 用户终端跑

## 用户手动命令
```bash
cd /Users/wangwei/Documents/离线会记

# 1. next build (需网络)
cd frontend && npx next build

# 2. cargo build --release
cd src-tauri && cargo build --release

# 3. sync bundle
cd ../..
bash scripts/sync_app_bundle.sh

# 4. commit + push
git add frontend/src/app/knowledge/page.tsx \
        frontend/src/components/Sidebar/index.tsx \
        frontend/src/i18n/locales/zh.ts \
        frontend/src/i18n/locales/en.ts \
        outputs/100-knowledge-page-UI暴露P0-P2-2026-08-10.md

git -c user.email=codex@local -c user.name=codex commit -m "feat(§100): UI 暴露 P0-P2 — Sidebar 知识 nav + /knowledge 页

71 报告 P0-P2 功能代码全在, 但 UI 无入口, 用户感知不到 (§91 反馈).
本次让用户能看到:

- Sidebar 加 '📚 知识' nav item (紫色高亮, 跟 home/settings 区分)
  60s interval 刷新主题计数 badge
- 新页 /knowledge (600 行):
  - 4 stat cards: 主题/决议/项目/待办
  - 左侧主题列表 (按 type 过滤)
  - 右侧 4 panels: Topic Dossier / 行动项 / Obsidian / MCP Server / 夜间重建
- i18n 加 nav.knowledge

复用现有 Tauri commands (api_topic_*, api_action_item_*, api_obsidian_*)

§37 闸门: tsc 0 errors / next build (用户跑) / cargo (用户跑)"

git push origin perf/summary-map-concurrency

# 5. Obsidian 双写
cp outputs/100-knowledge-page-UI暴露P0-P2-2026-08-10.md \
   "$HOME/Documents/Obsidian Vault/项目/3-离线会记/100-knowledge-page-UI暴露P0-P2-2026-08-10.md"

# 6. GUI 验证
killall meetily 2>/dev/null
'/Users/wangwei/Documents/离线会记/target/release/bundle/macos/言镜 AI.app/Contents/MacOS/meetily' &
# 期望: Sidebar 看到紫色 "知识图谱" 按钮 (有数字 badge)
# 点击 → /knowledge 页 → 看到 4 stat cards + 主题列表 + 右侧 4 panels
```

## 关联
- §91 P0-P2 完整收尾 (功能代码已就位)
- §85 §88 §87 (功能 commit 历史)
- §37 硬闸门 / §92 防代码漏

## 未做 (按 §18 不主动改)
- Dashboard widgets (HomeDashboard 加 "📌 待办" + "💡 主题" 两块)
  → 用户先看 /knowledge 页效果, 再决定是否加
- Settings 加 Obsidian/MCP 配置 UI
  → §P0-B / §P1-A 配置 UI 暂留空白 (ObsidianSettings 已存在, MCP 没有)
- RecordingControls ⌥+Space hint
  → §P2-C LiveQA 已实装, 没明显提示用户
```

脚本准备