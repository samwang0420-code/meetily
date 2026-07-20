import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const root = path.dirname(new URL(import.meta.url).pathname);
const args = Object.fromEntries(process.argv.slice(2).map((arg, index, all) => arg.startsWith('--') ? [arg.slice(2), all[index + 1]] : null).filter(Boolean));
const hypothesisDir = path.resolve(args.hypotheses || path.join(root, 'hypotheses'));
const outputPath = path.resolve(args.output || path.join(root, 'reports', 'latest.json'));
const manifest = JSON.parse(fs.readFileSync(path.join(root, 'manifest.json'), 'utf8'));
const normalize = text => [...text.normalize('NFKC').toLowerCase()].filter(ch => !/\s/u.test(ch) && !/[，。！？；：、,.!?;:'"“”‘’（）()【】\[\]《》<>…—-]/u.test(ch)).join('');
const distance = (a, b) => {
  const previous = Array.from({length: b.length + 1}, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    let diagonal = previous[0]; previous[0] = i;
    for (let j = 1; j <= b.length; j++) {
      const above = previous[j];
      previous[j] = Math.min(previous[j] + 1, previous[j - 1] + 1, diagonal + (a[i - 1] === b[j - 1] ? 0 : 1));
      diagonal = above;
    }
  }
  return previous[b.length];
};
const readHypothesis = id => {
  const jsonPath = path.join(hypothesisDir, `${id}.json`);
  const txtPath = path.join(hypothesisDir, `${id}.txt`);
  if (fs.existsSync(jsonPath)) {
    const data = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
    const segments = Array.isArray(data) ? data : data.segments || [];
    return { text: segments.map(item => typeof item === 'string' ? item : item.text || '').join(''), segments: segments.map(item => typeof item === 'string' ? item : item.text || '') };
  }
  if (fs.existsSync(txtPath)) return { text: fs.readFileSync(txtPath, 'utf8'), segments: fs.readFileSync(txtPath, 'utf8').split(/\n+/).filter(Boolean) };
  return null;
};
const results = [];
for (const item of manifest.cases) {
  const referenceRaw = fs.readFileSync(path.join(root, item.reference), 'utf8');
  const hypothesis = readHypothesis(item.id);
  if (!hypothesis) { results.push({id:item.id, domain:item.domain, status:'missing'}); continue; }
  const reference = normalize(referenceRaw); const predicted = normalize(hypothesis.text);
  const hits = item.terms.filter(term => predicted.includes(normalize(term))).length;
  const fragments = hypothesis.segments.filter(segment => normalize(segment).length < 4).length;
  results.push({
    id:item.id, domain:item.domain, status:'ok',
    reference_chars:reference.length, hypothesis_chars:predicted.length,
    cer: reference.length ? distance(reference, predicted) / reference.length : 0,
    term_hits:hits, term_total:item.terms.length, term_recall:item.terms.length ? hits/item.terms.length : 1,
    segments:hypothesis.segments.length, fragment_ratio:hypothesis.segments.length ? fragments/hypothesis.segments.length : 1
  });
}
const completed = results.filter(item => item.status === 'ok');
const average = key => completed.length ? completed.reduce((sum,item)=>sum+item[key],0)/completed.length : null;
const report = {generated_at:new Date().toISOString(), hypothesis_dir:hypothesisDir, completed:completed.length, missing:results.length-completed.length, summary:{cer:average('cer'),term_recall:average('term_recall'),fragment_ratio:average('fragment_ratio')}, cases:results};
fs.mkdirSync(path.dirname(outputPath), {recursive:true}); fs.writeFileSync(outputPath, JSON.stringify(report,null,2));
console.log(JSON.stringify(report.summary,null,2));
if (!completed.length) process.exitCode = 2;
