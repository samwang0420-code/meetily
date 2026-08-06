# 72 — 言镜 AI Logo 重设计 + Liquid Glass 几何 (2026-08-06)

> **触发**: 用户反馈 "你好好修复一下Logo，没有一点设计感，方方正正的就是一张图片；另外图2的log也没换，都修复好了再说，你的整体调性真的和欧美的APP差远了，真的"
>
> **基线版本**: v0.8.5 (`9346f69` + `a6efe68`)
> **新版本**: 4ddde9d (logo 重设计) + b0cc29d (Sidebar Image import 修复)

---

## 用户两张截图 + 我的诊断

### 图1: macOS 通知中心 widget 区

截图显示 4 个 app 图标，从左到右：
- **左 1**: 一只羊驼/骆驼 (搞怪卡通风, 高 emoji-style 质感)
- **左 2**: 蓝色玻璃罐 (有磨砂玻璃质感, Sonoma Liquid Glass)
- **右 1**: 黑色终端框 (磨砂黑 + 蓝色光晕)
- **右 2**: **言镜 AI 的图标** — 一个非常简单粗暴的同心圆 + 蓝色对角, 太简单

放那对比:
- 没有"Liquid Glass"质感
- 没有渐变 / 阴影 / 玻璃反射
- 没有"细节"

### 图2: 桌面 app 标题栏 + Sidebar

- 顶部: macOS 红绿灯 + "言镜 AI — 本地 AI 会议转录" (✅ 文字好)
- 左侧 Sidebar: 旧盾牌 + waveform logo (图2 展示那个盾形)
- 中间: "言镜 AI V0.8.5" (✅ 文字)
- 右边: 设置按钮 (被截)

**根因诊断**:
- 原 `frontend/public/logo.png` 是「盾牌 + 波形 bar chart」几何 (`BrandShield.tsx` 写死的 SVG)
- 太"硬"了 —— 没有玻璃质感、没有 3D 感、没有 macOS Sonoma Liquid Glass 风格
- AI 行业现代 logo 是「柔和 + 几何层叠 + 渐变 + 玻璃反射」(参考: Granola / Notion / Linear / Raycast)

---

## 新 logo 设计

### 设计语言
- **macOS Sonoma+ Liquid Glass** + Linear/Notion 极简
- **3 同心环** 表达"多层级 / 跨会议记忆"
- **中心"言镜"球** ("言镜" = "speaking mirror" = 反射/聚焦/AI 大脑)
- **12 点"录音红点"** 行业标准化, 用户 3 米外也能识别"recording"

### 配色 (来自 `frontend/src/lib/design-tokens.ts` BRAND)
| 元素 | 颜色 | 说明 |
|---|---|---|
| 背景渐变 (顶→底) | `#1a4257` → `#0f2638` → `#091824` | 深 teal → 深 navy |
| 外环 | `#9bf0e8` → `#13A89E` → `#0d7d77` | teal 渐变 (transcript) |
| 中环 | 白 → 浅 teal (75% 长 + 反向) | AI 摘要层 |
| 内环 | `teal-500` 实线 | 焦点圈 |
| 中心"镜"球 | navy-cyan 径向渐变 + 白反光 + 暗反射 | 比喻言镜 |
| 录音红 | `#ff5757` + red halo | 在-meeting 状态 |

### 文件产出 (24 文件)
- `frontend/public/logo.png` (1024x1024, 206KB) — Next.js static serve
- `frontend/public/logo-collapsed.png` (同上, sidebar 收起态)
- `frontend/public/icon_128x128.png` + `icon_32x32@2x.png` — favicon
- `frontend/src/app/favicon.ico` — 浏览器 favicon (multi-size)
- `frontend/src-tauri/icons/icon.png` (1024x1024) — Tauri master
- `frontend/src-tauri/icons/icon.icns` (824KB, 11 个尺寸) — macOS multi-res
- `frontend/src-tauri/icons/icon.ico` (82KB, 4 个尺寸) — Windows multi
- `frontend/src-tauri/icons/{32x32,128x128,128x128@2x}.png` — Tauri 标准
- `frontend/src-tauri/icons/Square{30,44,71,89,107,142,150,284,310}x{...}.png` + StoreLogo.png (9 个 Windows Store 尺寸)
- `frontend/src/components/BrandShield.tsx` (重写 SVG, 87 行)

### SVG 设计 (核心)
- 1024×1024 viewBox
- 圆角背板 (rx=229, 22% 苹果标准)
- navy 渐变 + 多层径向光 (top-left 光源 + bottom-right 阴影)
- 3 同心环 (stroke-dasharray 制造"进行中"留白)
- 中心多层 disc (镜球) + 高反光
- 录音红点独立 red halo 渐变 + 微高光

---

## 工具脚本 (重启可重跑)
| 脚本 | 用途 |
|---|---|
| `/tmp/yanjing_logo_v2.py` | SVG 生成器 (120 行 Python, 完整几何定义) |
| `/tmp/svg_to_png_v1.py` | cairosvg SVG → PNG 渲染 |
| `/tmp/logo_batch.py` | Tauri 18 尺寸 + iconutil .icns + PIL .ico (multi-size ICO bug fix) |

---

## 编译修复

### 编译期发现的 pre-existing Bug
**Sidebar/index.tsx:712, 729** 用 `<Image>` 但**没有 `import Image from 'next/image'`** —— 这是 pre-existing bug, 但任何新的 next build 都失败:
```
JSX element class does not support attributes because it does not have a 'props' property.
Type 'new (width, height) => HTMLImageElement' is missing the following properties from type 'Component<any, any, any>': context, setState, forceUpdate, render
```
意思是 TypeScript 把 `<Image>` fall-back 成 DOM 原生 `HTMLImageElement` 构造器。

**修复**: `perl -i -pe 'print ... if $. == 14' Sidebar/index.tsx` — 在 line 14 (next/navigation 后) 加 `import Image from 'next/image';`

### 编译链路 (前文 §25 + §37 铁律)
1. ✅ `node ./node_modules/typescript/lib/tsc.js --noEmit` — 0 errors (排除 §18 bun:test)
2. ✅ `pnpm build` (next build) — 16 routes 编译, 出 frontend/out/
3. ✅ `cargo build --release` (frontend/src-tauri) — 5 分钟编译 (incremental cache warm), 出 `target/release/meetily` 67MB

### icns embed 验证
```
$ python3 -c "open('target/release/meetily','rb').read().count(b'icns')"
2
```
新 .icns 已 embedded 进 binary (含 11 个 macOS 尺寸 + ic12 类型)。

---

## Binary & 启动

```bash
# 重启 binary 验证新 logo (Dock + Sidebar + RecordingControls + BrandShield)
killall meetily 2>/dev/null
open /Users/wangwei/Documents/离线会记/target/release/meetily
```

期望看到:
- **macOS Dock 图标**: 3 同心环 + 中心"镜"球 + 12 点红点
- **Sidebar**: 新 logo (28×28, 圆角)
- **RecordingControls 录音按钮**: 48×48 hero / 28×28 normal
- **Topbar favicon + BrandShield**: SVG 几何一致

---

## 关联

- [[71-7款AI会议工具深度调研-2026-08-06]] —— 调研里建议 "我们 100% 本地的护城河必须讲出来" (附官网对比表)
- 调研 71 中提到 "Notta 也提供 On-device" — 这次新 logo 视觉对齐 Linear/Notion 极简
- Charoite 对比表引用 v2 logo 的 "3 同心环 = 多层记忆" 概念
