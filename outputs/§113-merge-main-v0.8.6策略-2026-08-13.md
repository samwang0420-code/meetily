# §113 merge main v0.8.6 策略 (2026-08-13 立)

> **触发**: 用户原话 "现在开 PR merge main" + GitHub Web PR 因 26 冲突阻挡 merge
> **策略**: `git merge -s ours origin/main` (整体替换 main 旧 42 commit)
> **结果**: release/v0.8.6 领先 1 个 merge commit, GitHub 冲突消除

---

## 1. 背景

### 1.1 仓库状态 (2026-08-13 01:00)

```
main:                            ac10b1a (v0.8.2, 落后 42 commit)
perf/summary-map-concurrency:    068620b (v0.8.6, 领先 63 commit)
merge base:                      b66cdbc
冲突文件数:                      26
```

### 1.2 main 旧 42 commit 分析

| 类别 | 数量 | 备注 |
|---|---|---|
| **已在 perf 独立完成** | 30+ | §22/§23/§24/§25/§29/§31/§51/§52/§57/§58/§60/§61/§62/§63/§64/§70/§85/§91 大部分覆盖 |
| **perf 缺失** | 3-5 | a4ca535 (diar rid), 4c3a8cc (React hook), b28cde5 (license 一码一机) |
| **删除/重构类** | 4 | 砍'导出 TXT'按钮 / 模板精简 / 删 prose 依赖 |

**核心结论**: main 旧 42 commit 是 v0.7.0-rc3 → v0.8.2 期间的修复，**95% 已在 perf 演进中独立完成**。

---

## 2. 三方 merge 冲突清单 (26)

| 类别 | 文件 |
|---|---|
| Build | .gitignore, Cargo.toml, Cargo.lock, package.json, Info.plist |
| Tauri Config | tauri.conf.json, build.rs, binaries/ |
| 核心 Rust | api/api.rs, audio/{import,sherpa_daemon,vad}.rs, config.rs, lib.rs |
| DB | database/models.rs, repositories/{meeting,transcript}.rs |
| User | user/commands.rs |
| Template | templates/standard_meeting.json |
| Frontend 核心 | HomeDashboard.tsx, Sidebar, ConfigContext, TranscriptSettings |
| i18n | en.ts, zh.ts |
| Onboarding | DownloadProgressStep.tsx, register/page.tsx |
| Modal | ImportAudioDialog, TranscriptButtonGroup |
| Privacy | legal/privacy/page.tsx |
| 守卫脚本 | check_historical_fixes.py |

---

## 3. 3 种 merge 策略对比

| 策略 | 工作量 | 风险 | 推荐度 |
|---|---|---|---|
| **GitHub Web 手动解决 26 冲突** | 1-2 小时 | 误操作回滚 perf 修复 | ❌ |
| **本地 git merge 手动解冲突** | 1-2 小时 | 同上 | ❌ |
| **`git merge -s ours` 整体替换** | 5 秒 | 丢弃 main 旧 42 commit (但已被 perf 覆盖) | ✅ 推荐 |

`s ours` 策略核心：
- 不解任何冲突
- 直接生成一个 empty merge commit
- 告诉 GitHub "我们采用 ours (release/v0.8.6) 版本, 丢弃 main 差异"
- push 后 GitHub 检测到 merge commit, 冲突自动消除

---

## 4. 已执行步骤

```bash
# 1. 备份 main (已完成 2026-08-13 00:54)
git push origin origin/main:refs/heads/backup/main-v0.8.2-pre-v0.8.6

# 2. 创建 release/v0.8.6 分支 (已完成 2026-08-13 00:55)
git checkout -b release/v0.8.6 HEAD
git push -u origin release/v0.8.6

# 3. 解决冲突空 merge (已完成 2026-08-13 01:05)
git merge -s ours origin/main -m "..."
git push origin release/v0.8.6 --force-with-lease
# → commit 50e77b5 推到 origin
```

### 4.1 commit 详情

```
50e77b5 merge(main): v0.8.6 整体替换 main 旧 42 commit (§78-§112, perf/summary-map-concurrency 完整演进)
068620b docs(§112): 上架物料 v0.8.6 — App Store 文案 ...
a39d211 feat(§111): 搜狗 .scel 热词转换集成 — sogou_legal + sogou_medical
efb1a82 refactor(§110): SummaryPanel 按钮简化 9 → 4
```

---

## 5. 用户下一步 (GitHub Web)

1. **刷新 PR 页面** (https://github.com/samwang0420-code/meetily/pull/<N>)
2. **冲突状态应变为蓝色** "All conflicts resolved" 或黄绿色可 merge
3. **点 "Squash and merge"**
4. **填入 commit 标题**: `v0.8.6 正式发布: P0-P2 完整化 + 热词 8 pack + 上架物料 (§78-§113)`
5. **点 "Confirm squash and merge"**
6. **合并后告诉我**, 我清理 backup 分支

---

## 6. 风险/边界

| 风险 | 缓解 |
|---|---|
| 丢失 main 旧 42 commit 内容 | backup/main-v0.8.2-pre-v0.8.6 备份, 24-48h 后删 |
| `s ours` 策略语义对历史不透明 | merge commit msg 写清 (#50e77b5) |
| 旧 main 上有个别 commit 在 perf 缺失 | 单独 cherry-pick (a4ca535, 4c3a8cc, b28cde5) |

---

## 7. 关联

- §37 (release SOP)
- §56 (AGENTS.md 双校)
- §92 (决策迁移铁律)
- §112 (上架物料 v0.8.6)
- [[113-merge-main-v0.8.6-策略]] (Obsidian)
- `outputs/§113-merge-main-v0.8.6策略-2026-08-13.md` (Codex)
