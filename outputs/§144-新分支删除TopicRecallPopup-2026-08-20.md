# §144 新分支删除 TopicRecallPopup (2026-08-20)

## 触发
用户 8/20 截图:会议开始时弹出 "上次讨论过的话题" popup(列 3 个 topic 卡片),底部有 "知道了" / "查看全部" 按钮。

用户原话: "新开分支去掉"

## 决定
按 §37 开新分支 `codex/remove-topic-recall-popup` 删除整个 popup。

## 改动文件(4 个)

| 文件 | 改动 |
|---|---|
| `frontend/src/app/layout.tsx` | 删 `import { TopicRecallPopup }` + 删 `<TopicRecallPopup />` + 加 `§144 removed` 注释 |
| `frontend/src/components/TopicRecall/TopicRecallPopup.tsx` | 整个文件删除 |
| `frontend/src/components/TopicRecall/.gitkeep` | 新建(空目录守护) |
| `scripts/check_historical_fixes.py` | 加 2 个 §144 guard anchor |

## 保留(后续知识图谱仍用)
- 后端 `api_topic_recent` 命令(Knowledge 页面用)
- `TopicSearchModal`(`Cmd/Ctrl+K` 触发,不是 popup)
- `TopicSearchLauncher`(键盘快捷键)

## §37 硬闸门

| 项 | 结果 |
|---|---|
| tsc --noEmit | 0 errors |
| next build | 21/21 prerender OK |
| cargo check --lib | 0 errors, 0 warnings |
| cargo build --release | 5m 56s, binary 55M, sha=2501eb4e3d76 |
| check_historical_fixes.py | **436/436 PASS** (2 新 §144 anchor) |
| sync_app_bundle.sh | §108 + §99.6 + §98 + §99.3 全 OK |

## 恢复方法(代码 100% 保留)
```bash
git checkout main  # 切回 main
# 或者: 在 codex/remove-topic-recall-popup 分支 git revert 这次 commit
git revert HEAD   # 撤销 §144 commit
```

或者从 git log 取回:
```bash
git log -p HEAD~1 -- frontend/src/components/TopicRecall/TopicRecallPopup.tsx
```

## §15 GUI 验收(用户必做)
```
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
# 开始新会议 → 不再弹出 "上次讨论过的话题" popup
```

## 关联
- §141.7(隐藏会议脉络页 — 同 "华而不实" 反馈)
- §37(开新分支 SOP)
- §28(决策迁移铁律 — 必加 guard anchor)
- §104(用户偏好: 隐藏不删 / 便于恢复)
