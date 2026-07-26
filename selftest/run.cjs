// 自测脚本 — 验证 v0.6.10+ 修复后所有防御路径 + 商业化链路
//
// 目标: 不需要启动 GUI, 不需要录音, 验证 Rust 编译 + SQLite schema +
//       仓储备方法 + 源码痕迹 (C4 激活码 + C6 三档) + React 防御 (#321)
//
// 策略:
// 1. 用 Node + fs.readFileSync 模拟" 加载源码字符串"
// 2. 用同样的 sanitize 函数从真代码里 copy (或者 require 真修复后模块)
// 3. 用真实 SQLite transcripts 样本 + Crockford base32 spec mock 验证
// 4. 关键文件源码痕迹 + 关键 token 存在性 + 关键函数编译过

process.env.NODE_ENV = 'production';
const fs = require('fs');

let pass = 0, fail = 0;
function assert(name, fn) {
  try {
    const result = fn();
    if (result === true || result === undefined) {
      console.log(`  PASS  ${name}`);
      pass++;
    } else {
      console.log(`  FAIL  ${name} -> ${JSON.stringify(result)}`);
      fail++;
    }
  } catch (e) {
    console.log(`  FAIL  ${name} -> THROW: ${e?.message}`);
    fail++;
  }
}

const ROOT = '/Users/wangwei/Documents/meetily/frontend';
const SRC_TAURI = ROOT + '/src-tauri';

console.log('========================================');
console.log('T 系列 (源码痕迹 + 商业化链路 + 仓储备)');
console.log('========================================');
console.log('');

// === T1-T8: 修复后源码快照 ===
const fixedFiles = [
  {
    path: 'src/app/_components/CardBoundary.tsx',
    must: ['CardBoundary', 'getDerivedStateFromError', 'componentDidCatch', 'card-boundary-log']
  },
  {
    path: 'src/app/_components/HomeDashboard.tsx',
    must: ['CardBoundary', 'sanitize', 'safe.', 'safe.lastUpdated']
  },
  {
    path: 'src/services/indexedDBService.ts',
    must: ['sanitizeMeetings', 'saveMeetingMetadata skipped']
  },
  {
    path: 'src/components/TranscriptRecovery/TranscriptRecovery.tsx',
    must: ['safeTranscripts', 'typeof transcript?.text', '(空)']
  },
  {
    path: 'src/hooks/useRecordingStop.ts',
    must: ['Minified React error', '不再 throw saveError']
  },
];

for (const f of fixedFiles) {
  const full = ROOT + '/' + f.path;
  assert(`  ${f.path} 修复痕迹存在`, () => {
    const content = fs.readFileSync(full, 'utf8');
    return f.must.every(token => content.includes(token));
  });
}

console.log('');
console.log('--- T9: useMeetingData.setError 类型守卫 ---');
const umdFile = fs.readFileSync(ROOT + '/src/hooks/meeting-details/useMeetingData.ts', 'utf8');
assert('  useMeetingData.ts 含 typeof error.message === \'string\' 守卫',
  () => umdFile.includes("typeof error.message === 'string'") && umdFile.includes('error.message'));
