# §49 fix 落地 — commit 31f60e2 (2026-08-01)

## commit 信息
- **commit**: `31f60e2`
- **branch**: `fix/polling-30min` (从 main cff1621 切)
- **tag**: `v0.8.5`
- **binary**: `/Users/wangwei/Documents/meetily/target/release/meetily` (67.77 MB, mtime 15:01)

## 改动 4 文件

| 文件 | 行 | 改动 |
|---|---|---|
| SidebarProvider.tsx | 208 | MAX_POLLS 300 → 1800 |
| SidebarProvider.tsx | 224 | timeout_15min → timeout_30min |
| SidebarProvider.tsx | 218-235 | 超时前兜底查后端 status='completed' |
| i18n/locales/zh.ts | 543 | "摘要超时 (15 分钟)" → "(30 分钟)" |
| i18n/locales/en.ts | 653 | "Summary timeout (15 min)" → "(30 min)" |
| check_historical_fixes.py | 末尾 | 加 4 个 §49 锚点 |

## §37 硬闸门结果

| 步骤 | 结果 |
|---|---|
| npx tsc --noEmit | 0 errors (1 个 §18 bun:test 不动) |
| npx next build | 10s ✓ |
| cargo build --release | 1m31s ✓ (24 个 §18 warning 不动) |
| check_historical_fixes.py | **26/26 PASS** (22 → 26) |

## 验收命令

```bash
# 1) 替换运行中的 binary
killall meetily 2>/dev/null
open /Users/wangwei/Documents/meetily/target/release/meetily

# 2) 打开 a09de61d (谢关竹与江战...) 77 分钟录音, 应该直接显示已生成摘要:
sqlite3 "$HOME/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite" \
  "SELECT substr(result, 1, 200) FROM summary_processes WHERE meeting_id='meeting-a09de61d-0f7f-4798-b89f-f61649b3b961'"
# → "{\"english_cache\":{\"markdown\":\"# 谢关竹与江战关于入职及购买折叠屏手机的会议记录\\n\\n**会议摘要**\\n本次会议主要围绕主角谢关竹在南疆市的入职流程..."

# 3) 守卫
cd /Users/wangwei/Documents/meetily && python3 scripts/check_historical_fixes.py
# → Historical fix guard passed: 26/26
```

## 用户 GUI 验收 (按 §15 铁律必做)

1. **重启 app** → 看版本号 (footer / sidebar 显示 v0.8.5)
2. **打开 a09de61d** → 摘要面板应该直接显示完整 result (不报错)
3. **录 30s 新会议** → sqlite3 transcripts ORDER BY id DESC LIMIT 1 段数 ≥ 1
4. **触发一次 60+ 分钟摘要生成** (录 1 小时会议后点生成) → 30 分钟 polling 不超时

如果 1/2/3/4 任一不满足, 立刻截图回退.
