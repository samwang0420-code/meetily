"""Speaker Diarization (v0.6.14+) — 离线会记 B 方案第二步

使用 sherpa-onnx 1.13.4 的 OfflineSpeakerDiarization,提供:
  - num_speakers: 检测到的说话人数
  - num_segments: 说话人切换段数

完整时间戳 segments 在 sherpa-onnx 1.13.4 Python binding 中无法 enumerate
(只暴露 num_segments / num_speakers / sort_by_*),所以本模块只暴露这两个数字。
后续 v0.6.15 可以:
  - 改用 sherpa-onnx >= 1.14 (若有 enumerate binding)
  - 或者自己用 pyannote + campplus 组合手工实现 segment + cluster

模型路径:
  优先检查 $MODELS_ROOT/sherpa-diarize/
  fallback 到 /tmp 临时目录(开发者可用)

调用入口:
  is_available() -> bool
  count_speakers(audio_np: np.ndarray, sample_rate: int) -> int | None
"""
import os
import threading
import time
import sys


# v0.6.10+: 长会议保护阈值 — 超过这个秒数主动跳过 cam++ (4 人 1 小时会变成 5 人误识)
# 评测数据: benchmarks/diarization/reports/long-audio-decision.json
#   audio_seconds=3619, expected=4, actual=5, wall_seconds=446.87, peak_rss=360MB
MAX_DIAR_AUDIO_SECONDS = 5400  # 商业会议上限 90 分钟；更长内容明确降级
WINDOW_SECONDS = 180.0
WINDOW_OVERLAP_SECONDS = 20.0
GLOBAL_CLUSTER_THRESHOLD = 0.76


def _diar_threads():
    try:
        return max(1, min(8, int(os.environ.get("LIXIANHUIJI_DIAR_THREADS", "2"))))
    except ValueError:
        return 2

# 模型路径 (按优先级查找)
_MODELS_CANDIDATES = [
    os.path.expanduser("~/Library/Application Support/cn.lixianhuiji.app/models/sherpa-diarize"),
    "/tmp/diar_models/sherpa-diarize",
    "/tmp",
]


def _find_model_path():
    """找存在的 segmentation + embedding 模型路径"""
    for root in _MODELS_CANDIDATES:
        seg = os.path.join(root, "segmentation", "model.int8.onnx")
        emb = os.path.join(root, "embedding", "model.onnx")
        if os.path.exists(seg) and os.path.exists(emb):
            return root, seg, emb
    # 7/8 临时位置 fallback (旧开发位置, 保留兼容)
    alt_seg = "/tmp/diar_models/sherpa-onnx-pyannote-segmentation-3-0/model.int8.onnx"
    alt_emb = "/tmp/campplus.onnx"
    if os.path.exists(alt_seg) and os.path.exists(alt_emb):
        return "/tmp", alt_seg, alt_emb
    # 未找到时, 触发懒加载下载 (一次性, 锁保护)
    return _lazy_ensure_models()


_LAZY_LOCK = threading.Lock()
_LAZY_RESULT = None


def _lazy_ensure_models():
    """Trigger non-blocking diarization model download when missing.

    Returns (root, seg, emb) or (None, None, None). The first invocation kicks
    off an async download; subsequent invocations read the same snapshot.
    """
    global _LAZY_RESULT
    if _LAZY_RESULT is not None:
        return _LAZY_RESULT
    with _LAZY_LOCK:
        if _LAZY_RESULT is not None:
            return _LAZY_RESULT
        # First try synchronous check; if present, cache and return.
        for root in _MODELS_CANDIDATES:
            seg = os.path.join(root, "segmentation", "model.int8.onnx")
            emb = os.path.join(root, "embedding", "model.onnx")
            if os.path.exists(seg) and os.path.exists(emb):
                _LAZY_RESULT = (root, seg, emb)
                return _LAZY_RESULT
        # Not present → trigger async download (do NOT block the audio thread).
        try:
            from diar_download import ensure_models_async
            result = ensure_models_async(progress_cb=lambda msg: sys.stderr.write(f"[diar] {msg}\n"))
            sys.stderr.write(f"[diar] async download kicked off: {result}\n")
        except Exception as exc:  # noqa: BLE001
            sys.stderr.write(f"[diar] async download trigger failed: {exc}\n")
        _LAZY_RESULT = (None, None, None)
        return _LAZY_RESULT


