import json, subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
base=ROOT/'benchmarks'/'asr'; manifest=json.loads((base/'manifest.json').read_text())
ffmpeg=ROOT/'src-tauri'/'binaries'/'ffmpeg-aarch64-apple-darwin'
for item in manifest['cases']:
    out=base/'audio'/f"{item['id']}.wav"
    if out.exists(): continue
    text=(base/item['reference']).read_text().strip()
    aiff=Path('/tmp')/f"{item['id']}.aiff"
    voice='Tingting' if item['domain']=='legal' else 'Shelley (中文（中国大陆）)'
    subprocess.run(['say','-v',voice,'-r','175','-o',str(aiff),text],check=True)
    subprocess.run([str(ffmpeg),'-y','-i',str(aiff),'-ac','1','-ar','16000',str(out)],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,check=True)
    print(item['id'],out)
