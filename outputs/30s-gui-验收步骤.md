# 30 秒 GUI 验收脚本 (v0.6.10+)

> 适用场景: 我改了底层 ASR / IndexedDB / React 关键组件后让你验证.
> 注意: 不要用 `cargo run` / `npm run dev` 启 Tauri, Tauri macOS app 在 CLI shell
> 启动会被 launchd 当 orphan silent abort, panic 丢失 (AGENTS §15).

## 你需要做的 3 步 (2 分钟)

### 1. 重启 app
- 完全退出离线会记 (Cmd+Q)
- 重新双击图标启动
- 等待左下角显示 `● offline` + 版本号 v0.6.10

### 2. 录制 30 秒
- 进工作台
- 点麦克风 → "开始录音"
- 用 30 秒法律话术:

> 本案原告主张被告违反合同约定, 要求解除合同并支付违约金。
> 被告提出管辖权异议, 认为案件应当提交北京仲裁委员会处理。
> 法院审查后认为, 仲裁条款合法有效, 双方应先履行仲裁程序。
> 如果任何一方对仲裁裁决不服, 可以依法申请撤销, 但不能直接向中级人民法院提起上诉。

- 点停止, 等转写完成 (进度条走完)

### 3. 报告截图给我
需要以下 4 张 (粘到下一个对话):

| 截图编号 | 内容 | 期望特征 |
|---|---|---|
| 截图 1 | 工作台首页 | 不再有 React #321 红框 / "Failed to save meeting" 错误框 |
| 截图 2 | 新会议详情页 - 转写区 | 左侧至少 4 段带时间戳的文本,关键术语("原告""被告""违约金""仲裁条款""中级人民法院")未被识别成奇怪的同音词 |
| 截图 3 | 详情页 - 生成摘要后 | 摘要显示出来 (不再卡死), 事实校验栏对未提及项正确标"未提及" |
| 截图 4 | 法律诉讼模板渲染 | 摘要字段含"案由 / 当事人 / 待核事实实 / 风险"等法律垂直字段,不出现跨境内容残留 |

## 我会做的 1 步

你发完 4 张截图后, 我跑:

```bash
sqlite3 "/Users/wangwei/Library/Application Support/cn.lixianhuiji.app/meeting_minutes.sqlite" \
  "SELECT id, substr(text, 1, 30), timestamp FROM transcripts ORDER BY rowid DESC LIMIT 5"
```

确认 ≥ 4 段文本落库, 不是 0 段 (v0.6.12 毁灭性 bug 的前身).

## 如果还是崩

工作台弹 React #321 → 我已经有 CardBoundary 隔离, 不会全屏红框;
请打开浏览器 console (Cmd+Opt+I) → 把 `[CardBoundary]` 开头的红色错误整段 copy 给我.

