import fs from 'node:fs';
import path from 'node:path';
const root=path.dirname(new URL(import.meta.url).pathname);
const manifest=JSON.parse(fs.readFileSync(path.join(root,'manifest.json'),'utf8'));
const overlap=(a,b)=>Math.max(0,Math.min(a.end,b.end)-Math.max(a.start,b.start));
const results=[];
for(const item of manifest.cases){
 const truth=JSON.parse(fs.readFileSync(path.join(root,item.segments),'utf8')).segments;
 const reportPath=path.join(root,'reports',`${item.id}.json`);
 if(!fs.existsSync(reportPath)){results.push({id:item.id,status:'missing'});continue;}
 const prediction=JSON.parse(fs.readFileSync(reportPath,'utf8'));
 const mapping=new Map();
 for(const expected of truth){
  const predicted=prediction.segments.reduce((best,current)=>overlap(expected,current)>overlap(expected,best||{start:0,end:0})?current:best,null);
  if(!predicted)continue;
  const key=expected.speaker; const counts=mapping.get(key)||new Map(); counts.set(predicted.speaker,(counts.get(predicted.speaker)||0)+1); mapping.set(key,counts);
 }
 const assigned=new Map([...mapping].map(([speaker,counts])=>[speaker,[...counts].sort((a,b)=>b[1]-a[1])[0]?.[0]]));
 let correct=0,covered=0;
 for(const expected of truth){
  const predicted=prediction.segments.reduce((best,current)=>overlap(expected,current)>overlap(expected,best||{start:0,end:0})?current:best,null);
  if(!predicted||overlap(expected,predicted)<=0)continue; covered++; if(assigned.get(expected.speaker)===predicted.speaker)correct++;
 }
 results.push({id:item.id,status:'ok',expected_speakers:item.expected_speakers,actual_speakers:prediction.num_speakers,segments:prediction.segments.length,covered_segments:covered,total_segments:truth.length,speaker_purity:covered?correct/covered:0});
}
const ok=results.filter(x=>x.status==='ok');
const summary={cases:ok.length,speaker_count_pass:ok.filter(x=>x.expected_speakers===x.actual_speakers).length,average_purity:ok.length?ok.reduce((s,x)=>s+x.speaker_purity,0)/ok.length:null,results};
fs.writeFileSync(path.join(root,'reports','summary.json'),JSON.stringify(summary,null,2));console.log(JSON.stringify(summary,null,2));
if(ok.some(x=>x.expected_speakers!==x.actual_speakers||x.speaker_purity<0.9))process.exitCode=1;
