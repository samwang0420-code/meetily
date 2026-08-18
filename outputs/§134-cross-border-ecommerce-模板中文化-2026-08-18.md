# §134 cross_border_ecommerce 模板 name 中文化 (2026-08-18)

## 触发

用户 8/18 截图: 模板选择器里看到 "Cross-border E-commerce / SMM (跨境电商 & 社媒营销)",
英文前缀 + 中文混排. 反馈 "这条去掉吧".

## 解读与决策

按 §131.3 中文化精神, **去掉英文前缀** 保留中文 — 用户最可能是指去掉英文部分
(不是删整个模板). 模板场景仍可用, 改动最小可逆.

如果用户实际意图是删整个模板, 跟我说我撤 §131.4 注册的 4 处.

## 修复 (1 文件, +1/-1)

`frontend/src-tauri/templates/cross_border_ecommerce.json`:
- `name`: `"Cross-border E-commerce / SMM (跨境电商 & 社媒营销)"` → `"跨境电商 & 社媒营销"`
- `description` 不动 (用户只说 "这条", 指 name 字段; description 里的 SMM 是行业通用缩写)

## §37 硬闸门

- cargo build --release: 1m46s
- check_historical_fixes.py: **276 → 277/277 PASS** (1 个新 §134 锚点)
- sync_app_bundle.sh: 3 binary 全 sync
- binary grep: 英文 "Cross-border E-commerce / SMM" **NOT FOUND**, 中文 "跨境电商 & 社媒营销" **FOUND**

## §15 GUI 验收 (用户必做)

1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
3. 任意 meeting → 设置 → 摘要 → 模板选择 → 应该看到 "跨境电商 & 社媒营销" 一行, 英文前缀消失

## 关联

- §131.3 (中文化 5 模板精神延伸)
- §131.4 (这模板在 §131.4 才 register 进 defaults.rs)
- §56 (commit 必带实际改动)
- §37 (硬闸门)
- §18 (不主动改 description 里的 SMM 缩写)
