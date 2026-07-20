import json, subprocess, time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]; base=ROOT/'benchmarks'/'asr'
manifest=json.loads((base/'manifest.json').read_text())
models=[('funasr-nano','funasr-nano-zh'),('sensevoice','sensevoice-zh')]
for output_name,model in models:
    proc=subprocess.Popen(['python3',str(ROOT/'src-tauri'/'scripts'/'sherpa_asr.py')],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,bufsize=1)
    runtime=[]
    for item in manifest['cases']:
        request={'id':item['id'],'action':'transcribe','model':model,'audio_path':str(base/'audio'/f"{item['id']}.wav"),'sample_rate':16000,'language':'zh','hotwords_pack':item['domain'],'hotwords_custom':','.join(item['terms'])}
        started=time.time(); proc.stdin.write(json.dumps(request,ensure_ascii=False)+'\n'); proc.stdin.flush(); response=json.loads(proc.stdout.readline()); wall=round(time.time()-started,3)
        if not response.get('ok'): raise RuntimeError(f"{model} {item['id']}: {response}")
        (base/'hypotheses'/output_name/f"{item['id']}.txt").write_text(response.get('text','')+'\n')
        runtime.append({'id':item['id'],'model':response.get('model'),'wall_seconds':wall,'decode_ms':response.get('decode_ms'),'audio_seconds':response.get('audio_seconds'),'chars':len(response.get('text',''))})
        print(output_name,item['id'],runtime[-1])
    proc.stdin.close(); proc.wait(timeout=30); stderr=proc.stderr.read()
    (base/'runtime'/f'{output_name}.json').write_text(json.dumps(runtime,ensure_ascii=False,indent=2))
    (base/'runtime'/f'{output_name}.stderr.log').write_text(stderr)
