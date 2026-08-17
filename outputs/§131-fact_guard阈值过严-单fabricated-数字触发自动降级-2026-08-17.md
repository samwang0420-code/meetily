# §131 fact_guard 阈值过严 — 单 fabricated 数字触发自动降级 (2026-08-17)

## 触发

用户 2026-08-17 截图反馈: 录制 CCTV 庭审视频 (走私运输毒品案公开审理, 23 分钟 / 112 段) 后生成摘要, 显示:

> ⚠️ 纪要质量复核（已自动降级）
> AI 生成的纪要包含未被原文证据支持的内容，系统已自动改为基于原文重建的安全版。请人工核对下方「确认项」。
> 以下金额在原文中未找到，可能为 AI 编造: 9.29千

用户认为摘要"生成报错"。实际生成成功了, 但自动降级到"安全版"导致只有 1 条 evidence bullet, 几乎无用。

## 根因

`fact_guard.rs::FactGuardReport::is_severe()` 旧逻辑:
```rust
pub fn is_safe(&self) -> bool { 
    self.unexpected_numbers.is_empty() 
    && self.unexpected_dates.is_empty() 
    && !self.overclaimed_decision 
}
pub fn is_severe(&self) -> bool { !self.is_safe() }
```

**任意 1 个 fabricated 数字** 即触发 severe → 整段 AI 原文被 conservative_fallback 替换 → 用户只看到 "纪要质量复核" 标题 + 1 条 evidence bullet。

实际数据:
- `meeting-67026eca` summary 包含 5 万余元等真实数字 (来源) + 编造的 9.29千 (severity 1)
- is_severe() = true → 整个 AI 摘要 (5 段) 被丢弃, 只剩 1 条 evidence bullet

`9.29千` 是 AI 生成的 fabricated number, 但其它内容 (4 段事实 + 主张 + 建议 + 遗留) 都正确。**为 1 个数字放弃全部 AI 输出** UX 极差。

## 修复

### 1. `fact_guard.rs::is_severe()` 阈值更严格

```rust
/// §131: severe 判定更严格 — 1 个无关 number 不再触发自动替换
/// 真 severe 条件:
///   1. overclaimed_decision (AI 把"提案"说成"最终决定" — 法律风险高)
///   2. 多个独立 issue (≥2 个 fabricated 数字/日期, 表示系统性失真)
/// 单个 fabricated 数字/日期 → needs_review=true (UI 横幅警告), 但保留 AI 原文供用户参考
pub fn is_severe(&self) -> bool {
    self.overclaimed_decision || self.issue_count() >= 2
}
```

### 2. `service.rs` 新增 needs_review 但非 severe 路径

```rust
// §131: needs_review 但非 severe (例如 1 个 fabricated 数字) 保留 AI 原文 + 追加警示横幅
if fact_report.needs_review() {
    warn!("Summary fact guard MINOR for meeting_id={}: ... — keeping AI summary with warning", ...);
}
```

继续走原 success path, 调用 `build_summary_result_json_with_facts(Some(&fact_report))`, 让前端 FactGuardBanner 显示黄色警告。

### 3. 前端 `FactGuardBanner.tsx` 已支持双色

- severe=true → 红色 banner + 显示 conservative_fallback 内容
- severe=false (needs_review) → 黄色 banner + 显示 AI 原文

## 行为对比

| 场景 | 旧 (severity=1 → severe) | 新 (§131) |
|---|---|---|
| 1 fabricated number | 自动降级 + 1 bullet | **保留 AI 原文 + 黄色警告** |
| 2 fabricated numbers | 自动降级 | 自动降级 (保留) |
| overclaimed_decision | 自动降级 | 自动降级 (保留) |
| 1 fabricated date | 自动降级 | **保留 + 黄色警告** |
| 干净 summary | 原文 | 原文 (无变化) |

## 测试覆盖 (3 新增 + 1 更新)

```rust
// §131: 单个 fabricated 数字不应触发 severe
#[test] fn single_fabricated_number_is_not_severe()

// §131: 2 个 fabricated 数字触发 severe (系统性失真)
#[test] fn two_fabricated_numbers_is_severe()

// §131: overclaimed_decision 单独触发 severe (法律风险高)
#[test] fn overclaimed_decision_alone_is_severe()

// 已有 chinese_real_transcript_no_false_positive 更新:
//   1 fabricated number → needs_review, NOT severe
//   ≥2 → severe
```

## §37 6 步硬闸门

- ✅ tsc --noEmit: 1 个 §18 bun:test 错误 (不动)
- ✅ next build: OK
- ✅ cargo check --lib: 0 errors (29 §18 warnings 不动)
- ✅ cargo test --lib: **340 passed / 0 failed / 3 ignored** (含 3 §131 新测 + 1 更新)
- ✅ cargo build --release: 1m42s, binary 69M **mtime 14:01**
- ✅ check_historical_fixes.py: **247/247 PASS** (+2 §131 锚点)
- ✅ sync_app_bundle.sh: §99.6 SHA 同步

## §15 GUI 验收 (用户必做)

1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'` (mtime 14:01)
3. 打开 meeting-67026eca (未命名会议 5:14) → 摘要区应仍显示 ⚠️ 警告 (黄色, 非红色) + AI 原文
4. 验证: AI 5 段 (基本事实/当事人主张/律师建议/待办事项/遗留问题) 全部可见, 黄色横幅标 "1 项需复核 · 9.29千 在原文中未找到"
5. 点击 "重新生成" → 重新跑 fact_guard, 期望同样黄色警告 (LLM 推理可能再编造 9.29千)

## 已知边界 (按 §18 不主动改)

- "两条最近录音相同" 是用户行为问题 (用户手动录了同一段 CCTV 庭审两次), 不做去重检测
- FactGuardBanner 文案不动 (前端已正确双色, 仅翻译键统一)
- conservative_fallback 内容不动 (1 bullet 在 severe 场景仍保留)
- 旧 67026eca summary_processes 记录不重写, 用户点 "重新生成" 才会跑新 fact_guard

## 关联

- §40 (8/7 法律/医疗模板深化, 引入 [证据: mm:ss] 强制引用)
- §18 (不主动改无关 bug — 1 fabricated number 不替换 AI 全文是 UX, 不是精度降低)
- §92 (决策迁移铁律)
- §56 (AGENTS.md §X 描述 ≠ 代码 commit, 这次 §131 已 git verify)
- §37 (硬闸门)

## commit

`fix(§131): fact_guard 阈值过严 — 1 个 fabricated number 保留 AI 原文 + 黄色警告, 仅 ≥2 issue 或 overclaimed 触发自动降级`
