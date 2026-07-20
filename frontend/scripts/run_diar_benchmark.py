import json
import sys
from pathlib import Path
import soundfile as sf
ROOT=Path(__file__).resolve().parents[1]
sys.path.insert(0,str(ROOT/'src-tauri'/'scripts'))
import diar
manifest=json.loads((ROOT/'benchmarks'/'diarization'/'manifest.json').read_text())
for item in manifest['cases']:
    audio,sr=sf.read(ROOT/'benchmarks'/'diarization'/item['audio'],dtype='float32')
    result=diar.process_diarization(audio,sr)
    output=ROOT/'benchmarks'/'diarization'/'reports'/f"{item['id']}.json"
    output.write_text(json.dumps(result,ensure_ascii=False,indent=2))
    print(item['id'], {'expected':item['expected_speakers'],'actual':result['num_speakers'],'segments':len(result['segments'])})
