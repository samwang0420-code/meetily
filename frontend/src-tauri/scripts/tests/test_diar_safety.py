import importlib.util
import unittest
from pathlib import Path
import numpy as np

SCRIPT=Path(__file__).resolve().parents[1]/'diar.py'
spec=importlib.util.spec_from_file_location('diar_under_test',SCRIPT)
module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)

class DiarSafetyTests(unittest.TestCase):
    def test_long_audio_returns_without_loading_model(self):
        previous=module.is_available
        module.is_available=lambda: True
        try:
            result=module.process_diarization(np.zeros(301*16000,dtype=np.float32),16000)
        finally:
            module.is_available=previous
        self.assertEqual(result.get('warning'),'audio_too_long')
        self.assertIsNone(result['num_speakers'])
        self.assertEqual(result['segments'],[])

if __name__=='__main__': unittest.main()
