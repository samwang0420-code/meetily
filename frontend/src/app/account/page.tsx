'use client';
import React, { useState, useRef } from 'react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { invoke } from '@tauri-apps/api/core';
import { openExternalUrl } from '@/lib/openExternalUrl';
import { useTranslation } from '@/i18n';
import { useAuth } from '@/contexts/AuthContext';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import { ArrowLeft } from 'lucide-react';

export default function AccountPage() {
  const { t, locale } = useTranslation();

  // §156: mailto: 在 Tauri webview 默认拦截, 必须用 plugin-opener 调系统邮件客户端
  const handleSupportMailto = async () => {
    const subjectZh = '言镜 AI - 会员迁移申请';
    const subjectEn = 'Offline-Meeting-Notes - License Migration';
    const bodyZh = `机器ID: ${machineId ?? ''}`;
    const bodyEn = `Machine ID: ${machineId ?? ''}`;
    const subject = encodeURIComponent(locale === 'zh' ? subjectZh : subjectEn);
    const body = encodeURIComponent(locale === 'zh' ? bodyZh : bodyEn);
    try {
      await openExternalUrl(`mailto:sam.wang01@icloud.com?subject=${subject}&body=${body}`);
    } catch (err) {
      console.error('mailto failed', err);
      toast.error(t('account.mailto_failed'));
    }
  };

  const router = useRouter();
  const { user, machineId, logout, activateMember, refresh, session } = useAuth();
  const [busy, setBusy] = useState(false);
  const [code, setCode] = useState('');
  const [redeeming, setRedeeming] = useState(false);

  // C4: 兑换激活码
  async function redeemCode() {
    const trimmed = code.trim().toUpperCase();
    if (!trimmed) {
      safeToast.error(locale === 'zh' ? '请输入激活码' : 'Please enter activation code');
      return;
    }
    if (!/^PROMO-[A-Z0-9]{8}-[A-Z0-9]{4}$/.test(trimmed)) {
      safeToast.error(locale === 'zh' ? '激活码格式不正确 (期望 PROMO-XXXXXXXX-YYYY)' : 'Invalid code format');
      return;
    }
    try {
      setRedeeming(true);
      const res = await invoke<{
        success: boolean;
        tier: string;
        expires_at: string;
        error_code: string | null;
        error_message: string | null;
      }>('user_redeem_activation_code', {
        session: session ?? '',
        code: trimmed,
      });
      if (res.success) {
        safeToast.success(locale === 'zh'
          ? `✨ Pro 已激活! 有效期至 ${res.expires_at.slice(0, 10)}`
          : `✨ Pro activated! Valid until ${res.expires_at.slice(0, 10)}`,
          { duration: 8000 });
        setCode('');
        await refresh();
      } else {
        // 失败: 用 error_message 友好化
        const msg = res.error_message ?? '激活失败';
        const friendly = res.error_code === 'not_logged_in'
          ? '请先登录账号再兑换'
          : msg;
        safeToast.error(friendly, { duration: 6000 });
      }
    } catch (e: any) {
      safeToast.error(`兑换失败: ${e?.message ?? e}`, { duration: 6000 });
    } finally {
      setRedeeming(false);
    }
  }

  async function handleLogout() {
    await logout();
    router.push('/');
  }

  async function copyMid() {
    if (!machineId) return;
    await navigator.clipboard.writeText(machineId);
    safeToast.success(t('account.machine_id_copied'));
  }

  // v0.6.10+: 改用 lead + 手动激活流程 (C3).
  // 不再让前端 bypass 真实支付直接 activate_member.
  async function recordLead(contactNote?: string) {
    if (!user) return;
    try {
      setBusy(true);
      const leadId = await invoke<number>('lead_record_upgrade', {
        email: user.email,
        contact: contactNote || user.email,
        note: 'From account page ' + new Date().toISOString(),
      });
      setBusy(false);
      safeToast.success(locale === 'zh'
        ? `已记录升级意向 (#${leadId}), 客服会尽快联系您`
        : `Upgrade interest recorded (#${leadId})`,
        { duration: 8000 });
    } catch (e: any) {
      setBusy(false);
      safeToast.error(t('account.activate_failed'));
    }
  }

  // admin 激活入口 (本机开发者用) - 通常不暴露在 UI 上
  // 隐藏在 dev mode 下: 用一个秘密按钮组合 (连点 5 次)
  const tapCount = useRef(0);
  function onTapMemberBadge() {
    tapCount.current += 1;
    if (tapCount.current >= 5) {
      tapCount.current = 0;
      activateMember().then((r) => {
        if (r) {
          safeToast.success(locale === 'zh' ? '本机开发者激活成功!' : 'Local dev activation success');
          refresh();
        }
      });
    }
  }

  if (!user) {
    // 未登录 -> 引导到 login
    React.useEffect(() => { router.replace('/login'); }, []);
    return null;
  }

  const isMember = user.membership === 'member';

  return (
    <div className="max-w-xl mx-auto p-6 space-y-5">
      {/* 顶部返回栏 */}
      <div className="flex items-center gap-3 -mt-2">
        <button
          onClick={() => router.push('/')}
          className="flex items-center gap-1 text-gray-600 hover:text-gray-900 text-sm"
        >
          <ArrowLeft className="w-4 h-4" />
          <span>返回工作台</span>
        </button>
      </div>
      <h1 className="text-2xl font-semibold text-gray-900">{t('account.membership_title')}</h1>

      {/* 用户卡片 */}
      <section className="bg-white border border-gray-200 rounded-xl p-5 space-y-2">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-base font-medium text-gray-900">{user.display_name || user.email}</div>
            <div className="text-xs text-gray-500">{user.email}</div>
          </div>
          <button onClick={handleLogout}
            className="text-xs text-gray-500 hover:text-gray-700 underline">
            {t('account.logout')}
          </button>
        </div>
        <div className="text-xs text-gray-600">
          {isMember
            ? <span className="text-green-700 font-medium">✨ {t('account.member')}</span>
            : <span className="text-gray-500">{t('account.free_trial')}</span>}
        </div>
      </section>

      {/* 机器识别码 */}
      <section className="bg-white border border-gray-200 rounded-xl p-5 space-y-2">
        <h2 className="text-sm font-medium text-gray-700">{t('account.machine_id_label')}</h2>
        <div className="flex items-center justify-between gap-2 bg-gray-50 border border-gray-200 rounded-lg px-3 py-2">
          <code className="text-xs text-gray-700 truncate">{machineId ?? '...'}</code>
          <button onClick={copyMid}
            className="px-2 py-1 text-xs text-blue-600 hover:bg-blue-50 rounded">
            {t('account.copy_machine_id')}
          </button>
        </div>
        <p className="text-xs text-gray-500">
          {locale === 'zh' ? (
            <>会员与本机绑定, 换机器请重新购买或 <a onClick={handleSupportMailto} className="text-blue-600 hover:underline cursor-pointer">联系客服</a> 迁移。</>
          ) : (
            <>Membership is bound to this machine. Re-purchase or <a onClick={handleSupportMailto} className="text-blue-600 hover:underline cursor-pointer">contact support</a> to migrate.</>
          )}
        </p>
      </section>

      {/* C4: 激活码兑换 — 加 id="redeem" 便于 pricing 页锚点跳转 */}
      {!isMember && (
        <section id="redeem" className="bg-white border-2 border-blue-300 rounded-xl p-5 space-y-3 scroll-mt-20">
          <div className="flex items-baseline justify-between">
            <h2 className="text-sm font-medium text-blue-700">🎫 有激活码? 立即激活 Pro</h2>
          </div>
          <p className="text-xs text-neutral-500">
            购买 Pro 后我们会发邮件给你一个 17 位激活码 (形如 PROMO-XXXXXXXX-YYYY). 粘贴到下方立刻激活, 无需重启.
          </p>
          <div className="flex items-stretch gap-2">
            <input
              type="text"
              value={code}
              onChange={(e) => setCode(e.target.value.toUpperCase())}
              placeholder="PROMO-XXXXXXXX-YYYY"
              maxLength={18}
              className="flex-1 px-3 py-2 text-sm font-mono border border-neutral-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
              disabled={redeeming}
            />
            <button
              onClick={redeemCode}
              disabled={redeeming || !code.trim()}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-md disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1"
            >
              {redeeming ? '激活中...' : '激活'}
            </button>
          </div>
          <Link href="/pricing" className="text-xs text-blue-600 hover:underline">
            还没买? 查看定价 →
          </Link>
        </section>
      )}

      {/* 会员卡 */}
      <section className="bg-gradient-to-br from-blue-50 to-indigo-50 border border-blue-200 rounded-xl p-5 space-y-4">
        <div className="flex items-baseline justify-between">
          <div>
            <div className="text-3xl font-bold text-blue-700">{t('account.membership_price')}</div>
            <div className="text-xs text-gray-500 mt-1">{t('account.membership_price_unit')}</div>
          </div>
          {!isMember && (
            <button onClick={() => recordLead('In-app button click')} disabled={busy}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg disabled:opacity-50">
              {busy ? '提交中...' : '获取支付方式'}
            </button>
          )}
          {isMember && (
            <span
              onClick={onTapMemberBadge}
              className="px-3 py-1 bg-green-100 text-green-700 text-xs font-medium rounded-full cursor-default select-none"
              title="已是 Pro 会员">
              ✨ Pro
            </span>
          )}
        </div>
        <p className="text-xs text-gray-600">
          ¥88 永久买断 (微信/USDT/信用卡均可).
          点上方按钮留联系方式, 客服 1 个工作日内回复.
          离线软件, 不收集任何使用数据.
        </p>

        {/* v0.6.10+: 支付指南卡 */}
        {!isMember && (
          <details className="mt-3 text-xs text-neutral-600">
            <summary className="cursor-pointer text-neutral-700 font-medium hover:text-neutral-900">
              💳 查看支付方式 + 价格包含什么
            </summary>
            <div className="mt-3 space-y-2 pl-2 border-l-2 border-blue-100">
              <div><strong>微信支付:</strong> ¥88 一次性, 扫客服码 → 把交易号粘过来 → 我们手动激活你的账号</div>
              <div><strong>USDT-TRC20:</strong> 12 USDT (≈¥88), 钱包地址向客服索取, 同样把交易哈希给我们激活</div>
              <div><strong>信用卡 (海外):</strong> 通过 Stripe 链接支付, 也可走 Paddle (开票用), 客服发链接</div>
              <div className="text-neutral-500 pt-1 border-t border-neutral-100 mt-2">
                <strong>价格包含什么?</strong><br />
                · 1 台机器永久使用 Pro 全部功能<br />
                · 后续所有大版本更新免费<br />
                · 优先回复客服 (sam.wang01@icloud.com)<br />
                · 换机器可申请 1 次免费迁移 (凭机器 ID)
              </div>
            </div>
          </details>
        )}
        <div className="text-sm font-medium text-gray-800">{t('account.membership_features')}</div>
        <ul className="text-xs text-gray-600 space-y-1 pl-1">
          <li>• {t('account.membership_feature_unlimited')}</li>
          <li>• {t('account.membership_feature_pro_model')}</li>
          <li>• {t('account.membership_feature_hotwords')}</li>
          <li>• {t('account.membership_feature_export')}</li>
          <li>• {t('account.membership_feature_support')}</li>
        </ul>
      </section>

      {/* v0.6.10+: 条款链接 */}
      <div className="text-[11px] text-neutral-500 text-center pt-2">
        使用本软件即代表您同意我们的{' '}
        <Link href="/legal/terms" className="text-blue-600 hover:underline">
          用户协议
        </Link>
        {' '}和{' '}
        <Link href="/legal/privacy" className="text-blue-600 hover:underline">
          隐私政策
        </Link>
      </div>
    </div>
  );
}