_DIAR_PATHS = None  # cache

def _paths():
    global _DIAR_PATHS
    if _DIAR_PATHS is not None:
        return _DIAR_PATHS
    _DIAR_PATHS = _find_model_path()
    return _DIAR_PATHS


def is_available() -> bool:
    """检查模型文件是否就绪"""
    _, seg, emb = _paths()
    return seg is not None and emb is not None


def ensure_models_with_status(progress_cb=None) -> dict:
    """Lazy-download the diarization models when they are missing.

    Returns a status dict from diar_download.ensure_models; on failure the ASR
    pipeline keeps working without speaker separation and the UI can surface a
    friendly retry banner.
    """
    try:
        from diar_download import ensure_models
    except ImportError:
        return {"ok": False, "error": "downloader_not_found", "tried": []}
    try:
        return ensure_models(progress_cb=progress_cb)
    except Exception as exc:  # noqa: BLE001 - never let downloader crash ASR
        return {"ok": False, "error": f"downloader_exception:{exc}", "tried": []}


def _build_diar(seg_path, emb_path):
    """构造 OfflineSpeakerDiarization (lazy, 一次性)"""
    import sherpa_onnx
    pyannote_cfg = sherpa_onnx.OfflineSpeakerSegmentationPyannoteModelConfig(model=seg_path)
    seg_model_cfg = sherpa_onnx.OfflineSpeakerSegmentationModelConfig(
        pyannote=pyannote_cfg, num_threads=_diar_threads(), debug=False, provider='cpu'
    )
    emb_cfg = sherpa_onnx.SpeakerEmbeddingExtractorConfig(
        model=emb_path, num_threads=_diar_threads(), provider='cpu',
    )
    cluster_cfg = sherpa_onnx.FastClusteringConfig(num_clusters=-1, threshold=0.4)
    diar_cfg = sherpa_onnx.OfflineSpeakerDiarizationConfig(
        segmentation=seg_model_cfg,
        embedding=emb_cfg,
        clustering=cluster_cfg,
        min_duration_on=0.3, min_duration_off=0.5,
    )
    return sherpa_onnx.OfflineSpeakerDiarization(diar_cfg)


_DIAR_LOCK = threading.Lock()
_DIAR_OBJ = None


def _get_diar():
    global _DIAR_OBJ
    if _DIAR_OBJ is not None:
        return _DIAR_OBJ
    _, seg, emb = _paths()
    if not seg or not emb:
        return None
    with _DIAR_LOCK:
        if _DIAR_OBJ is not None:
            return _DIAR_OBJ
        try:
            t0 = time.time()
            _DIAR_OBJ = _build_diar(seg, emb)
            sys.stderr.write(f"[diar] loaded in {time.time()-t0:.2f}s\n")
            return _DIAR_OBJ
        except Exception as e:
            sys.stderr.write(f"[diar] load failed: {e}\n")
            return None


def count_speakers(audio_np, sample_rate: int = 16000):
    """返回识别出的说话人数量 (None = 失败/不启用)

    audio_np: float32 numpy array, range [-1, 1]

    长会议走 process_diarization 的窗口化全局聚类；超过 90 分钟才降级。
    """
    if not is_available():
        return None
    duration = audio_np.shape[-1] / float(sample_rate) if hasattr(audio_np, 'shape') else 0
    if duration > 300:
        return process_diarization(audio_np, sample_rate).get("num_speakers")
    diar = _get_diar()
    if diar is None:
        return None
    try:
        audio = audio_np.astype('float32') if audio_np.dtype != 'float32' else audio_np
        if audio.ndim > 1:
            audio = audio.mean(axis=1)
        result = diar.process(audio)
        n_speakers = result.num_speakers
        return int(n_speakers)
    except Exception as e:
        sys.stderr.write(f"[diar] diarization failed: {e}\n")
        return None


