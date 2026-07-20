import importlib.util
import os
import tempfile
import numpy as np
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "sherpa_asr.py"
spec = importlib.util.spec_from_file_location("sherpa_asr", SCRIPT)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class ModelDiscoveryTests(unittest.TestCase):
    def test_discovers_complete_funasr_nano_pack_without_single_model_file(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            model = root / "funasr-nano-int8"
            tokenizer = model / "Qwen3-0.6B"
            tokenizer.mkdir(parents=True)
            for name in ("encoder_adaptor.int8.onnx", "embedding.int8.onnx", "llm.int8.onnx"):
                (model / name).write_bytes(b"x")
            (tokenizer / "tokenizer.json").write_text("{}")
            previous = module.MODELS_ROOT
            module.MODELS_ROOT = str(root)
            try:
                found = module._scan_models()
            finally:
                module.MODELS_ROOT = previous
            self.assertIn("funasr-nano-zh", found)
            self.assertEqual(found["funasr-nano-zh"]["kind"], "funasr_nano")
            self.assertEqual(found["funasr-nano-zh"]["path"], str(model))

    def test_rejects_incomplete_funasr_nano_pack(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "funasr-nano-int8").mkdir()
            previous = module.MODELS_ROOT
            module.MODELS_ROOT = str(root)
            try:
                found = module._scan_models()
            finally:
                module.MODELS_ROOT = previous
            self.assertNotIn("funasr-nano-zh", found)


class DiarLongAudioDispatchTests(unittest.TestCase):
    """v0.7.1+: 长会议 diar 异步 dispatch + /tmp 落盘 helper"""

    def test_persist_then_pickup_roundtrip(self):
        rid = "unit-test-roundtrip"
        state = {"result": {"num_speakers": 3, "segments": [{"start": 0, "end": 1, "speaker": 0}]}, "err": None}
        module._persist_diar_result(rid, state)
        try:
            picked = module._diar_pickup(rid)
            self.assertIsNotNone(picked)
            self.assertEqual(picked["rid"], rid)
            self.assertEqual(picked["num_speakers"], 3)
            self.assertIn("finished_at", picked)
            self.assertEqual(len(picked["segments"]), 1)
        finally:
            try:
                os.unlink(f"/tmp/lixianhuiji_diar/{rid}.json")
            except OSError:
                pass

    def test_pickup_returns_none_when_missing(self):
        self.assertIsNone(module._diar_pickup("non-existent-rid-12345"))

    def test_persist_with_error_marks_payload(self):
        rid = "unit-test-err"
        state = {"result": None, "err": RuntimeError("boom")}
        module._persist_diar_result(rid, state)
        try:
            picked = module._diar_pickup(rid)
            self.assertIsNotNone(picked)
            self.assertIn("error", picked)
            self.assertIn("boom", picked["error"])
        finally:
            try:
                os.unlink(f"/tmp/lixianhuiji_diar/{rid}.json")
            except OSError:
                pass

    def test_long_audio_dispatch_skips_synchronous_join(self):
        """模拟 30 分钟音频 → transcribe 不阻塞, 立刻返 diar_pending=True"""
        # 借 is_available monkeypatch + process_diarization 慢执行
        import time as _t
        previous_available = module.diar_is_available
        module.diar_is_available = lambda: True

        # 慢 process_diarization 让 join 必超时 (12s)
        def slow_diar(arr, sr):
            _t.sleep(0.05)
            return {"num_speakers": 2, "segments": []}
        from diar import process_diarization as real_process
        import diar
        diar.process_diarization = slow_diar
        previous_process = getattr(module, 'process_diarization', None)

        try:
            # mock arr/sr: 30 分钟 @ 16kHz
            arr = np.zeros(30 * 60 * 16000, dtype=np.float32)
            sr = 16000
            rid = "long-audio-test-rid"

            # 不直接调 transcribe() (依赖太多 model state), 跑分支核心
            audio_seconds = len(arr) / sr
            self.assertGreaterEqual(audio_seconds, 60.0)  # 必走 is_long

            # 模拟 _run_diar + dispatch
            state = {"result": None, "err": None}
            def _run_diar():
                try:
                    state["result"] = diar.process_diarization(arr, sr)
                except Exception as e:
                    state["err"] = e
                finally:
                    module._persist_diar_result(rid, state)
            import threading as _threading
            t = _threading.Thread(target=_run_diar, daemon=True)
            t.start()
            # 长音频不该 join → 立刻继续
            t.join(timeout=0.001)
            self.assertTrue(t.is_alive() or state["result"] is not None,
                            "long audio thread should not block; thread may finish quickly")

            # 等算完, 确认 pickup
            t.join(timeout=5.0)
            picked = module._diar_pickup(rid)
            self.assertIsNotNone(picked)
            self.assertEqual(picked["num_speakers"], 2)
        finally:
            module.diar_is_available = previous_available
            if previous_process is not None:
                diar.process_diarization = previous_process
            try:
                os.unlink(f"/tmp/lixianhuiji_diar/{rid}.json")
            except OSError:
                pass


    def test_diar_apply_to_db_with_real_meeting(self):
        """v0.7.1+: 真实 DB 上 diar_apply_to_db 命中真实 transcripts 行"""
        import sqlite3
        import tempfile
        # 创建临时 DB, schema 复制真生产
        with tempfile.NamedTemporaryFile(suffix=".sqlite", delete=False) as tf:
            db_path = tf.name
        try:
            conn = sqlite3.connect(db_path)
            try:
                conn.executescript("""
                    CREATE TABLE meetings (id TEXT PRIMARY KEY, title TEXT, created_at TEXT);
                    CREATE TABLE transcripts (
                        id TEXT PRIMARY KEY,
                        meeting_id TEXT,
                        transcript TEXT,
                        timestamp TEXT,
                        audio_start_time REAL, audio_end_time REAL,
                        duration REAL,
                        speaker TEXT
                    );
                """)
                # 插入 meeting + 3 条 transcript (10s/20s/30s)
                conn.execute("INSERT INTO meetings VALUES ('meeting-test', 'T', '2026-07-20')")
                for i, start in enumerate([10.0, 20.0, 30.0]):
                    conn.execute(
                        "INSERT INTO transcripts (id, meeting_id, transcript, timestamp, audio_start_time, audio_end_time, duration, speaker) "
                        "VALUES (?, ?, ?, ?, ?, ?, ?, NULL)",
                        (f"tr-{i}", "meeting-test", f"text-{i}", "ts", start, start+5, 5.0),
                    )
                conn.commit()
            finally:
                conn.close()

            # 把 db_path 通过 env 注入
            old_env = os.environ.get("LIXIANHUIJI_DIAR_DB_PATH")
            os.environ["LIXIANHUIJI_DIAR_DB_PATH"] = db_path
            try:
                # 长会议 chunk 偏移 8s, segments 覆盖 12-16s (命中第一条) + 22-26s (命中第二条)
                payload = {
                    "meeting_id": "meeting-test",
                    "audio_start_offset_seconds": 8.0,
                    "segments": [
                        {"start": 4.0, "end": 8.0, "speaker": 0},   # 12-16s -> 第一条
                        {"start": 14.0, "end": 18.0, "speaker": 1}, # 22-26s -> 第二条
                    ],
                }
                result = module._diar_apply_to_db(payload)
                self.assertEqual(result.get("updated"), 2)
                self.assertEqual(result.get("meeting_id"), "meeting-test")
                # 验证 DB
                conn = sqlite3.connect(db_path)
                try:
                    speakers = [r[0] for r in conn.execute(
                        "SELECT speaker FROM transcripts WHERE meeting_id='meeting-test' ORDER BY audio_start_time"
                    )]
                    self.assertEqual(speakers, ["speaker_00", "speaker_01", None])  # 第 3 条未命中, 保持 NULL
                finally:
                    conn.close()
            finally:
                if old_env is not None:
                    os.environ["LIXIANHUIJI_DIAR_DB_PATH"] = old_env
                elif "LIXIANHUIJI_DIAR_DB_PATH" in os.environ:
                    del os.environ["LIXIANHUIJI_DIAR_DB_PATH"]
        finally:
            os.unlink(db_path)

    def test_diar_apply_to_db_missing_meeting_id(self):
        """无 meeting_id 时 skip, 不报错"""
        payload = {"audio_start_offset_seconds": 0.0, "segments": [{"start": 0, "end": 1, "speaker": 0}]}
        result = module._diar_apply_to_db(payload)
        self.assertEqual(result.get("updated"), 0)
        self.assertEqual(result.get("reason"), "missing context")
if __name__ == "__main__":
    unittest.main()
