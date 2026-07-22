'use client';

import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from '@/i18n';
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { type DeviceProfile } from './HardwareStatusBadge';

// v0.7.0+ P0-4: 首次启动强制展示硬件档位 + 功能限制弹窗.
// localStorage 标记 hardware_onboarding_seen=true 后不再自动弹出.
// 设置页 HardwareSection 提供"重新查看"按钮, 直接 open 控制.

const STORAGE_KEY = 'hardware_onboarding_seen';

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function HardwareOnboardingModal({ open, onOpenChange }: Props) {
  const { t, locale } = useTranslation();
  const [profile, setProfile] = useState<DeviceProfile | null>(null);

  useEffect(() => {
    if (!open) return;
    (async () => {
      try {
        const p = await invoke<DeviceProfile>('device_detect_profile');
        setProfile(p);
      } catch (e) {
        console.warn('hardware detect failed', e);
      }
    })();
  }, [open]);

  function handleAcknowledge() {
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(STORAGE_KEY, 'true');
    }
    onOpenChange(false);
  }

  const tierZh = profile?.tier === 'high' ? '完美配置'
    : profile?.tier === 'medium' ? '基础可用' : '低配不推荐';
  const tierEn = profile?.tier === 'high' ? 'High-spec'
    : profile?.tier === 'medium' ? 'Mid-spec' : 'Low-spec';

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogTitle>
          {locale === 'zh' ? '硬件检测结果' : 'Hardware Detection'}
        </DialogTitle>
        <DialogDescription>
          {locale === 'zh'
            ? '检测到的设备规格将决定可用的功能档位.'
            : 'Your device specs determine which features are available.'}
        </DialogDescription>

        {!profile ? (
          <div className="text-sm text-slate-500 py-8 text-center">
            {locale === 'zh' ? '检测中…' : 'Detecting…'}
          </div>
        ) : (
          <div className="space-y-4 mt-2">
            <div className="rounded-lg border p-3 bg-slate-50 dark:bg-slate-900">
              <div className="text-xs text-slate-500 mb-1">
                {locale === 'zh' ? '当前档位' : 'Current Tier'}
              </div>
              <div className="text-2xl font-bold">
                {locale === 'zh' ? tierZh : tierEn}
              </div>
            </div>

            <div className="grid grid-cols-2 gap-2 text-sm">
              <div className="border rounded p-2">
                <div className="text-xs text-slate-500">
                  {locale === 'zh' ? '总内存' : 'RAM'}
                </div>
                <div className="font-medium">
                  {(profile.total_memory_mb / 1024).toFixed(1)} GB
                </div>
              </div>
              <div className="border rounded p-2">
                <div className="text-xs text-slate-500">{t("hardware.spec_cpu")}</div>
                <div className="font-medium text-xs">{profile.cpu_brand}</div>
              </div>
              {profile.is_apple_silicon && (
                <div className="border rounded p-2">
                  <div className="text-xs text-slate-500">{t("hardware.spec_metal_vram")}</div>
                  <div className="font-medium">
                    {(profile.metal_vram_mb / 1024).toFixed(1)} GB
                  </div>
                </div>
              )}
              <div className="border rounded p-2">
                <div className="text-xs text-slate-500">
                  {locale === 'zh' ? '最长录音' : 'Max Duration'}
                </div>
                <div className="font-medium">
                  {profile.recommended_max_meeting_minutes} {locale === 'zh' ? '分钟' : 'min'}
                </div>
              </div>
            </div>

            <div className="space-y-1 text-sm">
              <CapabilityLine
                enabled={!profile.cam_plus_plus_disabled}
                zh="cam++ 人声分离"
                en="cam++ speaker diarization"
              />
              <CapabilityLine
                enabled={!profile.nano_disabled}
                zh="FunASR-Nano 高精度模型"
                en="FunASR-Nano high-precision model"
              />
              <CapabilityLine
                enabled={!profile.long_summary_disabled}
                zh="长音频 Map-Reduce 摘要"
                en="Long audio Map-Reduce summary"
              />
            </div>

            <p className="text-xs text-slate-500 leading-relaxed">
              {locale === 'zh'
                ? `本机最低配置: ≥8GB 内存, 推荐 ≥16GB Apple Silicon. 内存持续 > 1.2GB 时, 系统会自动卸载 cam++/Nano 模块.`
                : `Min: ≥8GB RAM. Recommended: ≥16GB Apple Silicon. Auto-unloads cam++/Nano when RSS > 1.2GB.`}
            </p>
          </div>
        )}

        <div className="flex justify-end pt-2">
          <Button onClick={handleAcknowledge}>
            {locale === 'zh' ? '知道了' : 'Got it'}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function CapabilityLine({ enabled, zh, en }: { enabled: boolean; zh: string; en: string }) {
  const { locale } = useTranslation();
  return (
    <div className="flex items-center gap-2">
      <span className={enabled ? 'text-green-600' : 'text-red-500'}>
        {enabled ? '✓' : '✗'}
      </span>
      <span className={enabled ? '' : 'line-through text-slate-400'}>
        {locale === 'zh' ? zh : en}
      </span>
    </div>
  );
}

// v0.7.0+ P0-4: helper. 首次启动是否要展示 (供 layout 调).
export function shouldShowHardwareOnboarding(): boolean {
  if (typeof window === 'undefined') return false;
  return window.localStorage.getItem(STORAGE_KEY) !== 'true';
}

export function resetHardwareOnboardingSeen() {
  if (typeof window !== 'undefined') {
    window.localStorage.removeItem(STORAGE_KEY);
  }
}
