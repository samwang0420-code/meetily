import importlib.util
import os
import tempfile
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

if __name__ == "__main__":
    unittest.main()
