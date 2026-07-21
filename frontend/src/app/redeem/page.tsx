'use client';

import React, { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from '@/i18n';
import { useAuth } from '@/contexts/AuthContext';
import { safeToast } from '@/lib/safeToast';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

// v0.7.0+ P0-3: 自助激活码兑换页面
// 用户流程: 输入激活码 → 读本机 machine_id → 调 user_redeem_activation_code → 成功自动升级 Pro
// 错误分层: invalid_format / not_found / expired / already_used / machine_mismatch / not_logged_in

interface RedeemResult {
  success: boolean;
  tier: string;
  expires_at: string;
  error_code: string | null;
  error_message: string | null;
}

export default function RedeemPage() {
  const { t, locale } = useTranslation();
  const router = useRouter();
  const { user, machineId, refresh, session } = useAuth();
  const [code, setCode] = useState('');
  const [redeeming, setRedeeming] = useState(false);
  const [machineIdCopied, setMachineIdCopied] = useState(false);

  // 匿名用户提示: 必须先登录
  useEffect(() => {
    if (user === null) {
      // 等 AuthContext 加载完成
    }
  }, [user]);

  async function handleRedeem() {
    const trimmed = code.trim().toUpperCase();
    if (!trimmed) {
      safeToast.error(
        locale === 'zh' ? '请输入激活码' : 'Please enter an activation code',
        { duration: 4000 }
      );
      return;
    }
    if (!user) {
      safeToast.error(
        locale === 'zh' ? '请先登录账号再兑换' : 'Please log in first',
        { duration: 5000 }
      );
      return;
    }
    setRedeeming(true);
    try {
      const res = await invoke<RedeemResult>('user_redeem_activation_code', {
        session: session ?? '',
        code: trimmed,
      });
      if (res.success) {
        safeToast.success(
          locale === 'zh'
            ? `✨ Pro 已激活! 有效期至 ${res.expires_at.slice(0, 10)}`
            : `✨ Pro activated! Valid until ${res.expires_at.slice(0, 10)}`,
          { duration: 8000 }
        );
        setCode('');
        await refresh();
        // 2s 后跳回首页或 dashboard
        setTimeout(() => router.push('/'), 1500);
      } else {
        const friendly = friendlyError(res.error_code, locale === 'zh');
        safeToast.error(friendly, { duration: 6000 });
      }
    } catch (e: any) {
      safeToast.error(
        locale === 'zh' ? `兑换失败: ${e?.message ?? e}` : `Redeem failed: ${e?.message ?? e}`,
        { duration: 6000 }
      );
    } finally {
      setRedeeming(false);
    }
  }

  async function copyMid() {
    if (!machineId) return;
    await navigator.clipboard.writeText(machineId);
    setMachineIdCopied(true);
    setTimeout(() => setMachineIdCopied(false), 2000);
  }

  return (
    <main className="min-h-screen flex items-center justify-center px-4 py-12 bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-900 dark:to-slate-950">
      <div className="w-full max-w-md space-y-6">
        <div className="text-center space-y-2">
          <h1 className="text-3xl font-bold text-slate-900 dark:text-slate-100">
            {locale === 'zh' ? '激活 Pro 会员' : 'Activate Pro'}
          </h1>
          <p className="text-sm text-slate-600 dark:text-slate-400">
            {locale === 'zh'
              ? '输入您收到的激活码, 自动解锁全部 Pro 权益'
              : 'Enter your activation code to unlock all Pro features'}
          </p>
        </div>

        {!user && (
          <div className="rounded-lg border border-amber-200 bg-amber-50 dark:bg-amber-950/30 dark:border-amber-900 p-4 text-sm">
            <div className="text-amber-900 dark:text-amber-200 font-medium mb-1">
              {locale === 'zh' ? '请先登录' : 'Please log in first'}
            </div>
            <Link
              href="/login"
              className="text-amber-700 dark:text-amber-300 underline text-xs"
            >
              {locale === 'zh' ? '前往登录 →' : 'Go to login →'}
            </Link>
          </div>
        )}

        <div className="bg-white dark:bg-slate-900 rounded-xl shadow-sm border border-slate-200 dark:border-slate-800 p-6 space-y-4">
          <div className="space-y-2">
            <label className="text-sm font-medium text-slate-700 dark:text-slate-300">
              {locale === 'zh' ? '激活码' : 'Activation Code'}
            </label>
            <Input
              type="text"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              placeholder={locale === 'zh' ? '例如: PROMO-XXXX-YYYY' : 'e.g. PROMO-XXXX-YYYY'}
              disabled={redeeming || !user}
              className="font-mono text-base tracking-wider uppercase"
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !redeeming) handleRedeem();
              }}
            />
          </div>

          <Button
            onClick={handleRedeem}
            disabled={redeeming || !user || !code.trim()}
            className="w-full"
            size="lg"
          >
            {redeeming
              ? (locale === 'zh' ? '兑换中…' : 'Redeeming…')
              : (locale === 'zh' ? '立即激活' : 'Activate Now')}
          </Button>

          <div className="pt-3 border-t border-slate-100 dark:border-slate-800 space-y-2">
            <div className="text-xs text-slate-500 dark:text-slate-400">
              {locale === 'zh' ? '本机识别码 (machine_id)' : 'This Machine ID'}
            </div>
            <div className="flex items-center gap-2">
              <code className="flex-1 text-xs font-mono bg-slate-50 dark:bg-slate-800 px-3 py-2 rounded border border-slate-200 dark:border-slate-700 break-all">
                {machineId ?? (locale === 'zh' ? '加载中…' : 'Loading…')}
              </code>
              <Button
                variant="outline"
                size="sm"
                onClick={copyMid}
                disabled={!machineId}
              >
                {machineIdCopied
                  ? (locale === 'zh' ? '已复制' : 'Copied')
                  : (locale === 'zh' ? '复制' : 'Copy')}
              </Button>
            </div>
            <p className="text-[10px] text-slate-400 dark:text-slate-500 leading-relaxed">
              {locale === 'zh'
                ? '激活码将与本机识别码绑定, 一码仅限一机使用. 如需更换设备请联系客服解绑.'
                : 'The code is bound to this machine. One code = one device. Contact support to unbind.'}
            </p>
          </div>
        </div>

        <div className="text-center">
          <Link
            href="/pricing"
            className="text-xs text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 underline"
          >
            {locale === 'zh' ? '← 返回定价页' : '← Back to Pricing'}
          </Link>
        </div>
      </div>
    </main>
  );
}

function friendlyError(code: string | null, zh: boolean): string {
  const map: Record<string, { zh: string; en: string }> = {
    not_logged_in: {
      zh: '请先登录账号再兑换',
      en: 'Please log in first',
    },
    invalid_format: {
      zh: '激活码格式不正确, 请检查拼写',
      en: 'Invalid code format',
    },
    not_found: {
      zh: '激活码不存在, 请联系客服',
      en: 'Code not found',
    },
    expired: {
      zh: '激活码已过期, 请联系客服换发',
      en: 'Code expired',
    },
    already_used: {
      zh: '激活码已被使用, 请联系客服',
      en: 'Code already used',
    },
    machine_mismatch: {
      zh: '此激活码已绑定到其他设备, 请联系客服解绑',
      en: 'Code is bound to another device',
    },
    revoked: {
      zh: '激活码已被管理员撤销',
      en: 'Code revoked by admin',
    },
    db_error: {
      zh: '服务器临时错误, 请稍后重试',
      en: 'Server error, please retry',
    },
  };
  const c = (code ?? 'db_error') in map ? code! : 'db_error';
  return map[c][zh ? 'zh' : 'en'];
}
