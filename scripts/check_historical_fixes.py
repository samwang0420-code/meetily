#!/usr/bin/env python3
"""
meetily historical-fix guard (AGENTS.md §35)
Independent of cargo/next build. Pure-text grep against the repo
to ensure that previously-fixed regressions are still present.

§15 §37 compliance: this script is a hard gate. Any non-zero
exit code blocks release binary.
"""
from __future__ import annotations
import argparse
import os
import subprocess
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def grep(pattern: str, path: str) -> bool:
    """rg if available, else grep -r. Exit 0 = match found."""
    try:
        r = subprocess.run(
            ["rg", "--quiet", pattern, path],
            capture_output=True, timeout=20,
        )
        return r.returncode == 0
    except FileNotFoundError:
        r = subprocess.run(
            ["grep", "-rqE", pattern, path],
            capture_output=True, timeout=20,
        )
        return r.returncode == 0


# Each anchor = (id, file, regex). The regex MUST match for the fix to be live.
# If a fix commit was rebased away, the regex will fail, and the guard fails.
ANCHORS = [
    # §32: 8s force-split threshold for continuous speech
    ("32_forced_split_threshold",
     "frontend/src-tauri/src/audio/transcription/worker.rs",
     r"(8\s*\*\s*1000|8000|forced_split|force_split)"),
    ("32_continuous_speech_test",
     "frontend/src-tauri/src/audio/transcription/worker.rs",
     r"test_continuous_speech_is_force_split_for_live_output"),

    # §33: VAD timestamp = absolute ms * 16000 / 1000, NO processed_samples add-back
    ("33_timestamp_not_double_counted",
     "frontend/src-tauri/src/audio/vad.rs",
     r"timestamp_ms\s*\*\s*16000\s*/\s*1000"),
    ("33_timestamp_test",
     "frontend/src-tauri/src/audio/vad.rs",
     r"test_speech_start_timestamp_is_not_double_counted"),

    # §34: force-split suppresses SpeechEnd.samples re-emission
    ("34_force_split_suppress_repeat",
     "frontend/src-tauri/src/audio/transcription/worker.rs",
     r"(suppress.*speech_end|speech_end_after_split|test_speech_end_does_not_repeat)"),

    # §23: sherpa daemon 2min idle kill (memory leak fix)
    ("23_daemon_idle_kill",
     "frontend/src-tauri/src/audio/sherpa_daemon.rs",
     r"(idle_kill|shutdown_global_daemon|touch_daemon_activity)"),

    # §36: Map-Reduce 1800-token hard cap (anti-hallucination)
    ("36_local_summary_chunk_threshold",
     "frontend/src-tauri/src/summary/service.rs",
     r"(LOCAL_SUMMARY_CHUNK_THRESHOLD\s*=\s*1800|min\(\s*1800)"),
    ("36_narrative_empty_states",
     "frontend/src-tauri/src/summary/templates/standard_meeting.json",
     r"(本次无新决议|本次无行动事项|单向叙事)"),

    # §22 §24: summary auth_token fallback to DB lookup
    ("24_summary_auth_db_fallback",
     "frontend/src-tauri/src/user/commands.rs",
     r"latest_session_in_db"),

    # §29: FunASR-Nano pro-only pricing tier gate
    ("29_funasr_nano_tier_gate",
     "frontend/src-tauri/src",
     r"(pro_only_funasr_nano|FunASR.*pro|Pro.*FunASR)"),

    # §27: 5GB import hard limit (user 7/22 explicit instruction)
    ("27_5gb_import_limit",
     "frontend/src-tauri/src/audio/import.rs",
     r"5\s*\*\s*1024\s*\*\s*1024\s*\*\s*1024"),

    # §38: default model = FunASR-Nano high precision
    ("38_default_funasr_nano",
     "frontend/src/contexts/ConfigContext.tsx",
     r"funasr-nano-zh"),

    # §79 P0-A Phase 2: topic graph LLM extract via BuiltInAI spawn
    ("79_topic_graph_llm_extract_spawn",
     "frontend/src-tauri/src/topic_graph/mod.rs",
     r"trigger_after_summary"),

    # §79 P0-A: summary service spawn topic graph extract
    ("79_topic_graph_spawn_in_service",
     "frontend/src-tauri/src/summary/service.rs",
     r"crate::topic_graph::trigger_after_summary"),

    # §79 P0-A: prompt builder still 1:1 (no regression)
    ("79_topic_extract_prompt_intact",
     "frontend/src-tauri/src/topic_graph/extract.rs",
     r"PROMPT_INSTRUCTIONS|逐行 JSON"),

    # §81 P2-A: action_items table migration
    ("81_action_items_migration",
     "frontend/src-tauri/migrations/20260807000000_action_items.sql",
     r"CREATE TABLE IF NOT EXISTS action_items"),

    # §81 P2-A: action_items mod.rs exists with toggle fn
    ("81_action_items_module",
     "frontend/src-tauri/src/action_items/mod.rs",
     r"pub async fn toggle_action_item"),

    # §81 P2-A: ActionItemsList frontend component
    ("81_action_items_ui",
     "frontend/src/components/MeetingDetails/ActionItemsList.tsx",
     r"api_action_item_toggle"),

    # §81 P2-A: invoke_handler registers action_items commands
    ("81_action_items_commands_registered",
     "frontend/src-tauri/src/lib.rs",
     r"action_items::api_action_item_(list|toggle)"),

    # §83 P2-C: topic recent backend command
    ("83_topic_recent_command",
     "frontend/src-tauri/src/topic_graph/mod.rs",
     r"pub async fn api_topic_recent"),

    # §83 P2-C: TopicSearchModal frontend
    ("83_topic_search_modal",
     "frontend/src/components/TopicSearch/TopicSearchModal.tsx",
     r"api_topic_search|api_topic_recent"),

    # §83 P2-C: layout.tsx mounts Cmd/Ctrl+K launcher
    ("83_topic_search_layout_launcher",
     "frontend/src/app/layout.tsx",
     r"TopicSearchLauncher"),

    # §P2-B: rebuild_dossier backend function
    ("p2b_rebuild_topic_dossier",
     "frontend/src-tauri/src/topic_graph/mod.rs",
     r"rebuild_topic_dossier"),

    # §P2-B: api_topic_rebuild_dossier registered
    ("p2b_rebuild_command",
     "frontend/src-tauri/src/lib.rs",
     r"api_topic_rebuild_dossier"),

    # §P2-B: TopicSearchModal rebuild button
    ("p2b_rebuild_button",
     "frontend/src/components/TopicSearch/TopicSearchModal.tsx",
     r"api_topic_rebuild_dossier"),

    # §P1-B: speaker aliases migration
    ("p1b_speaker_aliases_migration",
     "frontend/src-tauri/migrations/20260807000001_speaker_aliases.sql",
     r"CREATE TABLE IF NOT EXISTS speaker_aliases"),

    # §P1-B: speaker_aliases backend module + set_alias
    ("p1b_speaker_aliases_backend",
     "frontend/src-tauri/src/speaker_aliases/mod.rs",
     r"pub async fn set_alias"),

    # §P1-B: speaker commands registered
    ("p1b_speaker_commands",
     "frontend/src-tauri/src/lib.rs",
     r"speaker_aliases::api_speaker_alias_(list|set)"),

    # §P1-B: SpeakerRosterDrawer exists
    ("p1b_speaker_roster_drawer",
     "frontend/src/components/SpeakerRoster/SpeakerRosterDrawer.tsx",
     r"api_speaker_alias_set"),

    # §P1-B: SummaryPanel mounts the drawer trigger
    ("p1b_speaker_drawer_in_summary",
     "frontend/src/components/MeetingDetails/SummaryPanel.tsx",
     r"SpeakerRosterDrawer|open-speaker-roster"),

    # §31 P0: memory_watcher backend module + Tauri command
    ("31p0_memory_watcher_module",
     "frontend/src-tauri/src/hardware/memory_watcher.rs",
     r"device_get_memory_recommendation|start_memory_watcher"),

    # §31 P0: start_recording spawns watcher, stop_recording stops
    ("31p0_recording_hooks",
     "frontend/src-tauri/src/audio/recording_commands.rs",
     r"start_memory_watcher\(app\.clone\(\)\)|stop_memory_watcher\(\);"),

    # §31 P0: RecordingControls listens for memory-pressure toast
    ("31p0_frontend_listener",
     "frontend/src/components/RecordingControls.tsx",
     r"memory-pressure"),

    # §78 fix P0-A: topic_graph commands also must be registered (was missed in §78)
    ("78_topic_graph_commands_registered",
     "frontend/src-tauri/src/lib.rs",
     r"topic_graph::api_topic_"),

    # §15: Rust integration tests under #[cfg(test)]
    ("15_rust_tests_compile",
     "frontend/src-tauri/src/audio/transcription/worker.rs",
     r"#\[cfg\(test\)\]"),

    # §56 §40 follow-up: Bluetooth timeout uses integer math (u128 nanos, no f64/f32)
    ("56_bluetooth_buffer_timeout_integer",
     "frontend/src-tauri/src/audio/device_detection.rs",
     r"with_headroom_nanos: u128|from_secs_f64|mul_f32"),

    # §56 §31 P0 follow-up: devices path also spawns memory watcher
    ("56_p0_devices_path_memory_watcher",
     "frontend/src-tauri/src/audio/recording_commands.rs",
     r"devices path"),

    # §P2-B nightly scheduler: topic_graph/scheduler.rs 模块 + 启动入口
    ("p2b_scheduler_module",
     "frontend/src-tauri/src/topic_graph/scheduler.rs",
     r"start_topic_dossier_scheduler|run_one_pass"),

    # §P2-B nightly scheduler: lib.rs setup 调用
    ("p2b_scheduler_spawned",
     "frontend/src-tauri/src/lib.rs",
     r"Topic dossier nightly scheduler started"),

    # §P2-C live Q&A: 后端模块 + Tauri command 注册
    ("p2c_live_qa_module",
     "frontend/src-tauri/src/live_qa/mod.rs",
     r"api_meeting_live_qa|ask_live_qa"),

    # §P2-C live Q&A: invoke_handler 注册
    ("p2c_live_qa_registered",
     "frontend/src-tauri/src/lib.rs",
     r"live_qa::api_meeting_live_qa"),

    # §P2-C live Q&A: 前端 overlay 组件 (Alt+Space 弹窗)
    ("p2c_live_qa_overlay",
     "frontend/src/components/LiveQA/LiveQAOverlay.tsx",
     r"api_meeting_live_qa|altKey"),

    # §P2-C live Q&A: meeting-details 页挂载 overlay
    ("p2c_live_qa_mounted",
     "frontend/src/app/meeting-details/page.tsx",
     r"LiveQAOverlay"),

    # §90 app name: tauri.conf.json bundleName + Info.plist CFBundleDisplayName 都是 言镜 AI
    ("90_app_name_bundle",
     "frontend/src-tauri/tauri.conf.json",
     r'"bundleName": "言镜 AI"'),
    ("90_app_name_info_plist",
     "frontend/src-tauri/Info.plist",
     r"CFBundleDisplayName|言镜 AI"),

    # §90 transcript model: v0.8+ 列表 (FunASR-Nano 947MB / SenseVoice 228MB / Paraformer 216MB)
    ("90_transcript_models_v08",
     "frontend/src/hooks/useTranscriptionModels.ts",
     r"947MB|216MB|228MB"),

    # §90 summary models: 隐藏未下载, 默认只显示 available
    ("90_summary_models_filter",
     "frontend/src/components/BuiltInModelManager.tsx",
     r"showAllModels|not_downloaded"),

    # §90 pricing scroll: main 加 h-screen overflow-y-auto
    ("91_bug1_verify_i18n_pathname",
     "scripts/verify_i18n.mjs", r"fileURLToPath"),
    ("91_bug2_mcp_read_write",
     "meetily-mcp/src/main.rs", r"SQLITE_OPEN_READ_WRITE"),
    ("91_bug3_obsidian_duration_max_minus_min",
     "frontend/src-tauri/src/obsidian_export/mod.rs", r"max_end - min_start"),
    ("91_bug4_recording_devices_meeting_id",
     "frontend/src-tauri/src/audio/recording_commands.rs", r"meeting_id\.clone"),
    ("91_bug5_action_items_markdown_parser",
     "frontend/src-tauri/src/action_items/mod.rs", r"parse_markdown_action_items"),
    ("91_p1b_speaker_label_in_meeting_transcript",
     "frontend/src-tauri/src/api/api.rs", r"speaker_label: Option<String>"),
    ("91_p1b_speaker_join_in_sql",
     "frontend/src-tauri/src/database/repositories/meeting.rs", r"LEFT JOIN speaker_aliases"),
    ("91_p1b_transcript_view_renders_speaker",
     "frontend/src/components/TranscriptView.tsx", r"speaker_label \|\| transcript\.speaker_id"),
    ("91_hotwords_thuocl_it_json",
     "frontend/src-tauri/scripts/hotwords_data/thuocl_it.json", r'"words":'),
    ("91_hotwords_lawgpt_legal_json",
     "frontend/src-tauri/scripts/hotwords_data/lawgpt_legal_vocab.json", r'"words":'),
    ("91_hotwords_omaha_medical_json",
     "frontend/src-tauri/scripts/hotwords_data/omaha_medical.json", r'"words":'),
    ("91_hotwords_six_packs",
     "frontend/src-tauri/scripts/sherpa_hotwords.py", r"PACK_FILES = \{"),
    ("91_hotwords_list_packs_command",
     "frontend/src-tauri/src/user/commands.rs", r"hotwords_list_packs"),
    ("91_hotwords_pack_index",
     "frontend/src-tauri/scripts/hotwords_data/packs_index.json", r'"packs":'),
    ("91_p1b_test_passes",
     "frontend/src-tauri/src/speaker_auto_attach/mod.rs", r"test_detect_chinese_basic"),
    ("91_action_items_test_passes",
     "frontend/src-tauri/src/action_items/mod.rs", r"test_parse_e5b78a31_real_meeting"),
        ("90_pricing_page_scroll",
     "frontend/src/components/MainContent/index.tsx",
     r"h-screen overflow-y-auto"),

    # ===== §62 三联优化 (2026-08-07 补 anchor, 之前 guard 漏了) =====
    # §62 A: 多 daemon pool (Vec<Mutex<Option<SherpaHandle>>>) — 不是单 daemon
    ("62_a_sherpa_daemon_pool",
     "frontend/src-tauri/src/audio/sherpa_daemon.rs",
     r"inner:\s*Vec<Mutex<Option<SherpaHandle>>>"),
    # §62 A: round-robin counter (AtomicUsize + Relaxed)
    ("62_a_round_robin_atomic",
     "frontend/src-tauri/src/audio/sherpa_daemon.rs",
     r"counter:\s*AtomicUsize"),
    # §62 A: env MEETILY_SHERPA_DAEMONS 解析
    ("62_a_env_daemon_count",
     "frontend/src-tauri/src/audio/sherpa_daemon.rs",
     r"MEETILY_SHERPA_DAEMONS"),
    # §62 A: 三个新单测 (round-robin + pool count + distribution)
    ("62_a_test_round_robin",
     "frontend/src-tauri/src/audio/sherpa_daemon.rs",
     r"section_64_round_robin_wraps_within_pool"),
    # §62 B.1: import.rs hardlink
    ("62_b1_import_hardlink",
     "frontend/src-tauri/src/audio/import.rs",
     r"Section 64 hardlinked"),
    # §62 B.3: decoder.rs /tmp wav (temp_dir not parent_dir)
    ("62_b3_tmp_wav",
     "frontend/src-tauri/src/audio/decoder.rs",
     r"std::env::temp_dir"),
    # §62 C: max_tokens 1200→800
    ("62_c_max_tokens_800",
     "frontend/src-tauri/src/summary/processor.rs",
     r"DEFAULT_SUMMARY_MAX_TOKENS:\s*u32\s*=\s*800"),

    # ===== §63 provider 路由 (2026-08-07 补 anchor, 之前 guard 漏了) =====
    # §63: sherpa_funasr_nano → funasr-nano-zh (不再是 sensevoice-zh)
    ("63_funasr_nano_backend",
     "frontend/src-tauri/src/audio/retranscription.rs",
     r'"funasr-nano-zh"'),

    # ===== §90 friendlyImportTitle (2026-08-07 补 anchor) =====
    # §90: ImportAudioDialog 把数字文件名改成 "导入音频 YYYY-MM-DD HH:MM"
    ("90_friendly_import_title",
     "frontend/src/components/ImportAudio/ImportAudioDialog.tsx",
     r"导入音频.*stamp|friendly date title"),

    # ===== UI 版本号同步 (2026-08-07 §92 P0) =====
    ("ui_version_0_8_6_sidebar",
     "frontend/src/components/Sidebar/index.tsx",
     r"v0\.8\.6"),
    ("ui_version_0_8_6_dashboard",
     "frontend/src/app/_components/HomeDashboard.tsx",
     r"v0\.8\.6"),

    # ===== §93: macOS .app bundle 同步 (2026-08-07) =====
    # §93 anchor: sync_app_bundle.sh 脚本存在
    ("93_sync_app_bundle_script",
     "scripts/sync_app_bundle.sh",
     r"sync_app_bundle|sync.*app.*bundle"),

    # ===== §94: 全面代码审计 (2026-08-07) =====
    # §94 anchor 1: audit_codebase.py 存在
    ("94_audit_codebase_script",
     "scripts/audit_codebase.py",
     r"audit_codebase|check_invoke_commands"),
    # §94 anchor 2: pre_release_check.sh 存在
    ("94_pre_release_check_script",
     "scripts/pre_release_check.sh",
     r"pre_release|ALL.*STEPS"),
    # §94 anchor 3: backup/orig 死代码已删
    ("94_no_backup_files",
     ".gitignore",
     r"\*\.backup|\*\.orig|\*_old\.rs"),
    # §94 anchor 4: audio_v2 孤儿模块已删
    ("94_no_audio_v2",
     ".gitignore",
     r"audio_v2/"),
    # §94 anchor 5: lib_old_complex 已删
    ("94_no_lib_old_complex",
     ".gitignore",
     r"lib_old_complex\.rs"),
    # §94 anchor 6: 4 悬空命令已修
    ("94_4_dangling_fixed",
     "frontend/src/lib/builtin-ai.ts",
     r"parakeet_get_models_directory"),
    # §94 anchor 7: get_streaming_timing_stats 已注册
    ("94_streaming_timing_registered",
     "frontend/src-tauri/src/lib.rs",
     r"audio::transcription::worker::get_streaming_timing_stats"),
    # §94 anchor 8: 4 悬空 + 4 修复总览
    # ===== §95: import.rs §58/§60 决策补做 (2026-08-09) =====
    ("95_import_provider_dispatch",
     "frontend/src-tauri/src/audio/import.rs",
     r"effective_provider: String = match provider\.as_deref"),
    ("95_import_use_sherpa_branch",
     "frontend/src-tauri/src/audio/import.rs",
     r"use_sherpa = effective_provider == .sherpa_funasr_nano"),
    ("95_import_no_whisper_fallback",
     "frontend/src-tauri/src/audio/import.rs",
     r"永远不.*fallback.*[Ww]hisper|永不.*fallback.*whisper"),

    ("94_audit_summary",
     "outputs/94-全面代码审计-代码漏系统性问题-2026-08-07.md",
     r"§94|全面代码审计"),
    # §97 (2026-08-09): identifier 改造 cn.lixianhuiji.app → tech.yanjingai.app + 数据迁移
    ("97_tauri_identifier_yanjingai",
     "frontend/src-tauri/tauri.conf.json",
     r'"identifier":\s*"tech\.yanjingai\.app"'),
    ("97_config_app_bundle_id",
     "frontend/src-tauri/src/config.rs",
     r'pub const APP_BUNDLE_ID:\s*&str\s*=\s*"tech\.yanjingai\.app"'),
    ("97_dirs_root_app_data_uses_const",
     "frontend/src-tauri/src/config.rs",
     r'format!\("Library/Application Support/\{\}", APP_BUNDLE_ID\)'),
    ("97_migrate_legacy_fn",
     "frontend/src-tauri/src/lib.rs",
     r'pub fn migrate_legacy_app_data\(\) -> anyhow::Result'),
    ("97_setup_calls_migrate",
     "frontend/src-tauri/src/lib.rs",
     r"if let Err\(e\) = migrate_legacy_app_data\(\)"),
    ("97_python_yanjingai_env_var",
     "frontend/src-tauri/scripts/sherpa_asr.py",
     r"YANJINGAI_DIAR_DB_PATH"),
    ("97_python_path_yanjingai",
     "frontend/src-tauri/scripts/sherpa_asr.py",
     r"Application Support/tech\.yanjingai\.app/models/sherpa"),
    ("97_panic_path_uses_const",
     "frontend/src-tauri/src/lib.rs",
     r"p\.join\(crate::config::APP_BUNDLE_ID\)"),
    ("97_privacy_page_yanjingai",
     "frontend/src/app/legal/privacy/page.tsx",
     r"tech\.yanjingai\.app/crashes/"),
    ("97_transcript_settings_yanjingai",
     "frontend/src/components/TranscriptSettings.tsx",
     r"tech\.yanjingai\.app/models/sherpa"),
    # §98 (2026-08-10): sqlx checksum mismatch 自愈 + Info.plist CFBundleIdentifier 同步
    ("98_sync_migration_checksums_fn",
     "frontend/src-tauri/src/database/manager.rs",
     r"async fn sync_migration_checksums\("),
    ("98_self_heal_calls",
     "frontend/src-tauri/src/database/manager.rs",
     r"Self::sync_migration_checksums\(&pool\)"),
    ("98_fix_sqlx_checksums_script",
     "scripts/fix_sqlx_checksums.py",
     r"sync_migration_checksums|sync_one"),
    ("98_sync_app_bundle_plist",
     "scripts/sync_app_bundle.sh",
     r"§97 plist sync|CFBundleIdentifier.*EXPECTED_ID"),

    # §99 (2026-08-10): §96 PYTHONUSERBASE=$HOME hack 删除 + §97 §99 models/ 目录递归迁移
    ("99_no_pythonuserbase_hack",
     "frontend/src-tauri/src/audio/sherpa_daemon.rs",
     r"PYTHONUSERBASE.*=.*home|PYTHONUSERBASE.*\$HOME"),
    ("99_spawn_unbuffered_only",
     "frontend/src-tauri/src/audio/sherpa_daemon.rs",
     r"PYTHONUNBUFFERED.{0,5}1"),
    ("99_spawn_test_exists",
     "frontend/src-tauri/src/audio/sherpa_daemon.rs",
     r"section_99_spawned_python_can_import_sherpa_onnx"),
    ("99_migrate_models_recursive",
     "frontend/src-tauri/src/lib.rs",
     r"fn copy_dir_recursive"),
    ("99_migrate_calls_models",
     "frontend/src-tauri/src/lib.rs",
     r"copy_dir_recursive\(&src_models"),
    ("99_migrate_log_models_count",
     "frontend/src-tauri/src/lib.rs",
     r"models_files_copied="),

    # §99.2 (2026-08-10): import.rs::create_meeting_with_transcripts 写 user_id (修复 §59 漏)
    ("99_2_import_writes_user_id",
     "frontend/src-tauri/src/audio/import.rs",
     r"let user_id: i64 = match crate::user::commands::latest_session_in_db"),
    ("99_2_insert_meetings_has_user_id",
     "frontend/src-tauri/src/audio/import.rs",
     r"INSERT INTO meetings \([^)]*user_id"),
    ("99_2_test_exists",
     "frontend/src-tauri/src/audio/import.rs",
     r"section_99_2_create_meeting_writes_user_id"),

    # §99.3 (2026-08-10): sync_app_bundle.sh 顺序修复 (cp 在 codesign 之前)
    # + ~/Applications symlink 帮 LaunchServices 标准目录接管 (避免 kLSNoExecutableErr)
    ("99_3_sync_cp_before_codesign",
     "scripts/sync_app_bundle.sh",
     r"cp -f.*SRC_BINARY.*DST_BINARY"),
    ("99_3_apps_dir_symlink",
     "scripts/sync_app_bundle.sh",
     r"USER_APPS_DIR=\"\$HOME/Applications"),
    ("99_3_open_symlink_hint",
     "scripts/sync_app_bundle.sh",
     r"open '\$APP_LINK"),

    # §99.4 (2026-08-10): tauri bundle 路径检测 + 直接 exec 启动方式推荐
    # macOS 26 LaunchServices 对 ~/Documents/.../*.app 持续拒绝扫描 (kLSNoExecutableErr),
    # 用户应直接 exec bundle binary 或重启 macOS 清 LaunchServices cache.
    ("99_4_tauri_bundle_detect",
     "scripts/sync_app_bundle.sh",
     r"TAURI_BUNDLE=.*bundle/macos"),
    ("99_4_direct_exec_recommend",
     "scripts/sync_app_bundle.sh",
     r"bundle/macos.*Contents/MacOS/meetily"),

    # §99.5 (2026-08-10): Tauri setup() 里不能用 tokio::spawn, 必须用 tauri::async_runtime::spawn
    # 主线程是 tao event loop 不是 Tokio runtime, tokio::spawn 直接 panic:
    #   "there is no reactor running, must be called from the context of a Tokio 1.x runtime"
    # §86 §88 §62 都用 tauri::async_runtime::spawn, §99.2 漏修导致启动即 abort.
    # positive 验证: §99.5 注释 + tauri::async_runtime::spawn 必须出现 (fix 在位)
    # negative 验证 (不能有 tokio::spawn at line start) 由 code review + 同 anchor 间接保证
    ("99_5_setup_backfill_uses_tauri_async_runtime_spawn",
     "frontend/src-tauri/src/lib.rs",
     r"§99\.5.*tauri::async_runtime::spawn"),

    # §99.6 (2026-08-10): sync_app_bundle.sh 必须也 sync tauri bundle binary
    # 之前只 sync hand-made app, 没 sync target/release/bundle/macos/言镜 AI.app
    # 用户跑 bundle/macos 路径拿到 §99.5 修复前的旧 binary panic (mtime 落后 14+ 分钟).
    ("99_6_sync_tauri_bundle_sha_check",
     "scripts/sync_app_bundle.sh",
     r"§99\.6.*synced tauri bundle binary"),
    ("99_6_sync_tauri_bundle_skip_when_same",
     "scripts/sync_app_bundle.sh",
     r"§99\.6.*already in sync"),
]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--strict", action="store_true",
                    help="exit 1 on any failure (CI gate)")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    print(f"=== Historical fix guard (AGENTS.md §35) — {len(ANCHORS)} anchors ===\n")
    passed, failed = 0, 0
    for anchor_id, rel_path, regex in ANCHORS:
        full_path = os.path.join(REPO, rel_path)
        if not os.path.exists(full_path):
            ok = False
            detail = f"FILE MISSING: {rel_path}"
        else:
            ok = grep(regex, full_path)
            detail = "OK" if ok else f"regex {regex!r} not found in {rel_path}"
        status = "PASS" if ok else "FAIL"
        print(f"  [{status}] {anchor_id:<40} {detail[:90]}")
        if ok:
            passed += 1
        else:
            failed += 1

    print(f"\nResult: {passed}/{len(ANCHORS)} anchors passed, {failed} failed.")
    if args.strict and failed > 0:
        print("STRICT MODE: refusing release binary.", file=sys.stderr)
        return 1
    return 0 if failed == 0 else 2


if __name__ == "__main__":
    raise SystemExit(main())
