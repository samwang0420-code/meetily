import fs from 'node:fs';
const base=new URL('./',import.meta.url); const read=name=>JSON.parse(fs.readFileSync(new URL(`reports/${name}.json`,base)));
const runtime=name=>JSON.parse(fs.readFileSync(new URL(`runtime/${name}.json`,base)));
const nano=read('funasr-nano'), sense=read('sensevoice'), nr=runtime('funasr-nano'), sr=runtime('sensevoice');
const avg=(rows,key)=>rows.reduce((s,x)=>s+(x[key]||0),0)/rows.length;
const cerRelativeGain=(sense.summary.cer-nano.summary.cer)/sense.summary.cer;
const report={
 criteria:{minimum_relative_cer_gain:0.10,minimum_term_recall:0.90,maximum_decode_slowdown:3},
 metrics:{nano_cer:nano.summary.cer,sensevoice_cer:sense.summary.cer,relative_cer_gain:cerRelativeGain,nano_term_recall:nano.summary.term_recall,sensevoice_term_recall:sense.summary.term_recall,nano_avg_decode_ms:avg(nr,'decode_ms'),sensevoice_avg_decode_ms:avg(sr,'decode_ms'),decode_slowdown:avg(nr,'decode_ms')/avg(sr,'decode_ms')},
 decision:'keep_sensevoice_default',reasons:[]
};
if(cerRelativeGain<report.criteria.minimum_relative_cer_gain)report.reasons.push('Nano CER 相对改善不足 10%');
if(nano.summary.term_recall<report.criteria.minimum_term_recall)report.reasons.push('Nano 术语召回低于 90%');
if(report.metrics.decode_slowdown>report.criteria.maximum_decode_slowdown)report.reasons.push('Nano 解码耗时超过 SenseVoice 3 倍');
if(!report.reasons.length)report.decision='promote_nano_default';
fs.writeFileSync(new URL('reports/model-decision.json',base),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
