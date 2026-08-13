#!/usr/bin/env python3
"""
meetily codebase audit (AGENTS.md §94 §6.1)
扫描代码 vs 决策文档一致性, 找出"代码漏"问题.

检查项 (§94 全面审计):
  1. 死代码 / backup / orig 文件 (10+ 个已知 + 任何 *_old.* / *.backup / *.orig)
  2. 孤儿模块 (lib.rs pub mod 但 src/ 下 0 引用, audio_v2 是例子)
  3. 悬空 Tauri command (前端 invoke 但后端 invoke_handler 没注册)
  4. 孤儿 Tauri command (后端注册前端没调)
  5. 版本号一致性 (tauri.conf / package.json / Cargo.toml / .app Info.plist 4 处)
  6. identifier 一致性 (tauri.conf vs .app CFBundleIdentifier)
  7. v0.8.5 残留 (frontend i18n + 注释关键位置)
  8. git tracked backup/orig 文件

用法:
  python3 scripts/audit_codebase.py            # 默认扫描 + 报告
  python3 scripts/audit_codebase.py --strict   # 任意 fail exit 1 (CI gate)
  python3 scripts/audit_codebase.py --json     # JSON 输出

§37 SOP: 每次 release 前必跑.
"""
from __future__ import annotations
import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC_TAURI = REPO / "frontend" / "src-tauri"
FRONTEND_SRC = REPO / "frontend" / "src"
LIB_RS = SRC_TAURI / "src" / "lib.rs"

# 当前版本 (期望所有 version 字段对齐)
EXPECTED_VERSION = "0.8.6"
# 期望 identifier (P1 跟进, 当前还是 cn.lixianhuiji.app)
EXPECTED_IDENTIFIER = "cn.lixianhuiji.app"

# 类目
class Finding:
    def __init__(self, severity: str, category: str, message: str, file: str = "", line: int = 0):
        self.severity = severity  # "error" / "warn" / "info"
        self.category = category
        self.message = message
        self.file = file
        self.line = line

    def to_dict(self):
        return {
            "severity": self.severity,
            "category": self.category,
            "message": self.message,
            "file": self.file,
            "line": self.line,
        }


def shell(cmd: str, cwd: Path = None) -> str:
    """Run shell command, return stdout."""
    try:
        r = subprocess.run(
            cmd, shell=True, cwd=cwd or REPO,
            capture_output=True, text=True, timeout=30,
        )
        return r.stdout
    except subprocess.TimeoutExpired:
        return ""


def check_backup_files() -> list[Finding]:
    """1.1 backup / orig / _old / lib_old_complex 死代码"""
    findings = []
    patterns = [
        ("**/*.backup", ".backup"),
        ("**/*.orig", ".orig"),
        ("**/*.bak", ".bak"),
        ("**/*_old.rs", "_old.rs"),
        ("**/*-old.rs", "-old.rs (e.g. core-old.rs)"),
        ("**/lib_old_*.rs", "lib_old_*.rs"),
        ("frontend/src-tauri/src/lib_old_complex.rs", "lib_old_complex.rs (2437 行, 老的 lib.rs 备份)"),
    ]
    skip_dirs = {"node_modules", "target", ".next", ".git", "work/sources"}
    for pattern, label in patterns:
        for p in REPO.glob(pattern):
            rel = p.relative_to(REPO)
            if any(s in str(rel) for s in skip_dirs):
                continue
            findings.append(Finding(
                "error", "dead_code",
                f"{label}: {rel}",
                str(rel),
            ))
    return findings


def check_orphan_modules() -> list[Finding]:
    """1.2 孤儿模块: lib.rs pub mod xxx 但 src/ 下 0 引用.
    当前 audio_v2 是例子. 算法: 找 audio_v2/ 类目录, 验证不在 lib.rs mod 列表.
    """
    findings = []
    # 找 src-tauri/src 下子目录里有 pub mod 但不在 lib.rs mod 声明的模块
    if not LIB_RS.exists():
        return findings
    lib_content = LIB_RS.read_text(encoding="utf-8")
    for sub in (SRC_TAURI / "src").iterdir():
        if not sub.is_dir():
            continue
        if sub.name in {"audio", "audio_v2"} and (sub / "lib.rs").exists():
            # check if lib.rs has "mod audio_v2" or "pub mod audio_v2"
            if not re.search(rf"^\s*(pub\s+)?mod\s+{sub.name}\s*;", lib_content, re.M):
                # 找 lib.rs 内的子目录 (audio/ 里有 mod.rs)
                mod_in_audio = re.search(rf"^\s*(pub\s+)?mod\s+{sub.name}\s*;", lib_content, re.M)
                # 还要检查 audio/mod.rs (sub-mod)
                if not mod_in_audio and (sub / "mod.rs").exists():
                    sub_mod_content = (sub / "mod.rs").read_text(encoding="utf-8")
                    if not re.search(rf"^\s*(pub\s+)?mod\s+{sub.name}\s*;", lib_content, re.M):
                        findings.append(Finding(
                            "error", "orphan_module",
                            f"目录 {sub.relative_to(REPO)}/ 不在 lib.rs 注册 (audio_v2 例子)",
                            str(sub.relative_to(REPO)),
                        ))
    return findings


