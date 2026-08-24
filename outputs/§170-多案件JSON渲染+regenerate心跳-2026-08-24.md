# §170 多案件 JSON 渲染 + regenerate 心跳 + 二次审计修正 (2026-08-24)

## 触发
用户截图反馈 4 个问题:
1. 录音涉及两个案件, 没有区分
2. 没有任何格式 (实际是 `[{ "case_index": 1, ... }]` JSON 字符串被当 markdown 渲染)
3. 重新生成卡了十几分钟不动, 点了停止又重新点才成功
4. 审计"都做到了吗", 怀疑没全部做到

附件需求核心:
- 多案件 → JSON 数组 (§165 已实装后端, **前端 §170 真正渲染**)
- UI 视觉分区 (左原始转录 + 右 AI 纪要 + 顶部免责声明)
- 多段独立内容警告
- 推理参数固化 (temp=0.1 / top_p=0.3 / rep_penalty=1.05) — §163 已实装
- 硬后处理 fix_mapping + 拼音编辑距离 — §164 已实装

## 修复 (4 项)

### §170.1 BlockNoteSummaryView 多案件 JSON 数组渲染
- `frontend/src/components/AISummary/BlockNoteSummaryView.tsx`:
  - `detectSummaryFormat` 新增 Priority 2.5: 检测 `markdown.trimStart().startsWith('[{')` + JSON.parse + `parsed[0]?.case_index !== undefined` → 返回 `format: 'multi-case'`
  - 新增独立 `MultiCaseCard` component: 每案件一个独立 `useCreateBlockNote` 实例 (避免多 case 共享 editor state)
  - 渲染: 橙色边框 Card + 圆形 case_index 徽章 + defendant 名 + warning 标签 + content 调 BlockNote 渲染 (fact_guard 黄底高亮复用 §141 D 方案)
- `frontend/src/types/index.ts`:
  - `SummaryFormat = 'legacy' | 'markdown' | 'blocknote' | 'multi-case'`

### §170.2 SummaryPanel detectMultiCaseSummary 改用 raw markdown
- `frontend/src/components/MeetingDetails/SummaryPanel.tsx:detectMultiCaseSummary`:
  - 旧: 从 BlockNote aiSummary 结构提取 `inline.text` 拼接 (易因 BlockNote 转义失败)
  - 新: 直接读 `aiSummary.markdown` 原始字符串, JSON.parse 准确

### §170.3 SummaryPanel 5s 心跳 progress (防 Tauri 2 webview 事件丢包)
- `frontend/src/components/MeetingDetails/SummaryPanel.tsx`:
  - 新增 useEffect: `summaryStatus === 'processing' || 'regenerating'` 时启动 5s setInterval
  - `setSummaryProgress(prev => Math.min(95, prev + 5))` 慢慢往上爬
  - status 切换时 clearInterval 停止
  - 阶段 phase event 正常时由 `setSummaryProgress(pct)` 直接覆盖, 心跳只是 fallback
  - 用户反馈 "11 分钟 0% 转圈" 的根因: §160 invoke 30s + retry 1 = 60s 后端 timeout, 但实际 invoke 没 timeout (LLM 推理慢), spinner 0% 转 11 分钟, phase event 静默丢

### §170.4 useSummaryGeneration invoke 8s 心跳 toast
- `frontend/src/hooks/meeting-details/useSummaryGeneration.ts`:
  - `invokeWithTimeout('api_process_transcript', ...)` 前后用 try/finally 包裹
  - 8s heartbeat setInterval: `safeToast.info('请求中...', { description: '正在等待后端响应 (LLM 推理可能较慢)' })`
  - `clearInterval(heartbeatTimer)` 在 finally 清理
  - i18n 新增 `summary.requesting` / `summary.invoke_heartbeat` (zh + en)

## §37 6 步硬闸门
- ✅ tsc --noEmit: 0 errors
- ✅ next build: 35 routes, 1.45 MB /meeting-details (含多案件渲染分支)
- ✅ cargo check --lib: 0 errors
- ✅ cargo build --release: 4m11s, binary 55M mtime 11:57
- ✅ check_historical_fixes.py: 573 → **578/578 PASS** (5 个 §170 锚点)
- ✅ sync_app_bundle.sh: 3 binary 全 sync (言镜 AI + llama-helper + ffmpeg), sha 一致

## 验证 (用户必做, §15 强制)
```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 1. 打开故意杀人案庭审实录 (meeting-709b4aba) — 应看到多案件 Card 列表
#    Card 1: 三小 + 庭审内容
#    Card 2: 赵某 + 交通肇事内容 + 跨案件污染 warning
# 2. 顶部应有多案件警告 banner
# 3. 点"重新生成" — 进度条应该动 (心跳 fallback + phase event 双重)
# 4. 5s 后应看到 "请求中..." toast
```

## 关联
- §165 (后端 wrap_summary_as_multi_case_array)
- §166 (UI 多案件警告 banner — detectMultiCaseSummary §170.2 改用 raw markdown)
- §160 (in-flight guard + timeout — 仍然有效, 加上 §170.3/§170.4 fallback)
- §169 (regenerate force_fresh bypass cache)
- §54 (进度条 UI — §170.3 让 progress 真的能涨)
- §137 (navigation guard — 防 regenerate 时跳转)
- §141 D 方案 (highlightUnexpectedFacts 黄底高亮, 多案件每案件复用)
- commit hash (待 push 后填)
