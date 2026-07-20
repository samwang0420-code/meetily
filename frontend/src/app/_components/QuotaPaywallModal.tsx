'use client';

// v0.6.10+: 商业化付费墙弹窗
// 触发: 用户点击录音但配额已满 (C1 匿名 / C2 免费)
// 引导: 注册 / 登录 / 升级 Pro

import React from 'react';
import { useRouter } from 'next/navigation';
import { X, Sparkles, UserPlus, LogIn, ShieldCheck } from 'lucide-react';

interface Props {
  open: boolean;
  reason: 'anonymous_trial_exhausted' | 'free_monthly_limit_reached' | null;
  onClose: () => void;
  onUpgradeInterest: () => void;
}

export function QuotaPaywallModal({ open, reason, onClose, onUpgradeInterest }: Props) {
  const router = useRouter();
  if (!open) return null;

  const isAnon = reason === 'anonymous_trial_exhausted';

  return (
    <div
      role="dialog"
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div className="relative max-w-md w-full bg-white rounded-2xl shadow-xl border border-neutral-200 overflow-hidden">
        {/* close */}
        <button
          aria-label="关闭"
          onClick={onClose}
          className="absolute top-3 right-3 p-1.5 rounded-md text-neutral-400 hover:text-neutral-700 hover:bg-neutral-100"
        >
          <X className="h-4 w-4" />
        </button>

        {/* hero icon */}
        <div className="flex flex-col items-center pt-8 pb-4 px-6">
          <div className="relative">
            <div className="absolute inset-0 rounded-full bg-gradient-to-br from-blue-400 to-indigo-500 blur-xl opacity-30" />
            <div className="relative h-16 w-16 rounded-full bg-gradient-to-br from-blue-500 to-indigo-600 flex items-center justify-center shadow-lg">
              <Sparkles className="h-7 w-7 text-white" />
            </div>
          </div>
          <h2 className="mt-4 text-xl font-semibold text-neutral-900 tracking-tight">
            {isAnon ? '试用次数已用完' : '本月免费额度已用完'}
          </h2>
          <p className="mt-1.5 text-[13px] text-neutral-500 text-center">
            {isAnon
              ? '你已使用了 1 次免费试用, 注册账号即可每月继续 5 次免费录音额度。'
              : '本月 5 次免费录音额度已用完。升级 Pro 解锁无限录音 + 高级功能。'}
          </p>
        </div>

        {/* tier 对照 */}
        <div className="px-6 pb-2">
          <div className="grid grid-cols-2 gap-3">
            <div className="rounded-lg border border-neutral-200 p-3 bg-neutral-50">
              <div className="flex items-baseline justify-between">
                <span className="text-[11px] uppercase tracking-wider text-neutral-500 font-medium">
                  免费
                </span>
                <span className="text-base font-semibold text-neutral-900">¥0</span>
              </div>
              <ul className="mt-2 text-[11.5px] text-neutral-600 space-y-0.5">
                <li>• 每月 5 次录音</li>
                <li>• 每次转写 100 段</li>
                <li>• SenseVoice-zh 中文模型</li>
                <li>• 5 大行业热词</li>
              </ul>
            </div>
            <div className="rounded-lg border-2 border-blue-500 p-3 bg-blue-50/30 relative">
              <div className="absolute -top-1.5 -right-1.5 px-1.5 py-0.5 bg-blue-600 text-white text-[9px] font-semibold rounded">
                推荐
              </div>
              <div className="flex items-baseline justify-between">
                <span className="text-[11px] uppercase tracking-wider text-blue-700 font-medium">
                  Pro
                </span>
                <span className="text-base font-semibold text-blue-700">¥88</span>
              </div>
              <ul className="mt-2 text-[11.5px] text-neutral-700 space-y-0.5">
                <li>• <strong>无限录音</strong></li>
                <li>• <strong>FunASR-Nano</strong> 实验</li>
                <li>• 说话人分离 (短会议)</li>
                <li>• 优先级客服</li>
              </ul>
            </div>
          </div>
        </div>

        {/* CTAs */}
        <div className="px-6 py-5 space-y-2.5">
          {isAnon ? (
            <>
              <button
                onClick={() => { router.push('/register'); onClose(); }}
                className="w-full flex items-center justify-center gap-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium py-2.5"
              >
                <UserPlus className="h-4 w-4" />
                免费注册账号
              </button>
              <button
                onClick={() => { router.push('/login'); onClose(); }}
                className="w-full flex items-center justify-center gap-2 rounded-lg bg-white hover:bg-neutral-50 text-neutral-700 text-sm font-medium py-2.5 border border-neutral-200"
              >
                <LogIn className="h-4 w-4" />
                已有账号 · 登录
              </button>
            </>
          ) : (
            <>
              <button
                onClick={() => { router.push('/account'); onClose(); }}
                className="w-full flex items-center justify-center gap-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium py-2.5"
              >
                <Sparkles className="h-4 w-4" />
                查看 Pro 升级
              </button>
              <button
                onClick={() => { onUpgradeInterest(); onClose(); }}
                className="w-full flex items-center justify-center gap-2 rounded-lg bg-white hover:bg-neutral-50 text-neutral-700 text-sm font-medium py-2.5 border border-neutral-200"
              >
                留个联系方式, 我想升级
              </button>
            </>
          )}
        </div>

        {/* privacy footer */}
        <div className="px-6 py-3 bg-neutral-50 border-t border-neutral-100 flex items-center gap-1.5 text-[10.5px] text-neutral-500">
          <ShieldCheck className="h-3 w-3 text-emerald-600" />
          <span>100% 本地离线 · 数据不上传 · 任何时候可导出删除</span>
        </div>
      </div>
    </div>
  );
}