def check_invoke_commands() -> tuple[list[Finding], list[Finding], int, int]:
    """4.1 悬空 + 4.2 孤儿 Tauri commands.
    后端 invoke_handler 注册: 找 lib.rs 块.
    前端 invoke('cmd') 调用: 找 frontend/src 下 invoke 调用.
    """
    findings_dangling = []
    findings_orphan = []
    if not LIB_RS.exists():
        return findings_dangling, findings_orphan, 0, 0

    # 1. 后端注册 commands
    lib_content = LIB_RS.read_text(encoding="utf-8")
    m = re.search(r"generate_handler!\[\s*\n(.*?)\n\s*\]", lib_content, re.DOTALL)
    if not m:
        findings_dangling.append(Finding("error", "audit", "未找到 invoke_handler 块"))
        return findings_dangling, findings_orphan, 0, 0
    block = m.group(1)
    backend_cmds = set()
    for line in block.splitlines():
        # 单行命令: `xxx,` 或 `xxx` 或 `module::cmd,`
        line = line.strip().rstrip(",").strip()
        if not line or line.startswith("//"):
            continue
        if "(" in line or ")" in line:  # 排除宏
            continue
        # 取最后一段作为 command 名
        name = line.split("::")[-1]
        backend_cmds.add(name)

    # 2. 前端 invoke 调用 (兼容 invoke('x') / invokeTauri('x') / invoke<X>('x'))
    frontend_cmds = set()
    for ts in FRONTEND_SRC.rglob("*.ts*"):
        try:
            content = ts.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        # invoke('x') / invokeTauri('x') / invoke<X>('x')
        for match in re.finditer(
            r"""(?:invoke|invokeTauri)\s*(?:<[^>]*>)?\s*\(\s*['"]([a-z_][a-z_0-9]*)['"]""",
            content,
        ):
            frontend_cmds.add(match.group(1))

    # 3. 悬空
    for cmd in sorted(frontend_cmds - backend_cmds):
        findings_dangling.append(Finding(
            "warn", "dangling_command",
            f"前端 invoke('{cmd}') 但后端 invoke_handler 未注册 (用户调用会失败)",
        ))
    # 4. 孤儿
    for cmd in sorted(backend_cmds - frontend_cmds):
        # admin/* 不算 (admin 页面单独)
        if cmd.startswith("admin_") or cmd == "is_analytics_session_active":
            continue
        findings_orphan.append(Finding(
            "info", "orphan_command",
            f"后端注册 {cmd}() 但前端无 invoke 调用 (可能 dead command)",
        ))

    return findings_dangling, findings_orphan, len(backend_cmds), len(frontend_cmds)


def check_version_consistency() -> list[Finding]:
    """2.1 版本号一致性: tauri.conf / package.json / Cargo.toml / .app Info.plist 4 处"""
    findings = []
    versions = {}

    # tauri.conf.json
    conf = SRC_TAURI / "tauri.conf.json"
    if conf.exists():
        m = re.search(r'"version":\s*"([^"]+)"', conf.read_text(encoding="utf-8"))
        if m:
            versions["tauri.conf.json"] = m.group(1)

    # package.json
    pkg = REPO / "frontend" / "package.json"
    if pkg.exists():
        m = re.search(r'"version":\s*"([^"]+)"', pkg.read_text(encoding="utf-8"))
        if m:
            versions["package.json"] = m.group(1)

    # src-tauri/Cargo.toml
    cargo = SRC_TAURI / "Cargo.toml"
    if cargo.exists():
        m = re.search(r'^version\s*=\s*"([^"]+)"', cargo.read_text(encoding="utf-8"), re.M)
        if m:
            versions["src-tauri/Cargo.toml"] = m.group(1)

    # .app bundle Info.plist (如果存在)
    app_plist = REPO / "target/release/言镜 AI.app/Contents/Info.plist"
    if app_plist.exists():
        m = re.search(r"<key>CFBundleShortVersionString</key>\s*<string>([^<]+)</string>",
                      app_plist.read_text(encoding="utf-8"))
        if m:
            versions[".app Info.plist"] = m.group(1)

    # check 一致性
    if len(set(versions.values())) > 1:
        findings.append(Finding(
            "error", "version_mismatch",
            f"版本号不一致: {versions} (期望统一)",
        ))
    elif versions and list(versions.values())[0] != EXPECTED_VERSION:
        findings.append(Finding(
            "warn", "version_drift",
            f"版本号 {list(versions.values())[0]} != 期望 {EXPECTED_VERSION}: {versions}",
        ))

    return findings


