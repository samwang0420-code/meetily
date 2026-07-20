import React, { useState, useEffect } from 'react';
import { useTranslation } from '@/i18n';
import { Switch } from '@/components/ui/switch';
import { useConfig } from '@/contexts/ConfigContext';
import { FolderOpen } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { DeviceSelection, SelectedDevices } from '@/components/DeviceSelection';
import Analytics from '@/lib/analytics';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';

export interface Recording偏好设置 {
  save_folder: string;
  auto_save: boolean;
  file_format: string;
  preferred_mic_device: string | null;
  preferred_system_device: string | null;
}

interface RecordingSettingsProps {
  onSave?: (preferences: Recording偏好设置) => void;
}

export function RecordingSettings({ onSave }: RecordingSettingsProps) {
  const { t } = useTranslation();
  const { isAutoRetranscribe, toggleIsAutoRetranscribe } = useConfig();
  const [preferences, set偏好设置] = useState<Recording偏好设置>({
    save_folder: '',
    auto_save: true,
    file_format: 'mp4',
    preferred_mic_device: null,
    preferred_system_device: null
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [showRecordingNotification, setShowRecordingNotification] = useState(true);
  // 离线会记 v0.6.9: 实时字幕预览开关 (关闭后只录音, 录音结束再一次性离线识别)
  const [realtimePreview, setRealtimePreviewState] = useState<boolean>(() => {
    if (typeof window === 'undefined') return true;
    const saved = window.localStorage.getItem('realtimePreview');
    return saved === null ? true : saved === 'true';
  });
  const setRealtimePreview = (v: boolean) => {
    setRealtimePreviewState(v);
    try { window.localStorage.setItem('realtimePreview', String(v)); } catch {}
  };

  // Load recording preferences on component mount
  useEffect(() => {
    const loadPreferences = async () => {
      try {
        const prefs = await invoke<Recording偏好设置>('get_recording_preferences');
        set偏好设置(prefs);
      } catch (error) {
        console.error('Failed to load recording preferences:', error);
        // If loading fails, get default folder path
        try {
          const defaultPath = await invoke<string>('get_default_recordings_folder_path');
          set偏好设置(prev => ({ ...prev, save_folder: defaultPath }));
        } catch (defaultError) {
          console.error('Failed to get default folder path:', defaultError);
        }
      } finally {
        setLoading(false);
      }
    };

    loadPreferences();
  }, []);

  // Load recording notification preference
  useEffect(() => {
    const loadNotificationPref = async () => {
      try {
        const { Store } = await import('@tauri-apps/plugin-store');
        const store = await Store.load('preferences.json');
        const show = await store.get<boolean>('show_recording_notification') ?? true;
        setShowRecordingNotification(show);
      } catch (error) {
        console.error('Failed to load notification preference:', error);
      }
    };
    loadNotificationPref();
  }, []);

  const handleAutoSaveToggle = async (enabled: boolean) => {
    const new偏好设置 = { ...preferences, auto_save: enabled };
    set偏好设置(new偏好设置);
    await save偏好设置(new偏好设置);

    // Track auto-save setting change
    await Analytics.track('auto_save_recording_toggled', {
      enabled: enabled.toString()
    });
  };

  const handleDeviceChange = async (devices: SelectedDevices) => {
    const new偏好设置 = {
      ...preferences,
      preferred_mic_device: devices.micDevice,
      preferred_system_device: devices.systemDevice
    };
    set偏好设置(new偏好设置);
    await save偏好设置(new偏好设置);

    // Track default device preference changes
    // Note: Individual device selection analytics are tracked in DeviceSelection component
    await Analytics.track('default_devices_changed', {
      has_preferred_microphone: (!!devices.micDevice).toString(),
      has_preferred_system_audio: (!!devices.systemDevice).toString()
    });
  };

  const handleOpenFolder = async () => {
    try {
      await invoke('open_recordings_folder');
    } catch (error) {
      console.error('Failed to open recordings folder:', error);
    }
  };

  const handleNotificationToggle = async (enabled: boolean) => {
    try {
      setShowRecordingNotification(enabled);
      const { Store } = await import('@tauri-apps/plugin-store');
      const store = await Store.load('preferences.json');
      await store.set('show_recording_notification', enabled);
      await store.save();
      safeToast.success('偏好已保存');
      await Analytics.track('recording_notification_preference_changed', {
        enabled: enabled.toString()
      });
    } catch (error) {
      console.error('Failed to save notification preference:', error);
      safeToast.error(t('errors.save_preference_failed'));
    }
  };

  const save偏好设置 = async (prefs: Recording偏好设置) => {
    setSaving(true);
    try {
      await invoke('set_recording_preferences', { preferences: prefs });
      onSave?.(prefs);

      // Show success toast with device details
      const micDevice = prefs.preferred_mic_device || 'Default';
      const systemDevice = prefs.preferred_system_device || 'Default';
      safeToast.success("设备 preferences saved", {
        description: `麦克风: ${micDevice}, System Audio: ${systemDevice}`
      });
    } catch (error) {
      console.error('Failed to save recording preferences:', error);
      safeToast.error("保存设备偏好失败", {
        description: error instanceof Error ? error.message : String(error)
      });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="animate-pulse">
        <div className="h-4 bg-gray-200 rounded w-1/4 mb-4"></div>
        <div className="h-8 bg-gray-200 rounded mb-4"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-4">{t('settings.recordings.title')}</h3>
        <p className="text-sm text-gray-600 mb-6">
          {t('settings.recordings.title_desc')}
        </p>
      </div>

      {/* Auto Save Toggle */}
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">{t('settings.recordings.save')}</div>
          <div className="text-sm text-gray-600">
            Automatically save audio files when recording stops
          </div>
        </div>
        <Switch
          checked={preferences.auto_save}
          onCheckedChange={handleAutoSaveToggle}
          disabled={saving}
        />
      </div>

      {/* Folder Location - Only shown when auto_save is enabled */}
      {preferences.auto_save && (
        <div className="space-y-4">
          <div className="p-4 border rounded-lg bg-gray-50">
            <div className="font-medium mb-2">{t('settings.recordings.save_location')}</div>
            <div className="text-sm text-gray-600 mb-3 break-all">
              {preferences.save_folder || t('settings.recordings.default_folder')}
            </div>
            <button
              onClick={handleOpenFolder}
              className="flex items-center gap-2 px-3 py-2 text-sm border border-gray-300 rounded-md hover:bg-gray-50 transition-colors"
            >
              <FolderOpen className="w-4 h-4" />
              {t('settings.recordings.open_folder')}
            </button>
          </div>

          <div className="p-4 border rounded-lg bg-blue-50">
            <div className="text-sm text-blue-800">
              <strong>{t('settings.recordings.file_format_label')}:</strong> {preferences.file_format.toUpperCase()} {t('settings.recordings.file_format_unit')}
            </div>
            <div className="text-xs text-blue-600 mt-1">
              {t('settings.recordings.saved_with_timestamp')}: recording_YYYYMMDD_HHMMSS.{preferences.file_format}
            </div>
          </div>
        </div>
      )}

      {/* Info when auto_save is disabled */}
      {!preferences.auto_save && (
        <div className="p-4 border rounded-lg bg-yellow-50">
          <div className="text-sm text-yellow-800">
            {t('settings.recordings.disabled_hint')}
          </div>
        </div>
      )}

      {/* Recording Notification Toggle */}
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">{t('settings.recordings.start_notification')}</div>
          <div className="text-sm text-gray-600">
            {t('settings.recordings.start_notification_desc')}
          </div>
        </div>
        <Switch
          checked={showRecordingNotification}
          onCheckedChange={handleNotificationToggle}
        />
      </div>

      {/* 离线会记 v0.6.9: 实时字幕预览开关 (方案 4.A) */}
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">{t('settings.recordings.realtime_preview')}</div>
          <div className="text-sm text-gray-600">
            {t('settings.recordings.realtime_preview_desc')}
          </div>
        </div>
        <Switch
          checked={realtimePreview}
          onCheckedChange={setRealtimePreview}
        />
      </div>

      {/* v0.7.0+: 录音停止后自动重新转录 (优化结果). 默认开. */}
      <div className="flex items-center justify-between p-4 border rounded-lg">
        <div className="flex-1">
          <div className="font-medium">自动重新转录 (优化)</div>
          <div className="text-sm text-gray-600">
            录音停止后, 后台整段上下文重新跑一次, 字幕更准 (10-30 秒)。关闭后保留流式初步识别结果。
          </div>
        </div>
        <Switch
          checked={isAutoRetranscribe}
          onCheckedChange={toggleIsAutoRetranscribe}
        />
      </div>

      {/* Device 偏好设置 */}
      <div className="space-y-4">
        <div className="border-t pt-6">
          <h4 className="text-base font-medium text-gray-900 mb-4">{t('settings.recordings.audio_devices')}</h4>
          <p className="text-sm text-gray-600 mb-4">
            设置录音时优先使用的麦克风和系统音频设备。开始新录音时, 系统会自动选用这些设备。
          </p>

          <div className="border rounded-lg p-4 bg-gray-50">
            <DeviceSelection
              selectedDevices={{
                micDevice: preferences.preferred_mic_device,
                systemDevice: preferences.preferred_system_device
              }}
              onDeviceChange={handleDeviceChange}
              disabled={saving}
            />
          </div>
        </div>
      </div>
    </div>
  );
}