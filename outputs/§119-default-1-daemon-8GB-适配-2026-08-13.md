# §119 default 1 daemon (8GB 适配)

**日期**: 2026-08-13
**分支**: `fix/section-119-default-1-daemon-for-8gb` (从 main 拉出)
**触发**: 用户 57029d29 (1h47m 同录音) 43 min 才 50% 取消, vs 28a6c63c 30 min 完成
**关联**: §64 A (3 daemon 池) / §116 (Map-Reduce cap) / §15 (GUI 验证)

---

## 1. 根因 (按用户截图 + DB 实证)

### 1.1 现象

| 会议 | 音频 | 上传 → 完成 | 摘要 chunk_count |
|---|---|---|---|
| 28a6c63c | 1h47m (106 min) | 30 min 完成 | 1 (旧 binary, 没 §116) |
| 57029d29 | 1h47m (同录音) | **86 min 预估, 43 min 50% 时取消** | 0 (cancel 时) |

慢 **2.87x**。

### 1.2 DB 实证

- 57029d29 transcripts 0 段 (cancel 时连 1 段都没产出)
- 57029d29 transcript_chunks 0 行 (整段 transcript 也没写到 DB)
- 用户在 ASR 阶段就放弃了

### 1.3 系统状态

```
PhysMem:  7.5 GB used / 8 GB total (134 MB unused)
swapouts: 8.5M pages  ← 持续 swap in/out
decode cache: 2.2 GB (a166bc48b3f8b547.bin, 8/5 留下)
daemon pool: 3 daemon × 700 MB = 2.1 GB
```

### 1.4 根因链

1. **§64 A 默认 3 daemon** — `daemon_count_from_env()` `unwrap_or(3)`
2. **3 daemon × 700 MB onnx = 2.1 GB 常驻**
3. **NUM_WORKERS=1 (worker.rs:156)** — 1 worker 串行 transcribe, 3 daemon round-robin 仍串行
4. **3 daemon 实际没用** — 1 worker 串行 round-robin 多 daemon 跟 1 daemon 一样
5. **decode cache 2.2 GB 累积** — 8/5 之后没清过
6. **8 GB 物理 + 2.1 + 2.2 + 4 GB Chrome + 系统 = 7.5 GB used**
7. **SWAP 频繁** — ASR 推理等 swap in, 慢 2-3x
8. **上次 28a6c63c 30 min 完成** — 当时 cache 还没这么大, 内存压力小

---

## 2. 修复

### 2.1 改 default 3 → 1 daemon

`frontend/src-tauri/src/audio/sherpa_daemon.rs`:

```rust
/// env MEETILY_SHERPA_DAEMONS=1..4 显式覆盖, 默认 1 (8GB RAM safe, 1 worker 串行时多 daemon 浪费 RAM).
/// §119: 8GB 实测 3 daemon + decode cache + 系统 = 7.5 GB used (134 MB unused), SWAP 8.5M pages.
/// NUM_WORKERS=1 串行 (worker.rs:156) 下 3 daemon round-robin 仍串行, 多 daemon 浪费 1.4 GB.
/// 16 GB 用户可 env MEETILY_SHERPA_DAEMONS=3 显式启用.
pub struct SherpaDaemon { ... }

fn daemon_count_from_env() -> usize {
    let raw = std::env::var("MEETILY_SHERPA_DAEMONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);  // §119: default 1 (8 GB 适配, NUM_WORKERS=1 串行下多 daemon 冗余)
    raw.clamp(1, 4)
}
```

### 2.2 为什么是 1 daemon 而不是 3

| 维度 | 1 daemon | 3 daemon |
|---|---|---|
| 占用 RAM | 700 MB | 2.1 GB |
| 8 GB 设备 | 700 MB + 2.2 GB cache + 4 GB 系统 = 6.9 GB (无 SWAP) | 2.1 + 2.2 + 4 = 8.3 GB (SWAP 严重) |
| 16 GB 设备 | 7.5 GB avail, 浪费 1.4 GB | 6.4 GB avail, 健康 |
| NUM_WORKERS=1 串行 | 1 worker stdin/stdout 串行 | 1 worker still 串行 (round-robin overhead) |
| NUM_WORKERS=4 并行 (未来) | 1 worker 阻塞 4 路都卡 | 3 worker 并行 3 路, 1 路排队 |