def check_identifier_consistency() -> list[Finding]:
    """2.2 identifier 一致性"""
    findings = []
    conf = SRC_TAURI / "tauri.conf.json"
    if not conf.exists():
        return findings
    m = re.search(r'"identifier":\s*"([^"]+)"', conf.read_text(encoding="utf-8"))
    if m and m.group(1) != EXPECTED_IDENTIFIER:
        findings.append(Finding(
            "warn", "identifier_drift",
            f"identifier '{m.group(1)}' != 期望 '{EXPECTED_IDENTIFIER}' (P1 跟进)",
        ))
    return findings


def check_v085_residue() -> list[Finding]:
    """3. v0.8.5 残留: frontend i18n 关键位置 (用户可见) + 注释 (可不动)"""
    findings = []
    # 用户可见 (i18n key)
    user_visible_patterns = [
        ("frontend/src/i18n/locales/en.ts", r"version_label:\s*['\"]v0\.8\.5"),
        ("frontend/src/i18n/locales/en.ts", r"footer_copyright:\s*['\"][^'\"]*v0\.8\.5"),
        ("frontend/src/i18n/locales/zh.ts", r"version_label:\s*['\"]v0\.8\.5"),
        ("frontend/src/i18n/locales/zh.ts", r"footer_copyright:\s*['\"][^'\"]*v0\.8\.5"),
        ("frontend/src/i18n/locales/en.ts", r"beta_title:\s*['\"][^'\"]*v0\.8\.5"),
        ("frontend/src/i18n/locales/zh.ts", r"beta_title:\s*['\"][^'\"]*v0\.8\.5"),
    ]
    for rel, pat in user_visible_patterns:
        p = REPO / rel
        if p.exists() and re.search(pat, p.read_text(encoding="utf-8")):
            findings.append(Finding(
                "error", "v085_residue",
                f"用户可见的 v0.8.5 残留: {rel} pattern={pat}",
                rel,
            ))
    return findings


def check_git_tracked_backup() -> list[Finding]:
    """8. git tracked backup/orig 文件 (任何 .backup/.orig/_old 不能进 git)"""
    findings = []
    out = shell("git ls-files")
    for line in out.splitlines():
        if re.search(r"\.(backup|orig|bak|old|complex_old)$|_old\.|lib_old_|core-old", line):
            findings.append(Finding(
                "error", "git_tracked_backup",
                f"git tracked: {line}",
                line,
            ))
        elif "/audio_v2/" in line and not line.endswith("/"):
            findings.append(Finding(
                "warn", "git_tracked_orphan_module",
                f"audio_v2/ 在 git: {line}",
                line,
            ))
    return findings


def check_import_whisper_fallback() -> list[Finding]:
    """§95 fix: import.rs §58/§60 决策 — 永不 fallback whisper.

    §58/§60 决策要求 import 走 sherpa_funasr_nano / sherpa_paraformer / parakeet, 永不 fallback Whisper.
    但 §95 之前 import.rs:339 'use_parakeet = provider.as_deref() == Some("parakeet")' 仍隐式 fallback Whisper.
    检测: import.rs run_import 块不能调用 whisper_engine.transcribe_audio_with_confidence (已修复, §95 加 use_sherpa 分支).
    """
    findings = []
    p = REPO / "frontend/src-tauri/src/audio/import.rs"
    if not p.exists():
        return findings
    text = p.read_text(encoding="utf-8")
    # 检查: import.rs 不应再有 transcribe_audio_with_confidence 调用 (Whisper fallback)
    if "transcribe_audio_with_confidence" in text:
        findings.append(Finding(
            "error", "import_whisper_fallback",
            "import.rs 仍调用 whisper_engine.transcribe_audio_with_confidence (§60 决策: 永不 fallback Whisper)",
            "frontend/src-tauri/src/audio/import.rs",
        ))
    # 检查: import.rs 应有 sherpa 分支
    if "use_sherpa" not in text:
        findings.append(Finding(
            "error", "import_whisper_fallback",
            "import.rs 缺 use_sherpa 分支 (§95 fix 缺失, import 仍只能 parakeet/whisper)",
            "frontend/src-tauri/src/audio/import.rs",
        ))
    return findings


