'use client';

import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from '@/i18n';

export type DeviceTier = 'high' | 'medium' | 'low';

export interface DeviceProfile {
  total_memory_bytes: number;
  total_memory_mb: number;
  cpu_brand: string;
  is_apple_silicon: boolean;
  metal_vram_mb: number;
  tier: DeviceTier;
  recommended_max_meeting_minutes: number;
  recommended_asr_model: string;
  cam_plus_plus_disabled: boolean;
  nano_disabled: boolean;
  long_summary_disabled: boolean;
  detected_at: string;
}

// v0.7.0+ P0-4: 在所有页面顶部显示硬件档位徽章 + 限制提示.
// 内存压力 (RSS > 1.2GB) 时高亮成红色, 提示用户已自动降级.

export function HardwareStatusBadge() {
  const { t, locale } = useTranslation();
  const [profile, setProfile] = useState<DeviceProfile | null>(null);
  const [rssMb, setRssMb] = useState<number>(0);

  useEffect(() => {
    let timer: ReturnType<typeof setInterval> | undefined;
    const fetch = async () => {
      try {
        const p = await invoke<DeviceProfile>('device_detect_profile');
        setProfile(p);
        const r = await invoke<number>('device_current_memory_mb');
        setRssMb(r);
      } catch (e) {
        console.warn('hardware detect failed', e);
      }
    };
    fetch();
    timer = setInterval(fetch, 30000);
    return () => { if (timer) clearInterval(timer); };
  }, []);

  if (!profile) return null;

  const tierLabel = profile.tier === 'high'
    ? (locale === 'zh' ? '完美配置' : 'High-spec')
    : profile.tier === 'medium'
      ? (locale === 'zh' ? '基础可用' : 'Mid-spec')
      : (locale === 'zh' ? '低配不推荐' : 'Low-spec');

  const tierColor = profile.tier === 'high'
    ? 'bg-green-100 text-green-700 border-green-300'
    : profile.tier === 'medium'
      ? 'bg-amber-100 text-amber-700 border-amber-300'
      : 'bg-red-100 text-red-700 border-red-300';

  const memoryColor = rssMb > 1200
    ? 'bg-red-50 text-red-700 border-red-300'
    : 'bg-slate-100 text-slate-600 border-slate-200';

  return (
    <div className="flex items-center gap-2 text-xs flex-wrap" data-testid="hardware-status-badge">
      <span className={`px-2 py-0.5 rounded border font-medium ${tierColor}`}>
        {tierLabel}
      </span>
      <span className={`px-2 py-0.5 rounded border ${memoryColor}`}>
        {locale === 'zh' ? `进程占用 ${rssMb} MB` : `RSS ${rssMb} MB`}
      </span>
      <span className="text-slate-500">
        {locale === 'zh' ? `最长 ${profile.recommended_max_meeting_minutes} 分钟` : `max ${profile.recommended_max_meeting_minutes}m`}
      </span>
      {profile.cam_plus_plus_disabled && (
        <span className="px-2 py-0.5 rounded border bg-orange-50 text-orange-700 border-orange-300">
          {locale === 'zh' ? '人声分离已关' : 'diar off'}
        </span>
      )}
      {rssMb > 1200 && (
        <span className="px-2 py-0.5 rounded border bg-red-100 text-red-700 border-red-300 font-medium">
          {locale === 'zh' ? '内存压力,已自动降级' : 'memory pressure'}
        </span>
      )}
    </div>
  );
}
