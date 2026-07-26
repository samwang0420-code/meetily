#!/usr/bin/env bash
# apply-w2.sh - W2 国产 ASR 一键应用补丁(在 meetily 仓库根目录执行)
#
# 用法:bash outputs/patches/w2-sherpa-asr/apply.sh
# 作用:
#   1. 拷贝 sherpa_provider.rs 到 src/audio/transcription/
#   2. 修改 mod.rs 注册模块
#   3. 修改 engine.rs 加 'sherpa_paraformer' / 'sherpa_funasr_nano' 分支
#   4. 修改 api.rs TranscriptConfig 默认值
#   5. 修改 database schema 加 sherpa_provider 列
#   6. 修改 Cargo.toml 加 sherpa-rs 依赖
#   7. 改 TranscriptSettings.tsx UI
#
# 前置:
#   - cd ~/Documents/meetily && git checkout feature/w1-no-cloud
#   - 已 brew install cmake
#   - 已跑 bash outputs/scripts/install-sherpa-onnx.sh
#   - 已跑 bash outputs/scripts/install-sherpa-asr.sh

set -e

REPO="${1:-$(cd ../.. && pwd)}"
echo "📦 应用到仓库: ${REPO}"

if [ ! -d "${REPO}/frontend/src-tauri/src/audio/transcription" ]; then
    echo "❌ 找不到 audio/transcription 目录,请确认仓库正确"
    exit 1
fi

cd "${REPO}"

# 1) sherpa_provider.rs 拷贝
echo "1️⃣  拷贝 sherpa_provider.rs"
cp outputs/patches/w2-sherpa-asr/sherpa_provider.rs \
   frontend/src-tauri/src/audio/transcription/sherpa_provider.rs

# 2) 注册到 mod.rs
echo "2️⃣  patch audio/transcription/mod.rs"
python3 << 'PY'
import re
file = "frontend/src-tauri/src/audio/transcription/mod.rs"
s = open(file).read()

if "pub mod sherpa_provider" not in s:
    # 在 parakeet_provider 后加一行
    s = s.replace(
        "pub mod parakeet_provider;",
        "pub mod parakeet_provider;\npub mod sherpa_provider;"
    )
    s = s.replace(
        "pub use parakeet_provider::ParakeetProvider;",
        "pub use parakeet_provider::ParakeetProvider;\npub use sherpa_provider::{SherpaProvider, SherpaBackend, SherpaProviderConfig};"
    )
    open(file, "w").write(s)
    print("  mod.rs patched")
else:
    print("  mod.rs already patched")
PY

# 3) engine.rs 加分支
echo "3️⃣  patch audio/transcription/engine.rs"
python3 << 'PY'
file = "frontend/src-tauri/src/audio/transcription/engine.rs"
s = open(file).read()

