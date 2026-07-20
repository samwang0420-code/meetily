#!/usr/bin/env python3
"""
离线会记 W1 P0: ASR daemon 真实录音转写 + 热词纠错实测脚本.

CLI 跑 (绕过 Tauri app, 不受 launchd silent abort 影响):
  python3 test_daemon_hotwords.py <audio.wav_or_raw_f32> [model] [hotwords_pack] [hotwords_custom]

输入:
  - audio.wav_or_raw_f32: .wav 文件 (会被 soundfile 解码) 或 .f32 文件 (raw float32 samples)

输出:
  - raw_text (模型原始输出)
  - hotword_bias stderr 日志: words=N l0_hits=N
  - 成功时 stderr 末尾有 hotword_bias 行

⚠️  Tauri app 的 ASR 链路验证仍需 GUI 启动 (这条脚本只能验证 daemon 自身).
"""
import base64
import json
import os
import subprocess
import sys


def load_audio_for_daemon(path: str, sample_rate: int = 16000) -> str:
    """Return base64 of either raw float32 samples (.f32) or wav-decoded samples."""
    if path.endswith(".f32"):
        return base64.b64encode(open(path, "rb").read()).decode()
    # .wav / .flac / anything soundfile supports
    import numpy as np
    import soundfile as sf
    data, sr = sf.read(path)
    data = data.astype(np.float32)
    if data.ndim > 1:
        data = data.mean(axis=1)
    if sr != sample_rate:
        ratio = sample_rate / sr
        data = np.interp(
            np.linspace(0, len(data), int(len(data) * ratio)),
            np.arange(len(data)),
            data,
        ).astype(np.float32)
    return base64.b64encode(data.tobytes()).decode()


def transcribe_via_daemon(model: str, audio_path: str,
                          hotwords_pack: str = "none",
                          hotwords_custom: str = "") -> dict:
    script_dir = os.path.dirname(os.path.abspath(__file__))
    daemon = os.path.join(script_dir, "sherpa_asr.py")
    proc = subprocess.Popen(
        [sys.executable, daemon],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        audio_b64 = load_audio_for_daemon(audio_path)
        req = {
            "id": "test-1",
            "action": "transcribe",
            "model": model,
            "audio_b64": audio_b64,
            "sample_rate": 16000,
            "language": "zh",
            "hotwords_pack": hotwords_pack,
            "hotwords_custom": hotwords_custom,
        }
        proc.stdin.write(json.dumps(req, ensure_ascii=False) + "\n")
        proc.stdin.flush()
        proc.stdin.close()
        line = proc.stdout.readline()
        stderr = proc.stderr.read() if proc.poll() is not None else ""
        # Extract last hotword_bias line for visibility
        bias_line = ""
        for ln in stderr.splitlines():
            if "hotword_bias" in ln:
                bias_line = ln
        return {
            "model": model,
            "hotwords_pack": hotwords_pack,
            "hotwords_custom": hotwords_custom,
            "response": json.loads(line) if line.strip() else {},
            "hotword_bias_stderr": bias_line,
            "stderr_tail": stderr[-300:] if stderr else "",
        }
    finally:
        proc.terminate()


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    audio_path = sys.argv[1]
    model = sys.argv[2] if len(sys.argv) > 2 else "paraformer-zh"
    hotwords_pack = sys.argv[3] if len(sys.argv) > 3 else "tech"
    hotwords_custom = sys.argv[4] if len(sys.argv) > 4 else "Meetily,SenseVoice,Paraformer,BlockNote"

    print(f"=== transcribe model={model} pack={hotwords_pack} custom='{hotwords_custom[:40]}...' ===")
    result = transcribe_via_daemon(model, audio_path, hotwords_pack, hotwords_custom)
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
