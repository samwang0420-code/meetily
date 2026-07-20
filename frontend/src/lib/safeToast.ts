/**
 * safeToast.ts — 防御 React #321 (Minified React error #xxx) 二次炸开。
 *
 * 根因: 当 catch 块拿到的是 React reconciler 抛出的对象时, `String(error)`
 * 会得到 "Minified React error #321; visit https://react.dev/errors/321 ...",
 * 如果直接塞进 sonner 的 `description`, 在某些路径下 sonner 内部 state 更新
 * 会再次触发 React render 异常, 用户看到 toast 文本就是 React error 字面。
 *
 * 策略:
 *   1. 强制把 error 转成纯字符串 (`safeString`)
 *   2. 命中 React error pattern 时降级为友好中文 + 强制前缀 "[UI 异常]"
 *   3. 长度截断到 500 字符 (防超长日志撑爆 toast)
 *   4. 对外只暴露 `safeToast.error/success/warning/info`, 调用方不需要
 *      再思考如何 sanitize
 */

const REACT_INTERNAL_PATTERN =
  /Minified React error #\d+|https?:\/\/react\.dev\/errors\/\d+|reactjs\.org\/link\/minified-/i;

const MAX_DESC_LEN = 500;

const FALLBACK_BY_KIND: Record<'error' | 'success' | 'warning' | 'info', string> = {
  error: '[UI 异常] 操作未完成, 已记录到控制台, 请稍后重试。',
  success: '操作成功。',
  warning: '操作已跳过, 请稍后重试。',
  info: '请稍候。',
};

export function safeString(input: unknown): string {
  if (input == null) return '';
  if (typeof input === 'string') return input;
  if (input instanceof Error) {
    return `${input.name}: ${input.message}`;
  }
  try {
    return String(input);
  } catch {
    return '[unstringifiable value]';
  }
}

export function sanitizeDescription(input: unknown, kind: keyof typeof FALLBACK_BY_KIND = 'error'): string {
  const raw = safeString(input).trim();
  if (!raw) return FALLBACK_BY_KIND[kind];
  if (REACT_INTERNAL_PATTERN.test(raw)) {
    return FALLBACK_BY_KIND[kind];
  }
  // 多行归一 + 截断
  const flat = raw.replace(/\s+/g, ' ').slice(0, MAX_DESC_LEN);
  return flat;
}

// 宽松签名: 实际签名由 sonner 提供, 我们只关心 description sanitize
type SonnerLike = (message: string, options?: Record<string, unknown>) => unknown;

let _toastError: SonnerLike | null = null;
let _toastSuccess: SonnerLike | null = null;
let _toastWarning: SonnerLike | null = null;
let _toastInfo: SonnerLike | null = null;

export function bindSonner(toast: {
  error: SonnerLike;
  success: SonnerLike;
  warning: SonnerLike;
  info: SonnerLike;
}) {
  _toastError = toast.error;
  _toastSuccess = toast.success;
  _toastWarning = toast.warning;
  _toastInfo = toast.info;
}

type SafeOpts = Record<string, unknown>;
function call(
  target: SonnerLike | null,
  message: string,
  opts: SafeOpts | undefined,
  kind: keyof typeof FALLBACK_BY_KIND,
) {
  if (!target) {
    if (opts?.description !== undefined) {
      // eslint-disable-next-line no-console
      console.warn(`[safeToast:${kind}] ${message} :: ${sanitizeDescription(opts.description, kind)}`);
    } else {
      // eslint-disable-next-line no-console
      console.warn(`[safeToast:${kind}] ${message}`);
    }
    return;
  }
  const forward: Record<string, unknown> = {};
  if (opts) {
    for (const [k, v] of Object.entries(opts)) {
      if (k === 'description') {
        forward.description = sanitizeDescription(v, kind);
      } else {
        forward[k] = v;
      }
    }
  }
  try {
    target(message, forward);
  } catch (e) {
    // 终极兜底: 即便 sonner 自己炸了也不能再炸 React
    // eslint-disable-next-line no-console
    console.error('[safeToast] toast call failed', e);
  }
}

export const safeToast = {
  error(message: string, opts?: Record<string, unknown>) {
    call(_toastError, message, opts, 'error');
  },
  success(message: string, opts?: Record<string, unknown>) {
    call(_toastSuccess, message, opts, 'success');
  },
  warning(message: string, opts?: Record<string, unknown>) {
    call(_toastWarning, message, opts, 'warning');
  },
  info(message: string, opts?: Record<string, unknown>) {
    call(_toastInfo, message, opts, 'info');
  },
};
