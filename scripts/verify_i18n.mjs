import fs from 'node:fs';
import ts from '../frontend/node_modules/typescript/lib/typescript.js';

function readLocale(file, variable) {
  file = file.pathname;
  const source = fs.readFileSync(file, 'utf8');
  const sourceFile = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true);
  let initializer;
  sourceFile.forEachChild(node => {
    if (!ts.isVariableStatement(node)) return;
    for (const declaration of node.declarationList.declarations) {
      if (declaration.name.getText(sourceFile) === variable) initializer = declaration.initializer;
    }
  });
  if (!initializer || !ts.isObjectLiteralExpression(initializer)) throw new Error(`Missing locale object: ${variable}`);
  const keys = new Set();
  function walk(object, prefix = '') {
    for (const property of object.properties) {
      if (!ts.isPropertyAssignment(property)) continue;
      const raw = property.name.getText(sourceFile);
      const key = raw.replace(/^['"]|['"]$/g, '');
      const path = prefix ? `${prefix}.${key}` : key;
      if (ts.isObjectLiteralExpression(property.initializer)) walk(property.initializer, path);
      else keys.add(path);
    }
  }
  walk(initializer);
  return keys;
}

const zh = readLocale(new URL('../frontend/src/i18n/locales/zh.ts', import.meta.url), 'zh');
const en = readLocale(new URL('../frontend/src/i18n/locales/en.ts', import.meta.url), 'en');
const onlyZh = [...zh].filter(key => !en.has(key));
const onlyEn = [...en].filter(key => !zh.has(key));
if (onlyZh.length || onlyEn.length) {
  console.error(JSON.stringify({ onlyZh, onlyEn }, null, 2));
  process.exit(1);
}
console.log(`i18n OK: ${zh.size} recursive keys aligned`);