if "sherpa_paraformer" not in s:
    # 在 TranscriptEngine::Provider 分支前加一个新的枚举变体
    # 同时在 get_or_init_transcription_engine 的 match 加分支
    # 这里采用最稳妥的字符串 anchor 替换

    # 3a) TranscriptEngine 枚举加变体
    s = s.replace(
        "Provider(Arc<dyn TranscriptionProvider>),  // Trait-based (preferred for new code)",
        "Provider(Arc<dyn TranscriptionProvider>),  // Trait-based (preferred for new code)\n    Sherpa(Arc<crate::audio::transcription::sherpa_provider::SherpaProvider>),  // 离线会记 W2: 国产 ASR"
    )

    # 3b) TranscriptionEngine::is_model_loaded 分支
    s = s.replace(
        "Self::Provider(provider) => provider.is_model_loaded().await,",
        "Self::Provider(provider) => provider.is_model_loaded().await,\n            Self::Sherpa(provider) => provider.is_model_loaded().await,"
    )

    # 3c) TranscriptionEngine::get_current_model 分支
    s = s.replace(
        "Self::Provider(provider) => provider.get_current_model().await,\n        }\n    }\n\n    /// Get the provider name for logging",
        "Self::Provider(provider) => provider.get_current_model().await,\n            Self::Sherpa(provider) => provider.get_current_model().await,\n        }\n    }\n\n    /// Get the provider name for logging"
    )

    # 3d) provider_name 分支
    s = s.replace(
        "Self::Provider(provider) => provider.provider_name(),\n        }\n    }\n}",
        "Self::Provider(provider) => provider.provider_name(),\n            Self::Sherpa(provider) => provider.provider_name(),\n        }\n    }\n}"
    )

    # 3e) get_or_init_transcription_engine:在 \"parakeet\" 分支后插入 sherpa 分支
    # anchor: \"Self::Provider(_)\" 的 fallback 之前
    SH_BLOCK = '''        "sherpa_paraformer" => {
            info!("🐉 初始化 sherpa-onnx Paraformer-zh INT8 ASR");
            let cfg = crate::audio::transcription::SherpaProviderConfig::default_for_free_user();
            let provider = Arc::new(
                crate::audio::transcription::SherpaProvider::new(cfg)
                    .map_err(|e| format!("sherpa-onnx 初始化失败: {}", e))?
            );
            // 触发懒加载
            use tauri::Manager as _;
            let _ = app.state::<crate::AppState>();
            Ok(TranscriptionEngine::Sherpa(provider))
        }
        "sherpa_funasr_nano" => {
            info!("🔮 初始化 sherpa-onnx SenseVoice INT8 ASR (Pro)");
            let cfg = crate::audio::transcription::SherpaProviderConfig::default_for_pro_user();
            let provider = Arc::new(
                crate::audio::transcription::SherpaProvider::new(cfg)
                    .map_err(|e| format!("sherpa-onnx 初始化失败: {}", e))?
            );
            Ok(TranscriptionEngine::Sherpa(provider))
        }
        "localWhisper" | _ => {'''
    s = s.replace('"localWhisper" | _ => {', SH_BLOCK)

    open(file, "w").write(s)
    print("  engine.rs patched")
else:
    print("  engine.rs already patched")
PY

# 4) api.rs:TranscriptConfig 默认值 + DEFAULT_PARAKEET_MODEL 旁加一个
echo "4️⃣  patch api.rs + config.rs 默认值"
python3 << 'PY'
import re
api_file = "frontend/src-tauri/src/api/api.rs"
s = open(api_file).read()
if "sherpa_paraformer" not in s:
    # 将 api_get_transcript_config 默认从 parakeet 改为 sherpa_paraformer
    # (anchor: provider: "parakeet".to_string())
    s = s.replace(
        'provider: "parakeet".to_string(),',
        'provider: "sherpa_paraformer".to_string(),'
    )
    open(api_file, "w").write(s)
    print("  api.rs 默认值改 sherpa_paraformer")
else:
    print("  api.rs 已改")

cfg_file = "frontend/src-tauri/src/config.rs"
s = open(cfg_file).read()
if "DEFAULT_SHERPA_PARAFORMER_MODEL" not in s:
    add = '''
/// 离线会记 W2: 默认国产 ASR 模型 ID
pub const DEFAULT_SHERPA_PARAFORMER_MODEL: &str = "sherpa_paraformer";
pub const DEFAULT_SHERPA_FUNASR_NANO_MODEL: &str = "sherpa_funasr_nano";
'''
    s = s.replace(
        'pub const DEFAULT_PARAKEET_MODEL',
        add + '\npub const DEFAULT_PARAKEET_MODEL'
    )
    open(cfg_file, "w").write(s)
    print("  config.rs 加 DEFAULT_SHERPA_* 常量")
else:
    print("  config.rs already has DEFAULT_SHERPA_*")
PY

# 5) DB schema
echo "5️⃣  patch migrations"
NEW_MIG_FILE="frontend/src-tauri/migrations/20260709_add_sherpa_provider.sql"
cat > "${NEW_MIG_FILE}" << 'SQL'
-- W2 2026-07-09:加 sherpa provider 字段
-- transcript_settings 表加 一列 sherpa_provider 用于切换 国产 ASR 后端

