"use client";

// §P0-B Obsidian 集成设置卡片
// 显示: enabled 开关 + vault 路径 + subdir + 模板 + 上次导出状态 + 预览按钮
// 真实写盘动作在 §P0-B Phase 2 由 summary/service.rs 触发

import { useEffect, useState } from "react";
import { useTranslation } from "@/i18n";
import { invoke } from "@tauri-apps/api/core";
import { Switch } from "./ui/switch";
import { Input } from "./ui/input";
import { Button } from "./ui/button";
import { Label } from "./ui/label";
import { BookOpen, ExternalLink, CheckCircle2, AlertCircle } from "lucide-react";

interface ObsidianSettingsState {
  enabled: boolean;
  vault_path: string;
  subdir: string;
  template_id: string;
  last_exported_at: string | null;
  last_export_status: string | null;
  last_export_error: string | null;
  last_exported_meeting_id: string | null;
}

export function ObsidianSettings() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<ObsidianSettingsState | null>(null);
  const [previewMd, setPreviewMd] = useState<string | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    loadSettings();
  }, []);

  async function loadSettings() {
    try {
      const uid = window.localStorage.getItem('lixianhuiji.user_id');
      const userId = uid ? Number(uid) : 2; // fallback to machine owner (per §49 user_id=-1 fallback)
      const s = await invoke('api_obsidian_get_settings', { userId }) as ObsidianSettingsState;
      setSettings(s);
    } catch (e) {
      console.error('obsidian.get_settings failed', e);
      setSettings({
        enabled: false,
        vault_path: '~/Documents/Obsidian Vault',
        subdir: '会议',
        template_id: 'default',
        last_exported_at: null,
        last_export_status: null,
        last_export_error: null,
        last_exported_meeting_id: null,
      });
    }
  }

  async function saveSettings(patch: Partial<ObsidianSettingsState>) {
    if (!settings) return;
    setSaving(true);
    try {
      const next = { ...settings, ...patch };
      await invoke('api_obsidian_set_settings', {
        settings: {
          user_id: 2, // Phase 2: 从 auth session 取
          enabled: next.enabled,
          vault_path: next.vault_path,
          subdir: next.subdir,
          template_id: next.template_id,
          last_exported_meeting_id: next.last_exported_meeting_id,
          last_exported_at: next.last_exported_at,
          last_export_status: next.last_export_status,
          last_export_error: next.last_export_error,
        },
      });
      setSettings(next);
    } catch (e) {
      console.error('obsidian.set_settings failed', e);
    } finally {
      setSaving(false);
    }
  }

  async function previewMarkdown() {
    setPreviewError(null);
    setPreviewMd(null);
    try {
      const md = await invoke('api_obsidian_preview_markdown', { meetingId: 'preview' }) as string;
      setPreviewMd(md);
    } catch (e: any) {
      setPreviewError(String(e?.message || e));
    }
  }

  if (!settings) {
    return (
      <div className="p-6 mt-4 bg-white border border-gray-200 rounded-lg">
        <div className="text-gray-500">Loading...</div>
      </div>
    );
  }

  return (
    <div className="p-6 mt-4 bg-white border border-gray-200 rounded-lg space-y-4">
      <div className="flex items-start gap-3">
        <div className="p-2 bg-purple-100 rounded-lg">
          <BookOpen className="w-5 h-5 text-purple-600" />
        </div>
        <div className="flex-1">
          <h3 className="text-lg font-semibold">{t('settings.obsidian.title')}</h3>
          <p className="text-sm text-gray-600 mt-1">{t('settings.obsidian.description')}</p>
        </div>
      </div>

      <div className="flex items-center justify-between py-2 border-b border-gray-100">
        <div className="flex-1">
          <Label className="text-sm font-medium">{t('settings.obsidian.enabled_label')}</Label>
          <p className="text-xs text-gray-500 mt-1">{t('settings.obsidian.enabled_desc')}</p>
        </div>
        <Switch
          checked={settings.enabled}
          onCheckedChange={(v) => saveSettings({ enabled: v })}
          disabled={saving}
        />
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium">{t('settings.obsidian.vault_path_label')}</Label>
        <Input
          value={settings.vault_path}
          placeholder={t('settings.obsidian.vault_path_placeholder')}
          onChange={(e) => setSettings({ ...settings, vault_path: e.target.value })}
          onBlur={() => saveSettings({ vault_path: settings.vault_path })}
          disabled={saving}
        />
      </div>

      <div className="space-y-2">
        <Label className="text-sm font-medium">{t('settings.obsidian.subdir_label')}</Label>
        <Input
          value={settings.subdir}
          placeholder={t('settings.obsidian.subdir_placeholder')}
          onChange={(e) => setSettings({ ...settings, subdir: e.target.value })}
          onBlur={() => saveSettings({ subdir: settings.subdir })}
          disabled={saving}
        />
      </div>

      <div className="text-xs text-gray-500 pt-2 border-t border-gray-100">
        {settings.last_export_status === 'success' && (
          <span className="flex items-center gap-1 text-green-600">
            <CheckCircle2 className="w-3 h-3" />
            {t('settings.obsidian.last_status_success', {
              time: settings.last_exported_at ?? '',
              meeting: settings.last_exported_meeting_id ?? '',
            })}
          </span>
        )}
        {settings.last_export_status === 'failed' && (
          <span className="flex items-center gap-1 text-red-600">
            <AlertCircle className="w-3 h-3" />
            {t('settings.obsidian.last_status_failed', { error: settings.last_export_error ?? '' })}
          </span>
        )}
        {!settings.last_export_status && (
          <span>{t('settings.obsidian.last_status_idle')}</span>
        )}
      </div>

      <Button variant="outline" size="sm" onClick={previewMarkdown} className="w-full">
        <ExternalLink className="w-4 h-4 mr-2" />
        {t('settings.obsidian.test_btn')}
      </Button>

      {previewMd && (
        <pre className="bg-gray-50 border border-gray-200 rounded-md p-3 text-xs overflow-auto max-h-96 whitespace-pre-wrap font-mono">
          {previewMd}
        </pre>
      )}
      {previewError && (
        <div className="text-xs text-red-600 bg-red-50 border border-red-200 rounded-md p-2">
          {t('settings.obsidian.test_error', { msg: previewError })}
        </div>
      )}
    </div>
  );
}