def _segments_from_result(diar, audio, sample_rate: int):
    raw_result = diar.process(audio)
    n_speakers = int(raw_result.num_speakers) if hasattr(raw_result, "num_speakers") else 0
    segments = []
    for item in raw_result.sort_by_start_time():
        start, end = float(item.start), float(item.end)
        duration = float(item.duration) if hasattr(item, "duration") else end - start
        if duration >= 0.3 and end > start:
            segments.append({"start": start, "end": end, "speaker": int(item.speaker), "duration": duration, "text": ""})
    return n_speakers, segments


def _embedding_extractor():
    import sherpa_onnx
    _, _, emb = _paths()
    config = sherpa_onnx.SpeakerEmbeddingExtractorConfig(model=emb, num_threads=_diar_threads(), provider="cpu")
    return sherpa_onnx.SpeakerEmbeddingExtractor(config)


def _speaker_embedding(extractor, audio, sample_rate: int, segment):
    import numpy as np
    clip = audio[int(segment["start"] * sample_rate):int(segment["end"] * sample_rate)]
    if len(clip) < sample_rate:
        return None
    stream = extractor.create_stream()
    stream.accept_waveform(sample_rate, clip)
    stream.input_finished()
    if not extractor.is_ready(stream):
        return None
    vector = np.asarray(extractor.compute(stream), dtype=np.float32)
    norm = float(np.linalg.norm(vector))
    return vector / norm if norm > 0 else None


def _cosine(left, right):
    import numpy as np
    return float(np.dot(left, right))


def _map_window_speakers(extractor, audio, sample_rate, segments, centroids, counts, threshold=0.72):
    local_vectors = {}
    for speaker in sorted({item["speaker"] for item in segments}):
        candidates = sorted((item for item in segments if item["speaker"] == speaker), key=lambda item: item["duration"], reverse=True)
        for candidate in candidates:
            vector = _speaker_embedding(extractor, audio, sample_rate, candidate)
            if vector is not None:
                local_vectors[speaker] = vector
                break
    mapping = {}
    used_global = set()
    for local, vector in local_vectors.items():
        scored = sorted(((index, _cosine(vector, centroid)) for index, centroid in enumerate(centroids) if index not in used_global), key=lambda item: item[1], reverse=True)
        if scored and scored[0][1] >= threshold:
            global_id = scored[0][0]
            count = counts[global_id]
            centroids[global_id] = (centroids[global_id] * count + vector) / (count + 1)
            centroids[global_id] /= max(float((centroids[global_id] ** 2).sum()) ** 0.5, 1e-6)
            counts[global_id] += 1
        else:
            global_id = len(centroids)
            centroids.append(vector)
            counts.append(1)
        mapping[local] = global_id
        used_global.add(global_id)
    return mapping


def _window_speaker_vectors(extractor, audio, sample_rate, segments, max_clips=3):
    import numpy as np
    vectors = {}
    for speaker in sorted({item["speaker"] for item in segments}):
        candidates = sorted(
            (item for item in segments if item["speaker"] == speaker),
            key=lambda item: item["duration"],
            reverse=True,
        )
        samples = []
        for candidate in candidates:
            vector = _speaker_embedding(extractor, audio, sample_rate, candidate)
            if vector is not None:
                samples.append(vector)
            if len(samples) >= max_clips:
                break
        if samples:
            centroid = np.mean(samples, axis=0)
            norm = float(np.linalg.norm(centroid))
            if norm > 0:
                vectors[speaker] = centroid / norm
    return vectors