ALTER TABLE transcript_settings ADD COLUMN sherpa_provider TEXT DEFAULT 'paraformer';
ALTER TABLE transcript_settings ADD COLUMN asr_quality TEXT DEFAULT 'balanced';
SQL
echo "  加了 ${NEW_MIG_FILE}"

# 6) Cargo.toml 加 sherpa-rs 依赖
echo "6️⃣  patch Cargo.toml"
python3 << 'PY'
toml_file = "frontend/src-tauri/Cargo.toml"
s = open(toml_file).read()
if "sherpa-rs" not in s:
    # anchor: 在 eyre 那行附近(meetily 已经有 eyre),加 sherpa-rs
    if "eyre" in s:
        s = s.replace(
            'eyre = { version = "0.6"',
            'eyre = { version = "0.6"\nsherpa-rs = { git = "https://github.com/thewh1teagle/sherpa-rs", tag = "v0.6.8", default-features = false, features = ["download-binaries"] }'
        )
    else:
        # 否则通用加在末尾
        s += '\n[target.'\''cfg(target_os = "macos")'\'']\nsherpa-rs = { git = "https://github.com/thewh1teagle/sherpa-rs", tag = "v0.6.8", default-features = false, features = ["download-binaries"] }\n'
    open(toml_file, "w").write(s)
    print("  Cargo.toml 已加 sherpa-rs 依赖")
else:
    print("  Cargo.toml 已经包含 sherpa-rs")
PY

# 7) UI 加切换
echo "7️⃣  patch TranscriptSettings.tsx UI"
python3 << 'PY'
file = "frontend/src/components/TranscriptSettings.tsx"
s = open(file).read()
if "'sherpa_paraformer'" not in s:
    # 加 provider 枚举(在 TranscriptModelProps 那个 union 里)
    s = s.replace(
        "provider: 'localWhisper' | 'parakeet' | 'deepgram' | 'elevenLabs' | 'groq' | 'openai';",
        "provider: 'localWhisper' | 'parakeet' | 'sherpa_paraformer' | 'sherpa_funasr_nano' | 'deepgram' | 'elevenLabs' | 'groq' | 'openai';"
    )
    # 在 localWhisper 的分支后注入 sherpa 选项
    insert_at = '''        } else if (provider === 'localWhisper') {
            // Whisper
            fetchApiKey(provider);
        }'''
    new_insert = '''        } else if (provider === 'localWhisper') {
            // Whisper
            fetchApiKey(provider);
        } else if (provider === 'sherpa_paraformer') {
            // W2:免费国产 Paraformer-zh INT8
            setApiKey(null);
        } else if (provider === 'sherpa_funasr_nano') {
            // W2:Pro 国产 SenseVoice INT8 (FunASR-Nano 替代)
            setApiKey(null);
        }'''
    # 简化:用更宽松 anchor
    s = s.replace(
        "if (provider !== 'localWhisper' && provider !== 'parakeet') {\n                                    fetchApiKey(provider);",
        "if (provider !== 'localWhisper' && provider !== 'parakeet' && provider !== 'sherpa_paraformer' && provider !== 'sherpa_funasr_nano') {\n                                    fetchApiKey(provider);"
    )

    open(file, "w").write(s)
    print("  TranscriptSettings.tsx 加 sherpa_* 选项")
else:
    print("  TranscriptSettings.tsx already has sherpa")
PY

echo ""
echo "✅ W2 补丁已应用"
echo ""
echo "下一步:"
echo "  cargo check -p meetily  (1) 首次会很慢,要 git clone sherpa-rs 子模块,约 5-10 分钟"
echo "  编译通过后跑 cargo build"
echo "  修改 DB 让默认 provider 走 sherpa_paraformer:"
echo "    sqlite3 ~/Library/Application\\ Support/cn.lixianhuiji.app/meeting_minutes.sqlite \\"
echo "      \"UPDATE transcript_settings SET sherpa_provider='paraformer' WHERE id='1';\""
echo "  tauri dev 启动,在 TranscriptSettings 选 Paraformer-zh 或 SenseVoice"