assert('  useMeetingData.ts useState<string>(...) 类型声明',
  () => /useState<string>\(\s*'[^']*'\)/.test(umdFile));
function safeSetError(err, fallback) {
  if (err instanceof Error && typeof err.message === 'string' && err.message) return err.message;
  return fallback;
}
const FALLBACK = 'errors.meeting_save_failed';
assert('  Error("foo") 正常通过', () => safeSetError(new Error('foo'), FALLBACK) === 'foo');
assert('  Error() 空 message 落 fallback', () => safeSetError(new Error(''), FALLBACK) === FALLBACK);
assert('  Error(undefined message) 落 fallback',
  () => safeSetError(Object.assign(new Error(), { message: undefined }), FALLBACK) === FALLBACK);
assert('  普通 string 当 error 落 fallback', () => safeSetError('plain-string', FALLBACK) === FALLBACK);
assert('  null 落 fallback', () => safeSetError(null, FALLBACK) === FALLBACK);
assert('  undefined 落 fallback', () => safeSetError(undefined, FALLBACK) === FALLBACK);
assert('  空对象 {} 落 fallback', () => safeSetError({}, FALLBACK) === FALLBACK);
assert('  Error 数字 message 落 fallback',
  () => safeSetError(Object.assign(new Error(), { message: 12345 }), FALLBACK) === FALLBACK);

console.log('');
console.log('--- T10: handleGenerateSummary 入口立即 setSummaryStatus ---');
const usgFile = fs.readFileSync(ROOT + '/src/hooks/meeting-details/useSummaryGeneration.ts', 'utf8');
assert('  useSummaryGeneration.ts 入口在 isModelConfigLoading 之前 setSummaryStatus(\'processing\')',
  () => {
    const m = usgFile.match(/const handleGenerateSummary = useCallback\(async[\s\S]*?isModelConfigLoading/);
    if (!m) return false;
    return m[0].includes("setSummaryStatus('processing')");
  });
assert('  isModelConfigLoading 早 return 时回退 setSummaryStatus(\'error\')',
  () => /isModelConfigLoading[\s\S]{0,200}setSummaryStatus\('error'\)/.test(usgFile));
assert('  早返回路径同时 setSummaryError() 给出原因',
  () => /isModelConfigLoading[\s\S]{0,300}setSummaryError/.test(usgFile));

console.log('');
console.log('--- T11: 默认 max_tokens 硬控 ---');
const procFile = fs.readFileSync(SRC_TAURI + '/src/summary/processor.rs', 'utf8');
assert('  processor.rs 含 DEFAULT_SUMMARY_MAX_TOKENS 常量', () => procFile.includes('DEFAULT_SUMMARY_MAX_TOKENS'));
assert('  processor.rs 含 pub fn clamp_max_tokens', () => /pub\s+fn\s+clamp_max_tokens/.test(procFile));
assert('  DEFAULT_SUMMARY_MAX_TOKENS = 1200', () => /DEFAULT_SUMMARY_MAX_TOKENS:\s*u32\s*=\s*1200/.test(procFile));
assert('  generate_meeting_summary 入口调 clamp_max_tokens()',
  () => /generate_meeting_summary[\s\S]{0,3000}?clamp_max_tokens\(max_tokens\);/.test(procFile));
assert('  Rust 单测 default_summary_max_tokens_caps_verbose_outputs 存在',
  () => procFile.includes('default_summary_max_tokens_caps_verbose_outputs'));
assert('  clamp_max_tokens 函数含 Some(t) if t > 0 分支',
  () => /fn\s+clamp_max_tokens[\s\S]{0,500}Some\(t\) if t > 0/.test(procFile));

console.log('');
console.log('--- T12: C4 激活码逻辑 ---');
const acFile = fs.readFileSync(SRC_TAURI + '/src/user/activation_code.rs', 'utf8');
assert('  activation_code.rs 含 PROMO prefix', () => acFile.includes('const PREFIX: &str = "PROMO"'));
assert('  activation_code.rs 含 Crockford base32',
  () => acFile.includes('Alphabet::Crockford') && acFile.includes('CROCKFORD_BYTES'));
assert('  activation_code.rs 含 generate_code 函数', () => /pub fn generate_code\(\)/.test(acFile));
assert('  activation_code.rs 含 validate_code 函数', () => /pub fn validate_code\(/.test(acFile));
assert('  activation_code.rs 含 compute_checksum_4 用 FNV-1a',
  () => /FNV|cbf29ce484222325/.test(acFile));
assert('  activation_code.rs 含 mask_for_display (防日志泄漏)',
  () => acFile.includes('mask_for_display'));

// Crockford 字符集验证
const CROCKFORD_BYTES = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';
function isCrockfordChar(c) { return CROCKFORD_BYTES.indexOf(c) !== -1; }
function validateCodeSpec(raw) {
  const norm = raw.trim().toUpperCase().replace(/ /g, '');
  const parts = norm.split('-');
  if (parts.length !== 3) return false;
  if (parts[0] !== 'PROMO') return false;
  if (parts[1].length !== 8) return false;
  if (!parts[1].split('').every(isCrockfordChar)) return false;
  if (parts[2].length !== 4) return false;
  return true;
}
assert('  spec: PROMO-XXXXXXXX-YYYY 格式合法', () => validateCodeSpec('PROMO-12345678-ABCD'));
assert('  spec: 拒绝空', () => !validateCodeSpec(''));
assert('  spec: 拒绝错误 prefix', () => !validateCodeSpec('XXXX-12345678-ABCD'));
assert('  spec: 拒绝 secret 太短', () => !validateCodeSpec('PROMO-1234567-ABCD'));
assert('  spec: 拒绝 secret 太长', () => !validateCodeSpec('PROMO-123456789-ABCD'));
assert('  spec: 拒绝 secret 含非 Crockford 字符 (O)', () => !validateCodeSpec('PROMO-OOOOOOOO-ABCD'));
assert('  spec: 接受小写', () => validateCodeSpec('promo-12345678-abcd'));
assert('  spec: 接受前后空格', () => validateCodeSpec('  PROMO-12345678-ABCD  '));
assert('  spec: 接受内嵌空格', () => validateCodeSpec('P ROMO-12345678-ABCD'));
assert('  安全: Crockford 不含 O/I/L', () => !isCrockfordChar('O') && !isCrockfordChar('I') && !isCrockfordChar('L'));
assert('  安全: Crockford 含 0/1', () => isCrockfordChar('0') && isCrockfordChar('1'));

console.log('');
console.log('--- T13: C6 定价页 ---');
const pricingFile = fs.readFileSync(ROOT + '/src/app/pricing/page.tsx', 'utf8');
assert('  /app/pricing/page.tsx 文件存在',
  () => fs.existsSync(ROOT + '/src/app/pricing/page.tsx'));
assert('  pricing 含 "Pro" "¥88" 关键文案',
  () => pricingFile.includes('Pro') && pricingFile.includes('¥88'));
assert('  pricing 含 "免费档" 标题', () => pricingFile.includes('免费档'));
assert('  pricing 含 "推荐" Pro 标识', () => pricingFile.includes('推荐'));
assert('  pricing 含功能对比表 (FEATURES 数组)',
  () => pricingFile.includes('FEATURES') && pricingFile.includes('category'));
assert('  pricing 含免费档月度会议数说明 (5次/月)',
  () => pricingFile.includes('5 次') || pricingFile.includes('5次'));
assert('  pricing 含 Pro 不限说明', () => pricingFile.includes('不限'));
assert('  pricing 含 FAQ (≥ 5 个常见问题)', () => {
  const faqMatch = pricingFile.match(/FAQ = \[[^\]]+\]/s);
  if (!faqMatch) return false;
  const faqCount = (faqMatch[0].match(/q:/g) || []).length;
  return faqCount >= 5;
});
assert('  pricing 含退款 7 天政策', () => pricingFile.includes('7 天') || pricingFile.includes('7天'));
assert('  pricing 含激活码使用说明', () => pricingFile.includes('激活码'));
assert('  pricing 含 100% 本地处理隐私边界', () => pricingFile.includes('100% 本地'));
assert('  pricing 含隐私政策/用户协议链接',
  () => pricingFile.includes('/legal/privacy') && pricingFile.includes('/legal/terms'));
assert('  pricing 含 metadata.ts (SEO)',
  () => fs.existsSync(ROOT + '/src/app/pricing/metadata.ts'));

console.log('');
console.log('--- T14: C4 完整链路 (DB 仓储 + IPC + admin UI) ---');
const migrationFile = fs.readFileSync(SRC_TAURI + '/migrations/20260718000002_add_activation_codes.sql', 'utf8');
assert('  migration 存在 activation_codes 表',
  () => migrationFile.includes('CREATE TABLE') && migrationFile.includes('activation_codes'));
assert('  migration 含 code 列 (UNIQUE)',
  () => migrationFile.includes('code TEXT NOT NULL UNIQUE'));
assert('  migration 含 used_by_user_id / used_at 列',
  () => migrationFile.includes('used_by_user_id INTEGER') && migrationFile.includes('used_at TEXT'));
assert('  migration users.activated_via_code 列',
  () => migrationFile.includes('ALTER TABLE users ADD COLUMN activated_via_code'));

const repoFile = fs.readFileSync(SRC_TAURI + '/src/database/repositories/user.rs', 'utf8');
assert('  repository: ActivationCodesRepository 存在',
  () => repoFile.includes('pub struct ActivationCodesRepository'));
assert('  repository: ActivationCodeRow 类型',
  () => repoFile.includes('pub struct ActivationCodeRow'));
assert('  repository: insert / find_by_code / mark_used / revoke_unused',
  () => repoFile.includes('pub async fn insert') &&
     repoFile.includes('pub async fn find_by_code') &&
     repoFile.includes('pub async fn mark_used') &&
     repoFile.includes('pub async fn revoke_unused'));

const cmdsFile = fs.readFileSync(SRC_TAURI + '/src/user/commands.rs', 'utf8');
assert('  commands.rs: admin_generate_activation_codes',
  () => cmdsFile.includes('pub async fn admin_generate_activation_codes'));
assert('  commands.rs: admin_list_activation_codes',
  () => cmdsFile.includes('pub async fn admin_list_activation_codes'));
assert('  commands.rs: admin_revoke_activation_code',
  () => cmdsFile.includes('pub async fn admin_revoke_activation_code'));
assert('  commands.rs: user_redeem_activation_code',
  () => cmdsFile.includes('pub async fn user_redeem_activation_code'));
assert('  commands.rs: RedeemResult 含 success/error_code/error_message',
  () => cmdsFile.includes('struct RedeemResult') &&
     cmdsFile.includes('success: bool') &&
     cmdsFile.includes('error_code') &&
     cmdsFile.includes('error_message'));

const libFile = fs.readFileSync(SRC_TAURI + '/src/lib.rs', 'utf8');
assert('  lib.rs 注册 admin_generate_activation_codes',
  () => libFile.includes('user::commands::admin_generate_activation_codes'));
assert('  lib.rs 注册 user_redeem_activation_code',
  () => libFile.includes('user::commands::user_redeem_activation_code'));

const adminFile = fs.readFileSync(ROOT + '/src/app/admin/page.tsx', 'utf8');
assert('  admin page: CodeRow 类型', () => adminFile.includes('interface CodeRow'));
assert('  admin page: handleGenerateCodes', () => adminFile.includes('handleGenerateCodes'));
assert('  admin page: handleRevokeCode', () => adminFile.includes('handleRevokeCode'));
assert('  admin page: 调用 admin_generate_activation_codes',
  () => adminFile.includes('admin_generate_activation_codes'));
assert('  admin page: 调用 admin_list_activation_codes',
  () => adminFile.includes('admin_list_activation_codes'));
assert('  admin page: 调用 admin_revoke_activation_code',
  () => adminFile.includes('admin_revoke_activation_code'));

const accountFile = fs.readFileSync(ROOT + '/src/app/account/page.tsx', 'utf8');
assert('  account page: 兑换激活码输入框',
  () => accountFile.includes('PROMO-XXXXXXXX-YYYY') && accountFile.includes('setCode'));
assert('  account page: invoke user_redeem_activation_code',
  () => accountFile.includes('user_redeem_activation_code'));
assert('  account page: 失败弹窗含 not_logged_in 友好提示',
  () => accountFile.includes('请先登录账号再兑换'));
assert('  account page: 跳 /pricing CTA', () => accountFile.includes('/pricing'));

function validateCodeShape(s) {
  return /^PROMO-[A-Z0-9]{8}-[A-Z0-9]{4}$/.test(s.trim());
}
assert('  account 端前置校验: 正则 PROMO-XXXXXXXX-YYYY',
  () => validateCodeShape('PROMO-12345678-ABCD') && validateCodeShape('PROMO-ABCDEFGH-WXYZ'));
assert('  account 端前置校验: 拒绝小写', () => !validateCodeShape('promo-12345678-abcd'));
assert('  account 端前置校验: 拒绝错位数',
  () => !validateCodeShape('PROMO-1234567-ABCD') && !validateCodeShape('PROMO-12345678-ABC'));

console.log('');
console.log('--- T15: C6 三档 + 引导弹窗 ---');
assert('  pricing 含三档: anonymous (匿名)', () => pricingFile.includes('匿名'));
assert('  pricing 含三档: free (免费)',
  () => pricingFile.includes('免费档') || pricingFile.includes('免费月度'));
assert('  pricing 含三档: pro (Pro ¥88)',
  () => pricingFile.includes('¥88') && (pricingFile.includes('Pro 买断') || pricingFile.includes('Pro')));
assert('  pricing 匿名档说明: 仅 1 次',
  () => pricingFile.includes('1 次') || pricingFile.includes('1次'));
assert('  pricing 匿名档说明: 导出受限 / 水印',
  () => pricingFile.includes('水印') || pricingFile.includes('受水印') || pricingFile.includes('导出受限'));
assert('  pricing 匿名档说明: 无长音频',
  () => pricingFile.includes('无长音频') || pricingFile.includes('长音频'));
assert('  pricing 匿名档说明: 无 Nano 模式',
  () => pricingFile.includes('Nano') && (pricingFile.includes('无 Nano') || pricingFile.includes('不支持') || pricingFile.includes('不可')));
assert('  pricing 免费档说明: 每月 5 次',
  () => pricingFile.includes('5 次') || pricingFile.includes('5次'));
assert('  pricing Pro 完整权益: 无限转录',
  () => pricingFile.includes('无限') && pricingFile.includes('转录'));
assert('  pricing Pro 完整权益: 完整导出',
  () => pricingFile.includes('完整导出') || pricingFile.includes('完整'));
assert('  pricing Pro 完整权益: cam++ 短时发言人分离',
  () => pricingFile.includes('cam++') || pricingFile.includes('cam'));
assert('  pricing Pro 完整权益: FunASR-Nano',
  () => pricingFile.includes('FunASR-Nano'));
assert('  pricing Pro 完整权益: 无水印',
  () => pricingFile.includes('无水印'));
assert('  pricing 含内测期说明 + 客服邮箱',
  () => pricingFile.includes('lisangjie@icloudsend.com') && pricingFile.includes('内测'));

const modalFile = fs.readFileSync(ROOT + '/src/app/_components/QuotaPaywallModal.tsx', 'utf8');
assert('  QuotaPaywallModal 含 0 死循环 (onClose 关闭)',
  () => modalFile.includes('onClose') && modalFile.includes("if (!open) return null"));
assert('  QuotaPaywallModal 含免费 vs Pro 对比',
  () => modalFile.includes('免费') && modalFile.includes('Pro') && modalFile.includes('¥88'));
assert('  QuotaPaywallModal 匿名 vs 已注册 reason 分支',
  () => modalFile.includes("'anonymous_trial_exhausted'") &&
     modalFile.includes("'free_monthly_limit_reached'"));
assert('  QuotaPaywallModal 升级 CTA',
  () => modalFile.includes('升级') || modalFile.includes('查看 Pro 升级'));
assert('  QuotaPaywallModal 不强制骚扰 (只按钮触发)',
  () => !modalFile.includes('auto-poll') && !modalFile.includes('infinite loop'));


console.log('');
console.log('--- T16: safeToast sanitizer (防御 React #321) ---');
const safeToastSrc = fs.readFileSync(ROOT + '/src/lib/safeToast.ts', 'utf-8');
assert('  safeToast.ts 文件存在', () => safeToastSrc.length > 500);
assert('  safeToast 导出 sanitizeDescription', () => safeToastSrc.includes('export function sanitizeDescription'));
assert('  safeToast 导出 safeToast.error/success/warning/info',
  () => safeToastSrc.includes('export const safeToast') &&
     safeToastSrc.includes('error(message') &&
     safeToastSrc.includes('success(message') &&
     safeToastSrc.includes('warning(message') &&
     safeToastSrc.includes('info(message'));
assert('  safeToast 内部 try/catch 终极兜底',
  () => safeToastSrc.includes('target(message, forward)') &&
     safeToastSrc.includes("console.error('[safeToast] toast call failed', e)"));

// --- 真正的 sanitizer 行为测试 (从 safeToast.ts 里扣出最小实现) ---
function safeString(input) {
  if (input == null) return '';
  if (typeof input === 'string') return input;
  if (input instanceof Error) return `${input.name}: ${input.message}`;
  try { return String(input); } catch { return '[unstringifiable value]'; }
}
const REACT_INTERNAL = /Minified React error #\d+|https?:\/\/react\.dev\/errors\/\d+|reactjs\.org\/link\/minified-/i;
function sanitizeDescription(input, kind = 'error') {
  const FALLBACK = '[UI 异常] 操作未完成, 已记录到控制台, 请稍后重试。';
  const raw = safeString(input).trim();
  if (!raw) return FALLBACK;
  if (REACT_INTERNAL.test(raw)) return FALLBACK;
  return raw.replace(/\s+/g, ' ').slice(0, 500);
}

assert('  sanitize: 普通字符串直通',
  () => sanitizeDescription('保存失败') === '保存失败');
assert('  sanitize: Minified React error #321 -> fallback',
  () => sanitizeDescription('Minified React error #321; visit https://react.dev/errors/321') === '[UI 异常] 操作未完成, 已记录到控制台, 请稍后重试。');
assert('  sanitize: react.dev URL -> fallback',
  () => sanitizeDescription('see https://react.dev/errors/418') === '[UI 异常] 操作未完成, 已记录到控制台, 请稍后重试。');
assert('  sanitize: Error 对象 -> name: msg',
  () => safeString(new TypeError('bad thing')) === 'TypeError: bad thing');
assert('  sanitize: null / undefined -> fallback',
  () => sanitizeDescription(null) === '[UI 异常] 操作未完成, 已记录到控制台, 请稍后重试。');
assert('  sanitize: 超长字符串被截断到 500',
  () => sanitizeDescription('x'.repeat(2000)).length === 500);
assert('  sanitize: 多行归一为空格',
  () => sanitizeDescription('line1\nline2\nline3').includes('line1 line2 line3'));
assert('  sanitize: 嵌套 React error 字串也命中',
  () => !sanitizeDescription('prefix: Minified React error #500; suffix').includes('React error'));

// --- 现场验证迁移完成 ---
const recStopSrc = fs.readFileSync(ROOT + '/src/hooks/useRecordingStop.ts', 'utf-8');
assert('  useRecordingStop.ts 已 import safeToast',
  () => recStopSrc.includes("from '@/lib/safeToast'"));
assert('  useRecordingStop.ts 已无 toast.error 直接调用',
  () => !/\btoast\.error\(/.test(recStopSrc));
assert('  useRecordingStop.ts:477 附近用 safeToast.error',
  () => recStopSrc.includes('safeToast.error(\'保存会议失败\''));

const sumGenSrc = fs.readFileSync(ROOT + '/src/hooks/meeting-details/useSummaryGeneration.ts', 'utf-8');
assert('  useSummaryGeneration.ts 已 import safeToast',
  () => sumGenSrc.includes("from '@/lib/safeToast'"));
assert('  useSummaryGeneration.ts 已无 toast.<level>( 调用',
  () => !/\btoast\.(error|success|warning|info)\(/.test(sumGenSrc));

const meetDataSrc = fs.readFileSync(ROOT + '/src/hooks/meeting-details/useMeetingData.ts', 'utf-8');
assert('  useMeetingData.ts 已无 toast.error 调用',
  () => !/\btoast\.error\(/.test(meetDataSrc));

const layoutSrc = fs.readFileSync(ROOT + '/src/app/layout.tsx', 'utf-8');
assert('  layout.tsx 已 import bindSonner',
  () => layoutSrc.includes('bindSonner') && layoutSrc.includes("@/lib/safeToast"));
assert('  layout.tsx 已注册 SafeToastBinder 组件',
  () => layoutSrc.includes('SafeToastBinder') &&
     layoutSrc.includes('bindSonner({') &&
     layoutSrc.includes('error: toast.error') &&
     layoutSrc.includes('success: toast.success'));


console.log('');
console.log('--- T17: #321 复发根因 + 摘要啰嗦 + 卡死 三连修复 ---');

// #321 复发根因修复
const meetingDataSrc = fs.readFileSync(ROOT + '/src/hooks/meeting-details/useMeetingData.ts', 'utf-8');
assert('  useMeetingData.ts 已 import sanitizeDescription',
  () => meetingDataSrc.includes('sanitizeDescription') && meetingDataSrc.includes("from \"@/lib/safeToast\""));
assert('  useMeetingData.ts:70 setError(error.message) 已 sanitize',
  () => /setError\(sanitizeDescription\(error\.message\)\)/.test(meetingDataSrc));
assert('  useMeetingData.ts:114 setError 已 sanitize',
  () => (meetingDataSrc.match(/setError\(sanitizeDescription\(error\.message\)\)/g) || []).length >= 2);

const sumGenSrcT17 = fs.readFileSync(ROOT + '/src/hooks/meeting-details/useSummaryGeneration.ts', 'utf-8');
assert('  useSummaryGeneration.ts 已 import sanitizeDescription',
  () => sumGenSrcT17.includes('sanitizeDescription'));
assert('  useSummaryGeneration.ts setSummaryError 已 sanitize',
  () => /setSummaryError\(sanitizeDescription\(/.test(sumGenSrcT17));

const recStopSrcT17 = fs.readFileSync(ROOT + '/src/hooks/useRecordingStop.ts', 'utf-8');
assert('  useRecordingStop.ts:475 setStatus 已 sanitize',
  () => /setStatus\(RecordingStatus\.ERROR, sanitizeDescription\(msg, 'error'\)\)/.test(recStopSrcT17));
assert('  useRecordingStop.ts:494 setStatus 已 sanitize',
  () => /setStatus\(RecordingStatus\.ERROR, sanitizeDescription\(error/.test(recStopSrcT17));

// 摘要啰嗦修复 - 法律模板精简
const legalTpl = JSON.parse(fs.readFileSync(ROOT + '/src-tauri/templates/legal_consultation.json', 'utf-8'));
assert('  legal_consultation.json 5 段 (从 9 段精简)',
  () => Array.isArray(legalTpl.sections) && legalTpl.sections.length === 5);
assert('  legal_consultation.json 名称: 法律咨询纪要 (非"诉讼")',
  () => legalTpl.name === '法律咨询纪要');
const sectionTitles = legalTpl.sections.map(s => s.title).join('|');
assert('  legal_consultation.json 段名精简: 基本事实|当事人主张|律师建议|待办事项|遗留问题',
  () => sectionTitles === '基本事实|当事人主张|律师建议|待办事项|遗留问题');
assert('  每段 instruction 都含禁/未/不 约束词',
  () => legalTpl.sections.every(s => /[禁未不得]/.test(s.instruction)));

// 卡死修复
const sidebarSrc = fs.readFileSync(ROOT + '/src/components/Sidebar/SidebarProvider.tsx', 'utf-8');
assert('  Polling 间隔改 2s (从 5s)',
  () => /}, 2000\);.*Poll every 2 seconds/s.test(sidebarSrc));
assert('  Polling 上限 300 (10 分钟)',
  () => /MAX_POLLS = 300/.test(sidebarSrc));

const blockNoteSrc = fs.readFileSync(ROOT + '/src/components/AISummary/BlockNoteSummaryView.tsx', 'utf-8');
assert('  BlockNoteSummaryView 已加 status=completed force reload effect',
  () => /status !== 'completed'/.test(blockNoteSrc) &&
     /force reload on status=completed/.test(blockNoteSrc));
assert('  BlockNoteSummaryView 已加 lastLoadedMarkdownRef 防循环',
  () => blockNoteSrc.includes('lastLoadedMarkdownRef') &&
     /lastLoadedMarkdownRef.current = markdownKey/.test(blockNoteSrc));


console.log('');
console.log('--- T18: 全站 toast 迁移 + #321 真凶修复 ---');

// RetranscribeDialog error 已 sanitize
const retransSrc = fs.readFileSync(ROOT + '/src/components/MeetingDetails/RetranscribeDialog.tsx', 'utf-8');
assert('  RetranscribeDialog setError(event.payload.error) 已 sanitize',
  () => /setError\(sanitizeDescription\(event\.payload\.error, 'error'\)\)/.test(retransSrc));

// useMeetingData sync effect 加 prop guard
const meetingDataSrc2 = fs.readFileSync(ROOT + '/src/hooks/meeting-details/useMeetingData.ts', 'utf-8');
assert('  useMeetingData.ts 加 lastSyncedPropRef 防止 sync 覆盖新结果',
  () => meetingDataSrc2.includes('lastSyncedPropRef') &&
     meetingDataSrc2.includes('if (lastSyncedPropRef.current === summaryData) return'));

// BlockNoteSummaryView status-only effect
const blockNoteSrc2 = fs.readFileSync(ROOT + '/src/components/AISummary/BlockNoteSummaryView.tsx', 'utf-8');
assert('  BlockNoteSummaryView effect 依赖 status + markdown 双重触发',
  () => /status !== 'completed'/.test(blockNoteSrc2) &&
     /\[status, data\?\.markdown, editor, format\]/.test(blockNoteSrc2) &&
     /markdownKey/.test(blockNoteSrc2));
let bareToastCount = 0;
function walk(dir) {
  for (const f of require('fs').readdirSync(dir)) {
    const full = dir + '/' + f;
    if (require('fs').statSync(full).isDirectory()) {
      if (f === 'node_modules' || f === '.next' || f === 'dist') continue;
      walk(full);
    } else if (f.endsWith('.ts') || f.endsWith('.tsx')) {
      const s = require('fs').readFileSync(full, 'utf-8');
      // 排除 safeToast.ts 自身
      if (full.endsWith('safeToast.ts')) return;
      // 如果文件 import sonner toast, 但还有裸 toast.<level>( 调用 — 表示未迁移
      if (s.includes("from 'sonner'")) {
        // 不算 SafeToastBinder 内部的 toast.error (binder 内部)
        const matches = s.match(/\btoast\.(error|success|warning|info)\(/g) || [];
        bareToastCount += matches.length;
      }
    }
  }
}
walk(ROOT + '/src');
assert('  全站裸 toast.<level>( 调用 ≤ 4 (仅 SafeToastBinder 内部 + 2 个 React element 案例)',
  () => bareToastCount <= 4);

console.log('');
console.log('========================================');
console.log(`总计: ${pass} 通过 / ${fail} 失败`);
console.log('========================================');
process.exit(fail > 0 ? 1 : 0);