def _cluster_local_speakers(records, threshold=GLOBAL_CLUSTER_THRESHOLD):
    import numpy as np
    clusters = [{"members": [index], "centroid": record["vector"].copy()} for index, record in enumerate(records)]
    while len(clusters) > 1:
        best = None
        for left in range(len(clusters)):
            for right in range(left + 1, len(clusters)):
                score = _cosine(clusters[left]["centroid"], clusters[right]["centroid"])
                if score >= threshold and (best is None or score > best[0]):
                    best = (score, left, right)
        if best is None:
            break
        _, left, right = best
        members = clusters[left]["members"] + clusters[right]["members"]
        centroid = np.mean([records[index]["vector"] for index in members], axis=0)
        centroid /= max(float(np.linalg.norm(centroid)), 1e-6)
        clusters[left] = {"members": members, "centroid": centroid}
        clusters.pop(right)

    mapping = {}
    for global_id, cluster in enumerate(sorted(clusters, key=lambda item: min(item["members"]))):
        for index in cluster["members"]:
            mapping[(records[index]["window"], records[index]["speaker"])] = global_id
    return mapping


def process_diarization(audio_np, sample_rate: int = 16000):
    """返回跨窗口一致的 speaker segments；长音频按 4 分钟窗口处理。"""
    if not is_available():
        return {"num_speakers": None, "segments": []}
    diar = _get_diar()
    if diar is None:
        return {"num_speakers": None, "segments": []}
    try:
        audio = audio_np.astype("float32") if audio_np.dtype != "float32" else audio_np
        if audio.ndim > 1:
            audio = audio.mean(axis=1)
        duration_seconds = len(audio) / max(sample_rate, 1)
        if duration_seconds > MAX_DIAR_AUDIO_SECONDS:
            sys.stderr.write(
                f"[diar] skipped: {duration_seconds:.1f}s exceeds {MAX_DIAR_AUDIO_SECONDS}s commercial limit\n"
            )
            return {"num_speakers": None, "segments": [], "warning": "audio_too_long"}
        window_seconds = WINDOW_SECONDS
        overlap_seconds = WINDOW_OVERLAP_SECONDS
        if duration_seconds <= 300.0:
            n_speakers, segments = _segments_from_result(diar, audio, sample_rate)
            if n_speakers <= 0 or not segments:
                return {"num_speakers": None, "segments": []}
            sys.stderr.write(f"[diar] result: {n_speakers} speakers, {len(segments)} segments\n")
            return {"num_speakers": n_speakers, "segments": segments}

        extractor = _embedding_extractor()
        local_records, pending_segments = [], []
        step = window_seconds - overlap_seconds
        window_start = 0.0
        window_index = 0
        while window_start < duration_seconds:
            window_end = min(window_start + window_seconds, duration_seconds)
            chunk = audio[int(window_start * sample_rate):int(window_end * sample_rate)]
            _, local_segments = _segments_from_result(diar, chunk, sample_rate)
            vectors = _window_speaker_vectors(extractor, chunk, sample_rate, local_segments)
            for speaker, vector in vectors.items():
                local_records.append({"window": window_index, "speaker": speaker, "vector": vector})
            keep_from = 0.0 if window_index == 0 else overlap_seconds / 2
            keep_until = (window_end - window_start) if window_end >= duration_seconds else (window_end - window_start) - overlap_seconds / 2
            for item in local_segments:
                midpoint = (item["start"] + item["end"]) / 2
                if midpoint < keep_from or midpoint >= keep_until or item["speaker"] not in vectors:
                    continue
                pending_segments.append({**item, "start": item["start"] + window_start, "end": item["end"] + window_start, "window": window_index})
            window_start += step
            window_index += 1
        mapping = _cluster_local_speakers(local_records)
        merged = []
        for item in pending_segments:
            global_id = mapping.get((item["window"], item["speaker"]))
            if global_id is None:
                continue
            merged.append({key: value for key, value in {**item, "speaker": global_id}.items() if key != "window"})
        merged.sort(key=lambda item: (item["start"], item["end"]))
        num_speakers = len(set(mapping.values()))
        sys.stderr.write(f"[diar] global-cluster result: {num_speakers} speakers, {len(merged)} segments, windows={window_index}\n")
        return {"num_speakers": num_speakers or None, "segments": merged, "windowed": True, "windows": window_index, "global_clustering": True}
    except Exception as error:
        sys.stderr.write(f"[diar] process_diarization failed: {error}\n")
        return {"num_speakers": None, "segments": []}
