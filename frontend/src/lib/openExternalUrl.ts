// §156: Tauri 单窗口 webview 默认拦截 mailto: 协议 + target="_blank" 无新窗口机制
// 必须用 @tauri-apps/plugin-opener 调系统默认处理
import { openUrl } from '@tauri-apps/plugin-opener';

export async function openExternalUrl(url: string): Promise<void> {
  try {
    await openUrl(url);
  } catch (err) {
    // 开发 web 模式或 desktop 模式 openUrl 失败时, 用浏览器原生 <a> click 兜底
    console.error('[openExternalUrl] openUrl failed, falling back to window.open:', err);
    try {
      const a = document.createElement('a');
      a.href = url;
      a.target = '_blank';
      a.rel = 'noopener noreferrer';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
    } catch (e2) {
      console.error('[openExternalUrl] fallback also failed:', e2);
    }
  }
}
