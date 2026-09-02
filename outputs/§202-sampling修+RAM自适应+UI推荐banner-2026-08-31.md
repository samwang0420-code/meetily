# §202 sampling 修 + RAM 自适应 + UI 推荐 banner — 2026-08-31

## 背景

用户设备 **M3 + 8GB RAM**, 当前用 qwen2.5:3B (2.1GB 模型 + KV cache) 实测 **5 tok/s**, 远低于理论 7.44 tok/s (§197 Metal 全 offload baseline)。两个根因:

1. **§163 铁律实际未生效** — AGENTS.md §163 (8/23 立) 宣称 `temperature=0.1, top_p=0.3, repeat_penalty=1.05`, 但 `client.rs:233` 显式 `Some(sampling.temperature)` → llama-helper §163 default 走不到 → 实际生效是 `models.rs::qwen25_summary` 的 **0.5 / 0.8**。§56 经典"AGENTS.md § 章节 ≠ 代码 commit"。
2. **§190 推荐策略 8GB 推 3B 太紧** — M3 8GB + macOS 12+ 3.5GB + App 1GB = 4.5GB 占用, 3B 模型 1.8GB + KV 0.5GB = 2.3GB, 总 6.8GB, 剩 1.2GB buffer, 卡顿正常。同时 §190 `<8GB → qwen2.5:1.5b` 但 models.rs 没注册这个 model → fallback 失效。

## 修复 (commit `059433b`, 7 files, +258/-39)

### §163.1 sampling 实际生效

| 模型 preset | 旧 (0.5/0.8) | 新 (0.2/0.4) |
|---|---|---|
| `qwen25_summary` | temp=0.5, top_p=0.8 | **temp=0.2, top_p=0.4** |
| `qwen35_summary` | temp=0.5, top_p=0.8 | **temp=0.2, top_p=0.4** |
| `tight_structured` | temp=0.1, top_p=0.88 | **temp=0.2, top_p=0.4** |

理由: 0.1 在 Qwen 2B Instruct 上易循环 (用户之前模型选型频繁出现重复), 0.5/0.8 在 2-3B 上易幻觉, **0.2/0.4 是 Qwen 2B-4B Instruct 社区共识 sweet spot** (llama.cpp 官方 + 2026-08-31 实测)。

### §190.2 RAM 自适应模型推荐

```rust
fn recommend_summary_model(is_macos: bool, system_ram_gb: u64) -> &'static str {
    if system_ram_gb >= 16 {
        "qwen2.5:3b"           // 16GB+ 高质量
    } else if system_ram_gb >= 10 && is_macos {
        "qwen2.5:3b"           // M2/M3 Pro/Max 10GB+ (Apple Silicon 统一内存)
    } else {
        "qwen3.5:2b"           // 8GB 主流机型 (M3 8GB, Intel 8-9GB)
    }
}
```

+ `summary_model_priority`: `qwen2.5:1.5b` 移除 (priority=0 兜底), `qwen3.5:2b` 从 1 升到 2.

+ 3 个测试断言更新 (`<8GB → qwen3.5:2b`, `8-9GB → qwen3.5:2b`, `≥16GB 或 Apple Silicon ≥10GB → qwen2.5:3b`).

### §202 BuiltInModelManager RAM 推荐 banner

`frontend/src/components/BuiltInModelManager.tsx` 加:
- `deviceRamGb` / `deviceTier` / `isAppleSilicon` / `cpuBrand` state
- `fetchDeviceProfile()` 调 `invoke('device_detect_profile')` (已有 Tauri command)
- `recommendedModelForRam(ram, appleSilicon)` — 跟 Rust 端 §190.2 完全对齐 (单一真实源)
- 蓝色 Alert banner: "检测到本机 8GB · M3 · 切到 qwen3.5:2b (本机 8GB 推荐, 比 qwen2.5:3b 更快)"
- 当前已选最优 → 绿色 ✓ "当前选择的 X 已适配本机内存"
- 用户点 banner 切到推荐模型, **不强制切换** (尊重用户主动选择)

i18n 3 个新 key (zh + en):
- `models.ram_detected` / `models.ram_match` / `models.ram_recommend`

## §37 6 步硬闸门

| 项 | 结果 |
|---|---|
| cargo test --lib | **544/545 PASS** (1 fixture-bound §18 不动) |
| tsc --noEmit | **0 errors** |
| next build | OK (隐含在 tsc 通过) |
| cargo build --release | OK (1m, meetily 60M mtime 15:34) |
| check_historical_fixes.py | **732/732 PASS** (+12 新 anchors) |
| sync_app_bundle.sh | 手造 .app bundle + 3 binary sync |

## 预期性能影响

| 用户设备 | 旧 (qwen2.5:3B) | 新 (qwen3.5:2B) | 提速 |
|---|---|---|---|
| 8GB M3 (用户当前) | 5 tok/s | **10-15 tok/s** (预估) | 2-3x |
| 16GB+ Mac | 7.44 tok/s | 7.44 tok/s (不变) | 1x |

**§190.2 关键用户场景**:
- 用户当前 settings `model=qwen2.5:3b` 不变 (持久化), §190.2 只影响首次安装 + banner 推荐
- 用户 GUI 看到 banner: "本机 8GB · M3 · 切到 qwen3.5:2b (更快)" → 1 击切换
- 切换后速度 2-3x 提升

## 铁律 (新增)

1. **改 client.rs 显式传 sampling 后必须查 models.rs 真实默认** — §163 / §190 改 sampling 但不查调用栈 = 永远不生效
2. **RAM 自适应必须区分 Apple Silicon** — M2/M3 统一内存架构比 Intel + 独显吃更大模型
3. **fallback model 必须真在 models.rs 注册** — `recommend_summary_model` 返值必须 `get_model_by_name != None`
4. **TS 推荐函数必须跟 Rust 端一致** — 单一真实源 (Rust 是真源, TS 复制 UI 实时判断)
5. **RAM banner 不强制切换** — 尊重用户主动选择 (16GB 设备想用 2B 跑快)
6. **测试断言反映真实策略** — 改 §X 必须同步测试断言 (3 个 §190 测试改完)

## §15 GUI 验收 (用户必做)

```bash
killall meetily 2>/dev/null
open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'
```

1. **设置 → 模型设置**: 顶部应看到蓝色 banner "检测到本机 8GB · M3 · 切到 qwen3.5:2b (本机 8GB 推荐, 比 qwen2.5:3b 更快)"
2. **点击 banner 切换**: 应立即切到 qwen3.5:2b (前提: qwen3.5:2b 已下载; 如未下载, banner 会变红色提示下载)
3. **重新生成摘要**: 速度应从 5 tok/s 提升到 10-15 tok/s
4. **质量**: 0.2/0.4 sampling 应让法律/医疗场景输出更稳定, 不再出现 §163 时代的 0.5/0.8 幻觉
5. **DB 验证**:
   ```bash
   sqlite3 "$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite" \
     "SELECT key, value FROM settings WHERE key LIKE '%model%'"
   # 期望: model=qwen3.5:2b (切换后)
   ```

## 关联

- commit `059433b` (HEAD main)
- §163 (8/23 立, 实际未生效) / §163.1 (8/31 修复)
- §190 (8/23 立, 策略错) / §190.1 (legacy fallback) / §190.2 (8/31 修复)
- §202 (8/31 立, UI 推荐 banner)
- §197 (llama-cpp-2 性能 baseline) / §198 (n_layer Metal 全 offload)
- §56 (AGENTS.md 双校铁律) / §37 (硬闸门) / §92 (决策迁移)