def check_hardcoded_model_list() -> list[Finding]:
    """§94.1: 前端硬编码模型列表 (绕过 useTranscriptionModels hook).

    §90 决策改了 useTranscriptionModels hook, 但 TranscriptSettings.tsx 硬编码 SelectItem 没改,
    用户在设置页看到的是 v0.7 W2.5 列表 (SenseVoice 23 段 / Paraformer 10 段 / Parakeet),
    不是 §90 决策列表 (FunASR-Nano 947MB Pro + SenseVoice 228MB + Paraformer 216MB).
    """
    findings = []
    # 已知的硬编码 v0.7 W2.5 模式 (§90 应删 / 应改)
    hardcoded_patterns = [
        ("frontend/src/components/TranscriptSettings.tsx",
         r"parakeet.*(?:旧推荐|旧|v0\.7|不推荐)",
         "硬编码 parakeet 选项 (§90 v0.8+ 已删)"),
        ("frontend/src/components/TranscriptSettings.tsx",
         r"SenseVoice-zh\s*\(\s*推荐\s*·\s*23\s*段",
         "硬编码 SenseVoice 23 段 (§90 决策 228MB)"),
        ("frontend/src/components/TranscriptSettings.tsx",
         r"Paraformer-zh\s*\(\s*备选\s*·\s*10\s*段",
         "硬编码 Paraformer 10 段 (§90 决策 216MB)"),
    ]
    for rel, pat, desc in hardcoded_patterns:
        fp = REPO / rel
        if fp.exists():
            text = fp.read_text(encoding="utf-8")
            if re.search(pat, text):
                findings.append(Finding(
                    "error", "hardcoded_model_list",
                    f"{desc}: {rel}",
                    rel,
                ))
    return findings


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--strict", action="store_true", help="exit 1 on any error")
    ap.add_argument("--json", action="store_true", help="JSON output")
    args = ap.parse_args()

    print("=== meetily codebase audit (AGENTS.md §94 §6.1) ===\n")

    findings: list[Finding] = []
    findings.extend(check_backup_files())
    findings.extend(check_orphan_modules())
    findings.extend(check_v085_residue())
    findings.extend(check_hardcoded_model_list())
    findings.extend(check_import_whisper_fallback())
    findings.extend(check_git_tracked_backup())
    findings.extend(check_version_consistency())
    findings.extend(check_identifier_consistency())
    dangling, orphan, n_backend, n_frontend = check_invoke_commands()
    findings.extend(dangling)
    findings.extend(orphan)

    by_severity = {"error": 0, "warn": 0, "info": 0}
    by_category = {}
    for f in findings:
        by_severity[f.severity] += 1
        by_category[f.category] = by_category.get(f.category, 0) + 1

    if args.json:
        print(json.dumps({
            "summary": {
                "errors": by_severity["error"],
                "warns": by_severity["warn"],
                "info": by_severity["info"],
                "by_category": by_category,
                "backend_commands": n_backend,
                "frontend_commands": n_frontend,
            },
            "findings": [f.to_dict() for f in findings],
        }, indent=2, ensure_ascii=False))
    else:
        # 打印
        print(f"后端 invoke_handler: {n_backend} commands")
        print(f"前端 invoke() 调用: {n_frontend} unique")
        print(f"悬空 (前端调后端没注册): {len(dangling)}")
        print(f"孤儿 (后端注册前端没调): {len(orphan)}")
        print()
        if not findings:
            print("✅ ALL CHECKS PASSED\n")
        else:
            print(f"=== Findings ({len(findings)} total) ===\n")
            for f in findings:
                icon = {"error": "❌", "warn": "⚠️ ", "info": "ℹ️ "}[f.severity]
                line_info = f" :{f.line}" if f.line else ""
                print(f"  {icon} [{f.category}] {f.message}{line_info}")
            print()

        print(f"=== Summary ===")
        print(f"  errors: {by_severity['error']}")
        print(f"  warns:  {by_severity['warn']}")
        print(f"  info:   {by_severity['info']}")

    if args.strict and by_severity["error"] > 0:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
