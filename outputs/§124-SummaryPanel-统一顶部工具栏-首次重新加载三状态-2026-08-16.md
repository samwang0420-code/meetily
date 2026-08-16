# §124 SummaryPanel 顶部工具栏统一 — 首次/重新/加载三状态共享 (2026-08-16)

## 背景

用户 8/16 截图反馈: "重新生成摘要页面和第一次生成的页面不一样, 你要统一了哦".

之前 SummaryPanel.tsx 有 **3 套不同的 UI** (按状态切换):

| 状态 | 顶部工具栏 | 主区 |
|---|---|---|
| `!aiSummary` (首次生成) | **居中** 渲染 `<SummaryGeneratorButtonGroup>` (含 4 元素: 生成/语言/Lang Slot/⚙️Dialog/模板) + `EmptyStateSummary` | 居中 |
| `isSummaryLoading` (加载中) | **居中** 渲染 `<SummaryGeneratorButtonGroup>` (同 4 元素) | 流式 markdown / spinner |
| `aiSummary 存在` (重新生成) | **§110 设计** 4 元素 (说话人 / 重新生成 / ⚙️ Dropdown / 📤 Dropdown) | BlockNoteSummaryView 编辑视图 |

**问题**: 同一会议详情页, 用户进入"首次生成"和"重新生成", 看到 2 套不同的按钮组 (位置/功能/图标都不同), 体验割裂.

## 改动 (1 文件, +82/-80)

### `frontend/src/components/MeetingDetails/SummaryPanel.tsx`

**1. 删除 dead imports**:
- `<SummaryGeneratorButtonGroup>` 从 SummaryPanel 移除 (本文件不再使用)
- `<SummaryUpdaterButtonGroup>` 从 SummaryPanel 移除 (本文件从未真正 render, 仅 import 占用)
- 加 `Square` icon (lucide-react) — 用于"停止"按钮

**2. 顶部工具栏条件 `{aiSummary && !isSummaryLoading && (...)}` → `{!isSummaryLoading && (...)}`**:
- 3 状态都渲染同一套工具栏 (位置 `flex items-center justify-center w-full gap-2`)
- 仅 `isSummaryLoading = true` 时 (模型生成中) 不渲染, 改在主区显示 spinner

**3. 主按钮 3 态**:
| 状态 | 显示 | 行为 |
|---|---|---|
| `isSummaryLoading` | 红色 "■ 停止" | 调 `onStopGeneration()` |
| `aiSummary 存在` (&& !loading) | 蓝紫渐变 "✨ 重新生成" | 调 `onRegenerateSummary()` |
| `!aiSummary` | 蓝紫渐变 "✨ 生成摘要" | 调 `onGenerateSummary(customPrompt)` (无 transcripts 时 disabled) |

**4. 说话人 button** (`!aiSummary` 时隐藏):
- 条件渲染 `{aiSummary && <Button>...}</Button>}`
- 逻辑: 没摘要就没"已识别说话人"的可改意义, 隐藏而非 disabled

**5. ⚙️ 设置下拉**:
- Trigger button 加 `disabled={isSummaryLoading}` — 加载中无法改设置

**6. 📤 导出下拉**:
- Trigger button 加 `disabled={isSummaryLoading}` — 加载中无法导出
- (已经 `disabled={!aiSummary}` 在 4 个 DropdownMenuItem 上: 复制/MD/TXT/导出, 这些保持不变)

**7. ⚙️ 内模板 DropdownMenuItem**:
- 显示当前已选模板名 (`{selectedTemplateName || t('summary.template')}`, 加 `max-w-[120px] truncate`)
- 与 §123 SummaryGeneratorButtonGroup 内模板 dropdown 行为一致

**8. 删除 2 处居中的 SummaryGeneratorButtonGroup**:
- `isSummaryLoading ? (...)` 分支: 只保留流式 markdown + spinner
- `!aiSummary ? (...)` 分支: 只保留 EmptyStateSummary

