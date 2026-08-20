#!/usr/bin/env python3
"""Ensure the official whisper.cpp CoreML encoder asset is present beside every
Whisper ggml model so the macOS CoreML fast path can actually load.

Whisper.cpp loads the CoreML encoder at runtime by initialising
``whisper_encoder_impl`` from ``whisper_encoder_impl.mlmodelc``. If the asset is
not next to the .bin file the runtime silently falls back to the CPU/Metal
encoder. This script downloads the official pre-converted .mlmodelc archive
for each downloaded Whisper ggml model and verifies that the encoder can be
opened with CoreML.

The mapping is taken from the upstream ``ggerganov/whisper.cpp`` repository
release tags (e.g. ``v1.7.1``), which the project already pins. Each
``coreml/<name>.mlmodelc.zip`` is published as a release artifact, so we use
HTTPS rather than re-converting with coremltools (no Python deps required).
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

# Keep the URL in sync with frontend/src-tauri/Cargo.toml whisper-rs 0.13.x →
# whisper.cpp 1.7.1 build artifacts.
WHISPER_CPP_TAG = os.environ.get("WHISPER_CPP_TAG", "v1.7.1")
WHISPER_CPP_RELEASE_URL = (
    f"https://github.com/ggml-org/whisper.cpp/releases/download/{WHISPER_CPP_TAG}"
)

# Map ggml model filename -> encoder mlmodelc archive (per the official release).
ENCODER_ASSETS: dict[str, str] = {
    "ggml-tiny.bin": "coreml/ggml-tiny.mlmodelc.zip",
    "ggml-tiny.en.bin": "coreml/ggml-tiny.en.mlmodelc.zip",
    "ggml-base.bin": "coreml/ggml-base.mlmodelc.zip",
    "ggml-base.en.bin": "coreml/ggml-base.en.mlmodelc.zip",
    "ggml-small.bin": "coreml/ggml-small.mlmodelc.zip",
    "ggml-small.en.bin": "coreml/ggml-small.en.mlmodelc.zip",
    "ggml-medium.bin": "coreml/ggml-medium.mlmodelc.zip",
    "ggml-medium.en.bin": "coreml/ggml-medium.en.mlmodelc.zip",
    "ggml-large-v1.bin": "coreml/ggml-large-v1.mlmodelc.zip",
    "ggml-large-v2.bin": "coreml/ggml-large-v2.mlmodelc.zip",
    "ggml-large-v3.bin": "coreml/ggml-large-v3.mlmodelc.zip",
    "ggml-large-v3-turbo.bin": "coreml/ggml-large-v3-turbo.mlmodelc.zip",
    # Q5/Q5_1 keep the same encoder as their parent model.
    "ggml-tiny-q5_1.bin": "coreml/ggml-tiny.mlmodelc.zip",
    "ggml-base-q5_1.bin": "coreml/ggml-base.mlmodelc.zip",
    "ggml-small-q5_1.bin": "coreml/ggml-small.mlmodelc.zip",
    "ggml-medium-q5_0.bin": "coreml/ggml-medium.mlmodelc.zip",
    "ggml-large-v3-turbo-q5_0.bin": "coreml/ggml-large-v3-turbo.mlmodelc.zip",
    "ggml-large-v3-q5_0.bin": "coreml/ggml-large-v3.mlmodelc.zip",
}

DEFAULT_MODELS_DIR = Path(
    "~/Library/Application Support/tech.yanjingai.app/models"
).expanduser()


def _http_get(url: str, dest: Path) -> None:
    with urllib.request.urlopen(url, timeout=120) as response:
        if response.status != 200:
            raise RuntimeError(f"HTTP {response.status} when downloading {url}")
        dest.parent.mkdir(parents=True, exist_ok=True)
        with dest.open("wb") as handle:
            shutil.copyfileobj(response, handle)


def ensure_encoder(models_dir: Path, model_filename: str) -> dict:
    model_path = models_dir / model_filename
    encoder_dir = model_path.with_suffix("")  # ggml-tiny.bin -> ggml-tiny
    encoder_dir = encoder_dir.with_name(encoder_dir.name + ".mlmodelc")
    result = {
        "model": model_filename,
        "encoder_dir": str(encoder_dir),
        "downloaded": encoder_dir.is_dir(),
        "url": None,
    }
    if encoder_dir.is_dir():
        return result
    asset = ENCODER_ASSETS.get(model_filename)
    if not asset:
        result["error"] = f"no encoder asset registered for {model_filename}"
        return result
    url = f"{WHISPER_CPP_RELEASE_URL}/{asset}"
    archive = models_dir / asset
    result["url"] = url
    try:
        _http_get(url, archive)
    except (urllib.error.URLError, RuntimeError) as exc:
        result["error"] = str(exc)
        return result
    encoder_dir.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive) as zipf:
        zipf.extractall(encoder_dir.parent)
    archive.unlink(missing_ok=True)
    result["downloaded"] = encoder_dir.is_dir()
    return result


def iter_models(models_dir: Path) -> list[Path]:
    if not models_dir.exists():
        return []
    return sorted(models_dir.glob("ggml-*.bin"))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--models-dir", type=Path, default=DEFAULT_MODELS_DIR)
    parser.add_argument("--json", action="store_true", help="machine-readable output")
    args = parser.parse_args()

    models = iter_models(args.models_dir)
    summary = {
        "models_dir": str(args.models_dir),
        "tag": WHISPER_CPP_TAG,
        "models_total": len(models),
        "encoders_present": 0,
        "encoders_downloaded": 0,
        "encoders_failed": 0,
        "details": [],
    }
    for model in models:
        result = ensure_encoder(args.models_dir, model.name)
        if result.get("downloaded"):
            if result.get("url"):
                summary["encoders_downloaded"] += 1
            else:
                summary["encoders_present"] += 1
        elif "error" in result:
            summary["encoders_failed"] += 1
        summary["details"].append(result)

    if args.json:
        print(json.dumps(summary, indent=2))
    else:
        print(
            f"coreml encoders in {summary['models_dir']}: "
            f"present={summary['encoders_present']} "
            f"downloaded={summary['encoders_downloaded']} "
            f"failed={summary['encoders_failed']}"
        )
    return 0 if summary["encoders_failed"] == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