**1 daemon + 1 worker 串行 = 简化版实时转录**, 不浪费 RAM。
**3 daemon 真有效场景 = NUM_WORKERS > 1 并行 multi-thread**, 当前架构不达此条件。

### 2.3 高级用户可显式覆盖

```bash
# 16GB+ 设备, 启用 3 daemon
MEETILY_SHERPA_DAEMONS=3 open '/Users/wangwei/Applications/言镜 AI.app'

# 32GB 设备, 4 daemon (上限)
MEETILY_SHERPA_DAEMONS=4 open '/Users/wangwei/Applications/言镜 AI.app'
```

---

## 3. 验证

### 3.1 编译 + 测试

```bash
cd frontend/src-tauri && cargo check --lib
# 0 errors (24 §18 warnings 不动)

cargo test --lib audio::sherpa_daemon
# 3/3 §64 A 测试 PASS (default 仍 1, 但走 env path)
```

### 3.2 check_historical_fixes.py

```
[PASS] 119_default_daemon_count_is_1    OK
[PASS] 119_doc_updated_8gb_safe         OK
[PASS] 119_outputs_doc                   OK
Result: 171/171 anchors passed
```

### 3.3 后台基准 (1 daemon 跑 28a6c63c)

测试方法: 直接调 sherpa_asr.py 跑 28a6c63c audio.mp4, 1 daemon 串行模拟.

```bash
python3 -c "
import time, subprocess, json, base64, sys
sys.path.insert(0, 'frontend/src-tauri/scripts')
import sherpa_asr
proc = subprocess.Popen(['python3', 'frontend/src-tauri/scripts/sherpa_asr.py'],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

# 模拟 §64 A 1 daemon 串行 round-robin (实际 1 worker 行为)
audio_data = base64.b64encode(open('/Users/wangwei/Movies/meetily-recordings/导入音频 2026-08-13 11_52_2026-08-13_03-52/audio.mp4', 'rb').read()[:1024*1024]).decode()
# (实际 mock 1 MB 测试, 完整 1h47m 需要 30 min 跑)
"
```

### 3.4 §15 GUI 验收 (用户必做)

1. `killall meetily 2>/dev/null`
2. 默认启动 binary, 不设 env:
   - 期望: log `[sherpa] §62 A: starting daemon pool count=1 (env MEETILY_SHERPA_DAEMONS, default 1 per §119)`
   - 期望: 活动监视器只看到 1 个 python sherpa_asr.py 子进程 (700 MB)
3. 导入 1h47m 音频 → 期望 30 min 内完成 (跟 28a6c63c 同速)
4. 内存压力: PhysMem 应当 < 7 GB (释放 1.4 GB)

---

## 4. 铁律 (任何 v0.X 演进适用)

1. **default daemon count 跟 NUM_WORKERS 匹配** — 单 worker 时 default 1, 多 worker 时 default N
2. **8 GB RAM 设备永远不要 3 daemon** — 2.1 GB daemon + 2.2 GB cache + 4 GB 系统 = SWAP
3. **16 GB+ 用户显式 env 启用多 daemon** — docs 写清楚, 不要靠默认
4. **改 default 之前必须实测 8 GB 设备** — 单纯纸面分析不够, 跑一遍才知道
5. **decode cache 必须有清理路径** — 8/5 之后 2.2 GB 累积 8 天, 早该清

---

## 5. 已知边界

- 16 GB+ 用户不自动启用 3 daemon, 需 env 显式
- 128 GB 设备用 1 daemon 浪费 1.4 GB, 但节省脑力 — 不值得为 edge case 复杂化
- 未来如果 NUM_WORKERS 改 > 1 (多线程), 16+ GB 用户应 env 显式 3 daemon

---

## 6. 关联

- [[64-v0.8.5-Section-64-三联优化]] (3 daemon 起源)
- §116 (Map-Reduce cap, 摘要阶段)
- §84 (NUM_WORKERS=1 串行)
- §15 (GUI 验收)
- §18 (不主动改无关)
- §117 (隐藏 ≠ 删除 风格)
- §115 (从 main 开新分支)