### 主区 3 态 (也统一化, 不再"居中 button group 镶嵌主区")

```tsx
{isSummaryLoading ? (
  <div className="flex-1 min-h-0 overflow-y-auto px-6 pb-6">
    {/* 流式 markdown 或 spinner */}
  </div>
) : !aiSummary ? (
  <div className="flex-1 min-h-0 overflow-y-auto px-6 pb-6 pt-8">
    <EmptyStateSummary ... />
  </div>
) : transcripts?.length > 0 && (
  <div className="flex-1 overflow-y-auto min-h-0">
    {/* BlockNoteSummaryView + summaryResponse (deprecated) */}
  </div>
)}
```

## 用户感知

进入"导入音频 2026-08-14 14:05"会议 (已有摘要):
- **之前**: §110 4 元素按钮 (说话人/重新生成/⚙️/📤) — OK
- **现在**: 同一套 4 元素按钮 — 一致

进入"未命名会议" (没摘要):
- **之前**: 居中的 SummaryGeneratorButtonGroup (不同 UI) — 困惑
- **现在**: 同样的 4 元素按钮, "说话人" 自动隐藏, 主按钮显示"✨ 生成摘要" — 一致

## 验证 (§37 硬闸门)

- ✅ tsc --noEmit: 1 个 §18 bun:test (不动)
- ✅ next build: OK
- ✅ cargo check --lib: 0 errors / 28 warnings (§18 不动)
- ✅ cargo test --lib: **337 passed / 0 failed / 3 ignored**
- ✅ cargo build --release: 1m32s, binary 72M
- ✅ check_historical_fixes.py: **200/200 PASS** (+7 §124 anchors)
- ✅ sync_app_bundle.sh: 全 sync + §98 codesign

## §15 GUI 验收 (用户必做, 不能 CLI 测)

```bash
killall meetily 2>/dev/null
bash /Users/wangwei/Documents/离线会记/scripts/sync_app_bundle.sh
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

1. 进入"未命名会议" (没摘要) → 顶部 4 元素按钮显示: [**生成摘要** / **⚙️** / **导出MD** disabled], "说话人" 隐藏
2. 点击"✨ 生成摘要" → 进度开始, 主按钮变红色 "■ 停止", ⚙️ 和 📤 disabled
3. 生成完成 → 主按钮变 "✨ 重新生成", ⚙️ 📤 enabled, "说话人" 按钮出现
4. 对比老图标 vs 新图标, 应跟之前完全一致
5. 任何状态下, ⚙️ 内模板 dropdown 显示当前已选模板名 (不是"模板"两字)

## 铁律 (扩展 §18 / §104 / §106)

1. **同一逻辑面板 3 状态必须共享工具栏** — 不允许"首次进入 vs 重生成"看到不同按钮组
2. **Disabled 通过属性控制, 不是把元素整个隐藏** — 让用户能看到按钮存在, 只是当前不可点
3. **隐藏 ≠ 删除** (§110 + §104): "说话人 button" 是隐藏 (没摘要时), 不是删除源代码
4. **dead import 必须清理** — SummaryUpdaterButtonGroup 在 SummaryPanel 只是 import 占位 (从来不被 render), 必须 delete
5. **保持 §123 模板名显示行为统一** — SummaryGeneratorButtonGroup 内模板显示方式 (max-w truncate + selectedTemplateName) 在 §110 ⚙️ DropdownMenu 内 Template 项也用同样模式

## 关联

- [[124-SummaryPanel-统一顶部工具栏-三状态]] (Obsidian)
- `outputs/§124-...md` (Codex)
- §110 (4 元素工具栏设计初次引入)
- §123 (selectedTemplateName 模板名显示)
- §18 (hidden ≠ deleted 原则)
- §37 (硬闸门) / §92 (决策迁移铁律) / §15 (GUI 验收)

## commit

```
<next> fix(§124): SummaryPanel 顶部工具栏统一, 3 状态共享同一套 4 元素按钮
```
