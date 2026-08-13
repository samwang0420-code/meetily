import { toast } from 'sonner';

import Analytics from '@/lib/analytics';
import { DICTS, Locale } from '@/i18n';

/**
 * §104: 录音通知 toast 走 i18n, 跟随主界面 locale.
 * showRecordingNotification 是 utility function (非 React 组件),
 * 不能用 useTranslation hook, 改用 localStorage + DICTS 直接 lookup.
 */
function localT(path: string): string {
  if (typeof window === 'undefined') return path;
  const saved = window.localStorage?.getItem('lixianhuiji.locale');
  const locale: Locale = (saved === 'en' ? 'en' : 'zh');
  const dict = DICTS[locale];
  const parts = path.split('.');
  let cur: unknown = dict;
  for (const p of parts) {
    if (cur && typeof cur === 'object' && p in (cur as Record<string, unknown>)) {
      cur = (cur as Record<string, unknown>)[p];
    } else {
      return path;
    }
  }
  return typeof cur === 'string' ? cur : path;
}

/**
 * Shows the recording notification toast with compliance message.
 * Checks user preferences and displays a dismissible toast with:
 * - notice to inform participants
 * - "Don't show again" checkbox
 * - Acknowledgment button
 *
 * @returns Promise<void> - Resolves when notification is shown or skipped
 */
export async function showRecordingNotification(): Promise<void> {
  try {
    const { Store } = await import('@tauri-apps/plugin-store');
    const store = await Store.load('preferences.json');
    const showNotification = await store.get<boolean>('show_recording_notification') ?? true;

    if (showNotification) {
      let dontShowAgain = false;

      const toastId = toast.info(localT('recording.notification.title'), {
        description: (
          <div className="space-y-3 min-w-[280px]">
            <p className="text-sm font-medium text-gray-900">
              {localT('recording.notification.body')}
            </p>
            <label className="flex items-center gap-2 text-xs cursor-pointer hover:bg-blue-100 p-2 rounded transition-colors">
              <input
                type="checkbox"
                onChange={(e) => {
                  dontShowAgain = e.target.checked;
                }}
                className="rounded border-gray-300 text-blue-600 focus:ring-blue-500 focus:ring-2"
              />
              <span className="select-none text-gray-700">{localT('recording.notification.dont_show')}</span>
            </label>
            <button
              onClick={async () => {
                if (dontShowAgain) {
                  const { Store } = await import('@tauri-apps/plugin-store');
                  const store = await Store.load('preferences.json');
                  await store.set('show_recording_notification', false);
                  await store.save();
                }
                Analytics.trackButtonClick('recording_notification_acknowledged', 'toast');
                toast.dismiss(toastId);
              }}
              className="w-full px-3 py-1.5 bg-gray-900 text-white text-xs rounded hover:bg-gray-800 transition-colors font-medium"
            >
              {localT('recording.notification.ack')}
            </button>
          </div>
        ),
        duration: 10000,
        position: 'bottom-right',
      });
    }
  } catch (notificationError) {
    console.error('Failed to show recording notification:', notificationError);
    // Don't fail the recording if notification fails
  }
}
