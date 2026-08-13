// 自测总入口
process.env.NODE_ENV = 'production';
const fs = require('fs');
const { spawnSync } = require('child_process');
const here = '/Users/wangwei/Documents/离线会记/selftest';
const selftest = '/tmp/selftest_node';

if (!fs.existsSync(selftest + '/node_modules')) {
  console.log('[setup] 正在装测试依赖到 /tmp/selftest_node ...');
  fs.mkdirSync(selftest, { recursive: true });
  fs.writeFileSync(selftest + '/package.json', '{}');
  const r = spawnSync('npm', ['install', '--silent', 'react@18.3.1', 'react-dom@18.3.1', 'react-test-renderer@18.3.1', 'fake-indexeddb'], { cwd: selftest });
  if (r.status !== 0) { console.error('[setup] 装依赖失败'); process.exit(1); }
}

console.log('====== T 系列 (server-side render + 源码痕迹) ======');
const t = spawnSync('node', [here + '/run.cjs'], { stdio: 'inherit' });

console.log('\n====== U 系列 (真 React 18 commit + fake-indexeddb) ======');
const u = spawnSync('node', [selftest + '/U_v3.cjs'], { stdio: 'inherit', env: { ...process.env, NODE_PATH: selftest + '/node_modules' } });

console.log('\n====== C/D 系列 (Nano 设置 + cam++ 长会议保护) ======');
const cd = spawnSync('node', [selftest + '/run_cd.cjs'], { stdio: 'inherit' });

console.log('\n====== C 商业化 (quota + admin + 法律页) ======');
const c = spawnSync('node', [selftest + '/C_commercial.cjs'], {
  stdio: 'inherit',
  env: { ...process.env, LIXIANHUIJI_DEV_MODE: '1' }
});



const ok = (t.status === 0 && u.status === 0 && cd.status === 0 && c.status === 0);
console.log('\n====== 综合 ======');
console.log('T 系列:', t.status === 0 ? '✓ 通过' : '✗ 失败');
console.log('U 系列:', u.status === 0 ? '✓ 通过' : '✗ 失败');
console.log('C/D 系列:', cd.status === 0 ? '✓ 通过' : '✗ 失败');
console.log('C 商业化:', c.status === 0 ? '✓ 通过' : '✗ 失败');
console.log(ok ? '\n✅ 所有自测通过, 修复确认' : '\n❌ 有失败, 检查报告');
process.exit(ok ? 0 : 1);
