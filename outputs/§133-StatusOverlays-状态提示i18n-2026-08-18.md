# §133 StatusOverlays 状态提示 i18n (2026-08-18)

## 触发

用户 8/18 截图: 录音停止后, 底部浮窗显示英文 "Finalizing transcription..." 一直转。
zh locale 用户看到英文 = i18n 漏适配 (跟 §104 / §107 / §90 同类)。

## 根因 (1 跳)

`frontend/src/app/_components/StatusOverlays.tsx:50` 硬编码英文:
```tsx
<StatusOverlay show={isProcessing} message="Finalizing transcription..." />
<StatusOverlay show={isSaving} message="Saving transcript..." />
```

之前整个组件没 import useTranslation, 跟 §107 (§104.1 first try) 一样 — 引入 t() 但漏放 keys。

## 修复 (3 文件, +16/-3)

1. **`frontend/src/app/_components/StatusOverlays.tsx`**:
   - `import { useTranslation } from '@/i18n'`
   - `const { t } = useTranslation()`
   - 2 个 message 改 `t('transcript.status_overlay.finalizing')` / `t('transcript.status_overlay.saving')`

2. **`frontend/src/i18n/locales/zh.ts`** (line 41 附近):
   ```ts
   status_overlay: {
     finalizing: '正在完成转录…',
     saving: '正在保存转录…',
   },
   ```

3. **`frontend/src/i18n/locales/en.ts`** (line 37 附近):
   ```ts
   status_overlay: {
     finalizing: 'Finalizing transcription...',
     saving: 'Saving transcript...',
   },
   ```

4. **`scripts/check_historical_fixes.py`**:
   - 3 个 §133 锚点: StatusOverlays 用 t() + zh 文案 + en 文案
   - guard **273 → 276/276 PASS**

## §37 硬闸门

- tsc --noEmit: 0 errors (1 §18 bun:test 已知不动)
- next build: OK
- cargo build --release: 1m51s, binary ~70M
- check_historical_fixes.py: **276/276 PASS** (273 → 276)
- sync_app_bundle.sh: 3 binary 全 sync

## §15 GUI 验收 (用户必做)

1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
3. 录音 30s → 停止 → 底部浮窗应该显示 "正在完成转录…" (zh) 或 "Finalizing transcription..." (en)
4. 切到英文 locale 应该显示 "Finalizing transcription..."

## 关联

- §104.1 (录音通知 toast 翻译尝试, 路径错)
- §107 (录音通知 toast 翻译未生效修复, 路径补)
- §90 (UI 漏代码 4 项)
- §56 (AGENTS.md §X 描述 vs 代码 commit)
- §18 (不主动改无关 bug)
- §37 (硬闸门)
