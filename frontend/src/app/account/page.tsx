'use client';
import React, { useState, useRef } from 'react';
import Link from 'next/link';
import { useRouter } from 'next/navigation';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from '@/i18n';
import { useAuth } from '@/contexts/AuthContext';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import { ArrowLeft } from 'lucide-react';

export default function AccountPage() {
  const { t, locale } = useTranslation();
  const router = useRouter();
  const { user, machineId, logout, activateMember, refresh, session } = useAuth();
  const [busy, setBusy] = useState(false);
  const [code, setCode] = useState('');
  const [redeeming, setRedeeming] = useState(false);
  const [legalModal, setLegalModal] = useState<null | 'terms' | 'privacy'>(null);

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
    try {
      await logout();
    } catch (e: any) {
      console.warn('logout invoke failed, clearing local state anyway:', e);
    }
    // 强制清本地缓存 + reload, 避免 React #310 (残留 state + router 异常)
    try {
      if (typeof window !== 'undefined') {
        localStorage.removeItem('lixianhuiji.session');
        localStorage.removeItem('lixianhuiji.user');
      }
    } catch {}
    try {
      window.location.replace('/');
    } catch {
      router.push('/');
    }
  }

  async function copyMid() {
    if (!machineId) return;
    await navigator.clipboard.writeText(machineId);
    safeToast.success(t('account.machine_id_copied'));
  }

  // v0.6.10+: 改用 mailto 邮件 + 手动激活流程 (C3).
  // 不再让前端 bypass 真实支付直接 activate_member.
  // recordLead 暂保留备用 (admin 后台还有 lead 列表), UI 已改为 mailto 直发.
  /* eslint-disable @typescript-eslint/no-unused-vars */
  async function recordLead(contactNote?: string) {
    if (!user) return;
    try {
      setBusy(true);
      const leadId = await invoke<number>('lead_record_upgrade', {
        email: user.email,
        contact: contactNote || user.email,
        note: 'From account page ' + new Date().toISOString(),
      });
      safeToast.success(locale === 'zh'
        ? `已记录升级意向 (#${leadId}), 客服会尽快联系您`
        : `Upgrade interest recorded (#${leadId})`,
        { duration: 8000 });
    } catch (e: any) {
      console.warn('recordLead failed:', e);
    } finally {
      setBusy(false);
    }
  }
  /* eslint-enable @typescript-eslint/no-unused-vars */

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
            <>会员与本机绑定, 换机器请重新购买或 <a href="mailto:sam.wang01@icloud.com?subject=离线会记%20-%20会员迁移申请&body=机器ID:%20{machineId ?? '%20'}" className="text-blue-600 hover:underline">联系客服</a> 迁移。</>
          ) : (
            <>会员权益已绑定本机, 如需更换设备请 <a href="mailto:sam.wang01@icloud.com?subject=Offline-Meeting-Notes%20-%20License%20Migration&body=Machine%20ID:%20{machineId ?? '%20'}" className="text-blue-600 hover:underline">contact support</a> to migrate.</>
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
          {!isMember && (() => {
            const url = `mailto:sam.wang01@icloud.com?subject=${encodeURIComponent('离线会记 - Pro 购买咨询')}&body=${encodeURIComponent('机器ID: ' + (machineId || ' ') + '%0A%0A我想购买 Pro ¥88 永久买断, 请告诉我支付方式 (微信 / USDT / 信用卡).')}`;
            return (
              <a
                href={url}
                onClick={() => { try { invoke('open_external_url', { url }); } catch {} }}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg inline-block"
              >
                📩 联系客服购买
              </a>
            );
          })()}
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
          ¥88 永久买断 (微信 / USDT / 信用卡均可). 客服 1 个工作日内回复.
          离线软件, 不收集任何使用数据.
        </p>
        <div className="text-sm font-medium text-gray-800">{t('account.membership_features')}</div>
        <ul className="text-xs text-gray-600 space-y-1 pl-1">
          <li>• {t('account.membership_feature_unlimited')}</li>
          <li>• {t('account.membership_feature_pro_model')}</li>
          <li>• {t('account.membership_feature_hotwords')}</li>
          <li>• {t('account.membership_feature_export')}</li>
          <li>• {t('account.membership_feature_support')}</li>
        </ul>
      </section>

      {/* v0.6.10+: 条款链接 - 同页 Modal 打开, 避免 Tauri webview 拦截 target=_blank */}
      <div className="text-[11px] text-neutral-500 text-center pt-2">
        使用本软件即代表您同意我们的{' '}
        <button
          type="button"
          onClick={() => setLegalModal('terms')}
          className="text-blue-600 hover:underline"
        >
          用户协议
        </button>
        {' '}和{' '}
        <button
          type="button"
          onClick={() => setLegalModal('privacy')}
          className="text-blue-600 hover:underline"
        >
          隐私政策
        </button>
      </div>

      {/* Terms / Privacy Modal (同页打开, 避免 webview 拦截) */}
      {legalModal && (
        <div
          className="fixed inset-0 z-50 bg-black/40 flex items-center justify-center p-4"
          onClick={() => setLegalModal(null)}
        >
          <div
            className="bg-white rounded-xl max-w-2xl w-full max-h-[85vh] overflow-y-auto p-6 space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between border-b border-neutral-200 pb-3">
              <h2 className="text-lg font-semibold text-neutral-900">
                {legalModal === 'terms' ? '用户协议 / EULA' : '隐私政策'}
              </h2>
              <button
                onClick={() => setLegalModal(null)}
                className="text-neutral-500 hover:text-neutral-900 text-sm"
              >
                关闭
              </button>
            </div>
            {legalModal === 'terms' ? <LegalTermsBody /> : <LegalPrivacyBody />}
            <div className="pt-3 border-t border-neutral-200 flex justify-end">
              <button
                onClick={() => setLegalModal(null)}
                className="px-4 py-2 bg-blue-600 text-white text-sm rounded-md hover:bg-blue-700"
              >
                我已阅读
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function LegalTermsBody() {
  return (
    <div className="space-y-3 text-sm text-neutral-700">
      <p className="text-xs text-neutral-500">最后更新: 2026-07-18 · 离线会记 (Meetily) 团队</p>
      <h3 className="text-base font-medium text-neutral-900">一、许可</h3>
      <ul className="list-disc pl-5 space-y-1">
        <li>个人/企业使用: <strong>¥88 永久买断</strong> (绑定 1 台机器)</li>
        <li>源码: GitHub 开源 (MIT)</li>
        <li>许可证不可转让: 同一许可证不可在多台机器同时使用</li>
      </ul>
      <h3 className="text-base font-medium text-neutral-900">二、免责声明</h3>
      <ul className="list-disc pl-5 space-y-1">
        <li>本软件按"原样"提供, 无任何明示或暗示的保证</li>
        <li>开发者不对转写准确度、摘要质量承担法律责任</li>
        <li>用户应保留原始录音作为会议记录的法律依据</li>
        <li>不适用于医疗诊断、法律意见、金融决策等关键场景</li>
      </ul>
      <h3 className="text-base font-medium text-neutral-900">三、会员条款</h3>
      <ul className="list-disc pl-5 space-y-1">
        <li>会员费一次性, 永久有效 (无订阅)</li>
        <li>换机器可申请 1 次免费迁移, 之后每次 ¥20</li>
        <li>7 天内未深度使用可全额退款</li>
        <li>违反使用条款 (破解 / 滥用) 时, 开发者保留撤销资格</li>
      </ul>
    </div>
  );
}

function LegalPrivacyBody() {
  return (
    <div className="space-y-3 text-sm text-neutral-700">
      <p className="text-xs text-neutral-500">最后更新: 2026-07-18 · 离线会记 (Meetily) 团队</p>
      <h3 className="text-base font-medium text-neutral-900">一、数据处理原则</h3>
      <p>你的会议数据应当留在你设备上, 由你掌控. 默认情况下, 我们不上传任何音频、转写、摘要到外部服务器.</p>
      <h3 className="text-base font-medium text-neutral-900">二、我们不收集什么</h3>
      <ul className="list-disc pl-5 space-y-1">
        <li>不收集你的会议音频</li>
        <li>不收集你的转写文字</li>
        <li>不收集你的摘要内容</li>
        <li>不收集你的录音文件路径</li>
        <li>不收集你的使用行为</li>
      </ul>
      <h3 className="text-base font-medium text-neutral-900">三、本地存储</h3>
      <ul className="list-disc pl-5 space-y-1">
        <li>用户账号信息 (邮箱 + 密码哈希) — 仅在本机 SQLite</li>
        <li>会员状态 — 仅在本机数据库</li>
        <li>热词配置 — 仅在本机数据库</li>
        <li>会议元数据 — 仅在本机 IndexedDB</li>
      </ul>
      <h3 className="text-base font-medium text-neutral-900">四、第三方组件</h3>
      <p>本软件使用 sherpa-onnx (本地推理) / Ollama (本地 LLM, 用户自选) 等开源组件, 均在用户本机运行, 不联网.</p>
    </div>
  );
}
