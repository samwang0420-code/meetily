import importlib.util
import unittest
from pathlib import Path
import numpy as np

SCRIPT=Path(__file__).resolve().parents[1]/'diar.py'
spec=importlib.util.spec_from_file_location('diar_under_test',SCRIPT)
module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)

class DiarSafetyTests(unittest.TestCase):
    def test_over_ninety_minutes_returns_without_loading_model(self):
        previous=module.is_available
        module.is_available=lambda: True
        try:
            result=module.process_diarization(np.zeros((module.MAX_DIAR_AUDIO_SECONDS+1)*10,dtype=np.float32),10)
        finally:
            module.is_available=previous
        self.assertEqual(result.get('warning'),'audio_too_long')
        self.assertIsNone(result['num_speakers'])
        self.assertEqual(result['segments'],[])

    def test_global_clustering_keeps_same_speakers_across_windows(self):
        records=[]
        base_a=np.array([1.0,0.0,0.0],dtype=np.float32)
        base_b=np.array([0.0,1.0,0.0],dtype=np.float32)
        for window in range(20):
            drift=window*0.002
            records.extend([
                {'window':window,'speaker':0,'vector':np.array([1.0,drift,0.0],dtype=np.float32)},
                {'window':window,'speaker':1,'vector':np.array([drift,1.0,0.0],dtype=np.float32)},
            ])
        for record in records:
            record['vector']/=np.linalg.norm(record['vector'])
        mapping=module._cluster_local_speakers(records)
        self.assertEqual(len(set(mapping.values())),2)
        self.assertEqual(len({mapping[(window,0)] for window in range(20)}),1)
        self.assertEqual(len({mapping[(window,1)] for window in range(20)}),1)

    def test_global_clustering_preserves_four_distinct_speakers(self):
        records=[]
        for window in range(17):
            for speaker in range(4):
                vector=np.zeros(4,dtype=np.float32)
                vector[speaker]=1.0
                vector[(speaker+1)%4]=window*0.001
                vector/=np.linalg.norm(vector)
                records.append({'window':window,'speaker':speaker,'vector':vector})
        mapping=module._cluster_local_speakers(records)
        self.assertEqual(len(set(mapping.values())),4)

if __name__=='__main__': unittest.main()
