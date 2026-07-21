'use client';

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { type DeviceProfile, type DeviceTier } from '@/components/HardwareProfile/HardwareStatusBadge';

interface UseDeviceProfileResult {
  profile: DeviceProfile | null;
  tier: DeviceTier | null;
  isLoading: boolean;
  /** 检查给定分钟数是否超出本机推荐上限 */
  isWithinRecommendedLimit: (minutes: number) => boolean;
}

/**
 * v0.7.0+ P0-4: 全局 hook, 在消费侧拿到设备档位 + 推荐上限.
 * 用于 UI 拦截 (e.g. 会议目标时长 > tier 上限时禁用按钮或提示).
 */
export function useDeviceProfile(): UseDeviceProfileResult {
  const [profile, setProfile] = useState<DeviceProfile | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const p = await invoke<DeviceProfile>('device_detect_profile');
        if (!cancelled) {
          setProfile(p);
          setIsLoading(false);
        }
      } catch (e) {
        console.warn('device profile fetch failed', e);
        if (!cancelled) setIsLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, []);

  return {
    profile,
    tier: profile?.tier ?? null,
    isLoading,
    isWithinRecommendedLimit: (minutes: number) => {
      if (!profile) return true; // 加载中, 放行
      return minutes <= profile.recommended_max_meeting_minutes;
    },
  };
}
