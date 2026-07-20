# 设计参考 · Design Specs

> 离线会记 v0.6.0「Wave」— 从 awesome-design-md 拉取的 design system 集合，供 agent 生成代码时直接 drop-in。

## 选定的 6 份参考（基于"录音转录 + 桌面工具"场景）

| 品牌 | 文件 | 选用理由 | 借鉴 token |
|---|---|---|---|
| **Linear.app** | `linear.app/DESIGN.md` | 暗色专业工具同频 — 几乎纯黑 canvas + 单一薰衣草蓝 + hairline 1px | `colors: 5e6ad2 / 010102 / 23252a / f7f8f8` |
| **Raycast** | `raycast/DESIGN.md` | 录音/转录本身就是命令面板化的任务 — 全站暗色 + Inter ss03 字体 + 三层 surface | `colors: 07080a / 0d0d0d / 242728 / ff5757 / 59d499 / ffc533` |
| **Claude** | `claude/DESIGN.md` | AI 工具气质 — 暖米 canvas + 编辑型 serif + 暖珊瑚 CTA | `colors: faf9f5 / cc785c / 141413`（暗色 surface 备选） |
| **Vercel** | `vercel/DESIGN.md` | 开发者向 — 白底 + Geist 字体 + Mesh Gradient | `colors: 0070f3 / ff0080 / 50e3c2` |
| **Notion** | `notion/DESIGN.md` | 数据库属性彩色块 — 三色 accent 灵感来源 | `colors: 5645d4 / dd5b00 / ff64c8 / 2a9d99 / 1aae39` |
| **Spotify** | `spotify/DESIGN.md` | 录音播放 UI — 全黑沉浸 + 单一绿色 accent + pill 几何 | `colors: 121212 / 1ed760 / f3727f / ffa42b` |

## 「离线会记」三色决策

参考 6 份综合，**本产品聚焦于"录音/转录/摘要"三状态精确分工**，不是 1 个品牌 1 个色：

| 状态 | 主色 | 含义 | Hex |
|---|---|---|---|
| 🎙 录音中 | Recording Red | Raycast / Spotify 警示色 | `#ff5757` |
| 📝 转录中 | Transcript Purple | Linear / Claude 思维色 | `#5e6ad2` |
| ✨ 摘要中 | Summary Gold | Claude amber / Notion 暖色 | `#ffc533` |

每个 page 同一时间只允 1 个 accent 主导，其他做 chip / dot / inactive。

## 文件清单

```
spec/
├── linear.app/DESIGN.md    (548 lines)
├── raycast/DESIGN.md        (625 lines)
├── claude/DESIGN.md         (589 lines)
├── vercel/DESIGN.md         (705 lines)
├── notion/DESIGN.md         (821 lines)
├── spotify/DESIGN.md        (Spotify web design — 录音类工具天然相近)
└── README.md                (本文件)
```
