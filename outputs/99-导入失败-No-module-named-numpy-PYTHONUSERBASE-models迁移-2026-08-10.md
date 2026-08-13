# §99 — 导入失败 "No module named 'numpy'" 修复 (2026-08-10)

**commit**: `6907799` (perf/summary-map-concurrency)
**binary**: `/Users/wangwei/Documents/离线会记/target/release/meetily` 69M mtime 11:40

## 用户截图反馈

```
Sherpa transcription failed on segment 0: sherpa slot 0 error: No module named 'numpy'
```

## 根因 — 两个叠加 bug

### Bug 1: §96 PYTHONUSERBASE=$HOME hack 覆盖 numpy 默认 user-base

**§96 commit ad3b9e2** 在 spawn Python 子进程时加:
```rust
if let Some(home) = std::env::var_os("HOME") {
    cmd.env("PYTHONUSERBASE", home);
}
```

**实证 (修复前)**:
- 不带 PYTHONUSERBASE: homebrew python 默认 user-base = `~/Library/Python/3.14/lib/python` (numpy/sherpa_onnx 装在那) → import OK
- 带 PYTHONUSERBASE=/Users/wangwei: user-base 变成 `/Users/wangwei`, PEP 370 错误映射到 `~/lib/python3.14/site-packages` → import FAIL

**问题**: 探测时 (`python_path()`) import 成功, 但 spawn 时显式设了 PYTHONUSERBASE, env 不一致, 导致 spawn 找不到 numpy。

### Bug 2: §97 迁移函数漏复制 models/

**§97 commit 276906e** 的 `migrate_legacy_app_data` 只复制 3 个 sqlite 文件, 漏了 `models/` 目录:
```rust
let files_to_copy = ["meeting_minutes.sqlite", "meeting_minutes.sqlite-shm", "meeting_minutes.sqlite-wal"];
```

注释假设"用户机器新目录已有 4G decode_cache + models", 但实际:
- 旧 `cn.lixianhuiji.app/models/sherpa/` 有 `funasr-nano-int8` + `paraformer-zh-int8` (~1.2GB)
- 新 `tech.yanjingai.app/models/sherpa/` **不存在**

`sherpa_asr.py` 启动后扫 `MODELS_ROOT = ~/Library/Application Support/tech.yanjingai.app/models/sherpa`:
```
[sherpa_asr] daemon started, models_root=.../tech.yanjingai.app/models/sherpa
[sherpa_asr] discovered 0 model packs: []
```

→ 0 模型可用, 导入转录 0 段识别。

## 修复 (commit 6907799)

### 1. sherpa_daemon.rs (Bug 1)
- 删 `cmd.env("PYTHONUSERBASE", home)` 三行
- 注释改对: §99 不设 PYTHONUSERBASE, 让 Python 用默认 user-base
- 保留 `cmd.env("PYTHONUNBUFFERED", "1")` (stderr 不缓冲)

### 2. sherpa_daemon.rs §99 spawn 验证单测
```rust
#[test]
fn section_99_spawned_python_can_import_sherpa_onnx() {
    // 真 spawn Python 子进程 (与生产代码 env 完全一致, 无 PYTHONUSERBASE),
    // 发 {"action":"list"} 让 daemon 启动时 import sherpa_onnx/numpy/soundfile,
    // 验证 ok=true. 这是 "No module named 'numpy'" 的唯一可靠防线.
    ...
}
```
**测试结果**: PASS, models=0 (符合预期 — Bug 2 修复前新目录没模型)

### 3. lib.rs migrate_legacy_app_data (Bug 2)
加 `copy_dir_recursive` helper + 在迁移函数里:
```rust
let src_models = legacy_dir.join("models");
let dst_models = new_dir.join("models");
let dst_sherpa = dst_models.join("sherpa");
let need_copy = !dst_sherpa.is_dir()
    || std::fs::read_dir(&dst_sherpa).map(|mut it| it.next().is_none()).unwrap_or(true);
if need_copy {
    copy_dir_recursive(&src_models, &dst_models, &mut models_copied)?;
}
```
- 新目录 `models/sherpa/` 不存在 OR 为空 → 递归复制整个 `models/` 树
- 已存在 → 跳过 (用户已有不覆盖)

### 4. check_historical_fixes.py 6 个 §99 anchor
- `99_no_pythonuserbase_hack`: 不存在 `cmd.env("PYTHONUSERBASE"`
- `99_spawn_unbuffered_only`: `PYTHONUNBUFFERED` 还在
- `99_spawn_test_exists`: §99 测试存在
- `99_migrate_models_recursive`: `fn copy_dir_recursive` 存在
- `99_migrate_calls_models`: 迁移函数调用 copy_dir_recursive
- `99_migrate_log_models_count`: log 含 `models_files_copided=`

**guard 结果**: **107/107 PASS** (从 101/101 → 107/107)

## §37 硬闸门

- ✅ cargo check --lib: 0 errors (27 §18 warnings 不动)
- ✅ cargo test --lib: **330 passed / 0 failed / 3 ignored** (含 §99 spawn test)
- ✅ tsc --noEmit: 1 个 §18 .next/types noise (不动)
- ✅ next build: OK
- ✅ cargo build --release: **1m25s**, binary **69M mtime 11:40**
- ✅ bash scripts/sync_app_bundle.sh: **OK §98 codesign identifier=tech.yanjingai.app**
- ✅ python3 scripts/check_historical_fixes.py: **107/107 PASS**
- ✅ python3 scripts/audit_codebase.py --strict: 0 errors / 1 warn / 60 info

## §15 GUI 验收 (用户必做)

1. `killall meetily 2>/dev/null`
2. `open '/Users/wangwei/Documents/离线会记/target/release/言镜 AI.app'`
3. 首次启动: §99 迁移自动跑 (best-effort, log: `§99 migrate_legacy_app_data: copied_db=... skipped_db=... models_files_copied=N`)
4. 导入音频 → 期望: 转录正常, ~5-15 分钟完成 (取决于 CPU)
5. DB 验证:
   ```bash
   sqlite3 "$HOME/Library/Application Support/tech.yanjingai.app/meeting_minutes.sqlite" \
     "SELECT COUNT(t.id), SUM(LENGTH(t.transcript)) FROM transcripts t ORDER BY id DESC LIMIT 1"
   # 期望: 段数 >= 1, 字符 >= 10
   ```

## 铁律 (§99 立)

1. **Python 探测和 spawn 必须用相同 env** — 探测 import OK 不等于 spawn import OK (env 不一致会爆)
2. **PYTHONUSERBASE 永远不要显式设 `$HOME`** — homebrew Python 默认 user-base 已经是 `~/Library/Python/3.14/lib/python`, 显式覆盖反而破坏 PEP 370 路径映射
3. **§97 迁移函数必须 COPY 完整的用户数据** — 包括 db + decode_cache + **models**, 不能只复制 db (迁移半成品 = 用户首次启动导入失败)
4. **新代码改动必须加 guard anchor** — 不加 anchor = 下次重构又会被覆盖 (见 §56 §92 §94 历史教训)

## 回退方案 (§37 SOP)

```bash
git reset --hard 248b2b2  # §98 baseline
cd frontend && npx next build
cd src-tauri && cargo build --release
bash scripts/sync_app_bundle.sh
```

## 关联

- §96 commit ad3b9e2 (PYTHONUSERBASE hack 引入 bug)
- §97 commit 276906e (迁移漏 models/)
- §92 (§X 章节 ≠ 代码 commit)
- §94 (全面代码审计 + 守卫脚本)
- §15 (GUI 验收强制)
- §37 (硬闸门)
