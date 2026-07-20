// v0.6.23+ — pricing 页面静态校验 (Node 内置 test runner)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as url from 'node:url';
import * as vm from 'node:vm';
import ts from 'typescript';

const HERE = path.dirname(url.fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '..', '..');
const ZH = path.join(ROOT, 'src', 'i18n', 'locales', 'zh.ts');
const EN = path.join(ROOT, 'src', 'i18n', 'locales', 'en.ts');
const PRICING = path.join(ROOT, 'src', 'app', 'pricing', 'page.tsx');

function loadDict(file) {
  const src = fs.readFileSync(file, 'utf8');
  const replaced = src.replace(/export const \w+: Record<string, unknown> =/, 'globalThis.__dict =');
  const out = ts.transpileModule(replaced, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  }).outputText;
  const ctx = vm.createContext({ globalThis: {} });
  vm.runInContext(out, ctx);
  return ctx.globalThis.__dict;
}

function keys(o, prefix = '') {
  if (!o || typeof o !== 'object') return prefix ? [prefix] : [];
  return Object.entries(o).flatMap(([k, v]) =>
    v && typeof v === 'object' ? keys(v, prefix + k + '.') : [prefix + k]
  );
}

function diff(a, b) {
  const sb = new Set(b);
  const sa = new Set(a);
  return { onlyA: a.filter((k) => !sb.has(k)), onlyB: b.filter((k) => !sa.has(k)) };
}

function pricingHardcodedChinese() {
  const src = fs.readFileSync(PRICING, 'utf8');
  const lines = src.split('\n');
  const out = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (/^\s*(\/\/|\/\*|\*)/.test(line)) continue;       // line comment
    if (/^\s*{\/\*.*\*\/}\s*$/.test(line)) continue;     // JSX block comment (whole line)
    if (/[一-鿿]/.test(line)) out.push({ line: i + 1, text: line.trim().slice(0, 80) });
  }
  return out;
}

test('zh / en keys are 1:1', () => {
  const zh = keys(loadDict(ZH));
  const en = keys(loadDict(EN));
  const d = diff(zh, en);
  assert.deepEqual(d, { onlyA: [], onlyB: [] });
  assert.ok(zh.length > 600, 'zh keys too few: ' + zh.length);
});

test('pricing page has no visible hardcoded Chinese', () => {
  const hits = pricingHardcodedChinese();
  assert.deepEqual(hits, [], '硬编码中文:\n' + JSON.stringify(hits, null, 2));
});

test('pricing tier keys exist in both locales', () => {
  const zh = loadDict(ZH);
  const en = loadDict(EN);
  const required = [
    'pricing.tier_anonymous',
    'pricing.tier_free',
    'pricing.tier_pro',
    'pricing.tier_anonymous_subtitle',
    'pricing.tier_free_subtitle',
    'pricing.tier_pro_subtitle',
    'pricing.tier_anonymous_cta',
    'pricing.tier_free_cta',
    'pricing.tier_pro_cta',
    'pricing.tier_anonymous_features',
    'pricing.tier_free_features',
    'pricing.tier_pro_features',
    'pricing.beta_title',
    'pricing.beta_step_1',
    'pricing.beta_step_2',
    'pricing.beta_step_3',
    'pricing.beta_step_4',
    'pricing.compare_title',
    'pricing.faq_title',
    'pricing.privacy_strong',
    'pricing.privacy_banner_body',
    'pricing.privacy_strong_network',
    'pricing.footer_privacy',
    'pricing.footer_terms',
    'pricing.footer_download',
    'pricing.footer_copyright',
  ];
  for (const key of required) {
    const path = key.split('.');
    let z = zh, e = en;
    for (const p of path) {
      z = z?.[p];
      e = e?.[p];
    }
    assert.ok(z !== undefined, `zh missing: ${key}`);
    assert.ok(e !== undefined, `en missing: ${key}`);
  }
});
