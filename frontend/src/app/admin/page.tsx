'use client';

// {t('admin.version_label')}: Admin 后台 (C7)
// - 看 upgrade_leads (用户点了"我想升级"留下的意向)
// - 看 activation_orders (已{t('admin.stat_orders')})
// - 手动激活用户 (输入 email + 通道 + 凭证)

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'next/navigation';
import { toast } from 'sonner';
import { useTranslation } from '@/i18n';
import { safeToast } from '@/lib/safeToast';
import { ShieldCheck, ArrowLeft, Users, CreditCard, Mail, RefreshCw, CheckCircle2 } from 'lucide-react';

interface OrderRow {
  id: number;
  email: string;
  amount_cents: number;
  channel: string;
  proof: string | null;
  operator_email: string | null;
  created_at: string;
  notes: string | null;
}

interface CodeRow {
  id: number;
  code: string;
  tier: string;
  duration_days: number;
  expires_at: string;
  used_by_user_id: number | null;
  used_at: string | null;
  generated_by_operator: string | null;
  note: string | null;
  created_at: string;
}

interface LeadRow {
  id: number;
  email: string;
  contact: string | null;
  created_at: string;
}

interface AdminUserRow {
  id: number;
  email: string;
  membership: string;
  machine_id: string;
  month_meetings_used: number;
  is_active: boolean;
}

export default function AdminPage() {
  const { t } = useTranslation();
  const router = useRouter();
  const [token, setToken] = useState('');
  const [loggedIn, setLoggedIn] = useState(false);
  const [orders, setOrders] = useState<OrderRow[]>([]);
  const [leads, setLeads] = useState<LeadRow[]>([]);
  const [loading, setLoading] = useState(false);

  // 激活表单
  // C4: 激活码管理
  const [genCount, setGenCount] = useState(5);
  const [genNote, setGenNote] = useState('');
  const [genDuration, setGenDuration] = useState(365);
  const [codes, setCodes] = useState<CodeRow[]>([]);
  const [codesLoading, setCodesLoading] = useState(false);

  const refreshCodes = async () => {
    try {
      setCodesLoading(true);
      const rows = await invoke<CodeRow[]>('admin_list_activation_codes', { operatorToken: token, limit: 100, offset: 0 });
      setCodes(rows);
    } catch (e: any) {
      safeToast.error(t('admin.err_load_codes') + ': ' + (e?.message ?? e));
    } finally {
      setCodesLoading(false);
    }
  };

  const handleGenerateCodes = async () => {
    if (genCount < 1 || genCount > 200) return safeToast.warning('1..=200');
    try {
      setCodesLoading(true);
      const masked = await invoke<string[]>('admin_generate_activation_codes', {
        operatorToken: token,
        count: genCount,
        tier: 'member',
        durationDays: genDuration,
        note: genNote || null,
      });
      safeToast.success(t('admin.success_codes_generated', { count: masked.length }));
      await refreshCodes();
    } catch (e: any) {
      safeToast.error(t('admin.err_generate') + ': ' + (e?.message ?? e));
    } finally {
      setCodesLoading(false);
    }
  };

  const handleRevokeCode = async (code: string) => {
    if (!confirm(`${t('admin.codes_revoke_confirm')} ${code}${t('admin.codes_revoke_suffix')}`)) return;
    try {
      const affected = await invoke<number>('admin_revoke_activation_code', { operatorToken: token, code });
      safeToast.success(t('admin.success_revoked', { count: affected }));
      await refreshCodes();
    } catch (e: any) {
      safeToast.error(t('admin.err_revoke') + ': ' + (e?.message ?? e));
    }
  };

  // v0.7.0+: 用户管理 (退款 / 解绑 / 封号)
  const [users, setUsers] = useState<AdminUserRow[]>([]);
  const [usersLoading, setUsersLoading] = useState(false);
  const refreshUsers = async () => {
    try {
      setUsersLoading(true);
      const rows = await invoke<AdminUserRow[]>('admin_list_users', { operatorToken: token, limit: 100 });
      setUsers(rows);
    } catch (e: any) {
      safeToast.error('加载用户失败: ' + (e?.message ?? e));
    } finally {
      setUsersLoading(false);
    }
  };
  const handleRefund = async (uid: number, email: string) => {
    if (!confirm(`确认退款并撤销 ${email} 的会员?`)) return;
    try {
      await invoke<boolean>('admin_refund_user', { operatorToken: token, userId: uid, reason: '客户退款' });
      safeToast.success(`已退款 ${email}`);
      await refreshUsers();
    } catch (e: any) {
      safeToast.error('退款失败: ' + (e?.message ?? e));
    }
  };
  const handleUnbind = async (uid: number, email: string) => {
    if (!confirm(`解绑 ${email} 的机器? (会员保留, 用户可在新机器激活)`)) return;
    try {
      await invoke<boolean>('admin_unbind_machine', { operatorToken: token, userId: uid });
      safeToast.success(`已解绑 ${email}`);
      await refreshUsers();
    } catch (e: any) {
      safeToast.error('解绑失败: ' + (e?.message ?? e));
    }
  };
  const handleToggleActive = async (uid: number, email: string, active: boolean) => {
    if (!confirm(`${active ? '解封' : '封号'} ${email}?`)) return;
    try {
      await invoke<boolean>('admin_set_user_active', { operatorToken: token, userId: uid, active });
      safeToast.success(`${active ? '已解封' : '已封号'} ${email}`);
      await refreshUsers();
    } catch (e: any) {
      safeToast.error('操作失败: ' + (e?.message ?? e));
    }
  };

  const [activateEmail, setActivateEmail] = useState('');
  const [activateChannel, setActivateChannel] = useState('wxpay');
  const [activateProof, setActivateProof] = useState('');
  const [activateNotes, setActivateNotes] = useState('');

  const refresh = async () => {
    setLoading(true);
    try {
      const [o, l, cs] = await Promise.all([
        invoke<OrderRow[]>('admin_list_activation_orders', { operatorToken: token, limit: 100 }),
        invoke<LeadRow[]>('admin_list_upgrade_leads', { operatorToken: token, limit: 100 }),
        invoke<CodeRow[]>('admin_list_activation_codes', { operatorToken: token, limit: 100, offset: 0 }),
      ]);
      setOrders(o);
      setLeads(l);
      setCodes(cs);
    } catch (e: any) {
      safeToast.error(t('admin.err_auth_or_query') + ': ' + e?.message);
    } finally {
      setLoading(false);
    }
  };

  const handleActivate = async () => {
    if (!activateEmail) return safeToast.warning(t('admin.err_need_email'));
    try {
      setLoading(true);
      const ok = await invoke<boolean>('admin_activate_member', {
        req: {
          operator_token: token,
          email: activateEmail,
          amount_cents: 8800,
          channel: activateChannel,
          proof: activateProof || null,
          notes: activateNotes || null,
        },
      });
      if (ok) {
        safeToast.success(`${t('admin.success_activated')} ${activateEmail} ${t('admin.success_activated_suffix')}`);
        setActivateEmail('');
        setActivateProof('');
        setActivateNotes('');
        refresh();
      } else {
        safeToast.error(t('admin.err_activate'));
      }
    } catch (e: any) {
      safeToast.error(t('admin.err_activate') + ': ' + e?.message);
    } finally {
      setLoading(false);
    }
  };

  if (!loggedIn) {
    return (
      <div className="max-w-md mx-auto p-6 mt-20">
        <div className="bg-white border border-neutral-200 rounded-2xl p-6 shadow-sm">
          <div className="flex items-center gap-2 mb-4">
            <ShieldCheck className="h-5 w-5 text-blue-600" />
            <h1 className="text-lg font-semibold text-neutral-900">{t('admin.console')}</h1>
          </div>
          <p className="text-xs text-neutral-500 mb-4">
            {t('admin.page_subtitle')}
          </p>
          <input
            type="password"
            value={token}
            onChange={(e) => setToken(e.target.value)}
            placeholder="operator token"
            className="w-full px-3 py-2 border border-neutral-200 rounded-md text-sm mb-3"
          />
          <button
            disabled={!token}
            onClick={async () => {
              try {
                await refresh();
                setLoggedIn(true);
              } catch { /* refresh 自己已 toast */ }
            }}
            className="w-full bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium py-2 rounded-md disabled:opacity-50"
          >
            {t('admin.enter')}
          </button>
          <button onClick={() => router.push('/')}
            className="mt-3 w-full text-neutral-600 hover:text-neutral-800 text-xs flex items-center justify-center gap-1">
            <ArrowLeft className="h-3 w-3" />
            {t('admin.back_workspace')}
          </button>
        </div>
      </div>
    );
  }

  const totalRevenue = orders.reduce((s, o) => s + o.amount_cents, 0) / 100;

  return (
    <div className="max-w-5xl mx-auto p-6">
      {/* header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-2">
          <ShieldCheck className="h-5 w-5 text-blue-600" />
          <h1 className="text-lg font-semibold text-neutral-900">{t('admin.console')}</h1>
          <span className="text-xs text-neutral-500">{t('admin.version_label')}</span>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={refresh} disabled={loading}
            className="flex items-center gap-1 px-3 py-1.5 text-xs rounded-md bg-neutral-100 hover:bg-neutral-200 text-neutral-700 disabled:opacity-50">
            <RefreshCw className={`h-3 w-3 ${loading ? 'animate-spin' : ''}`} />
            {t('admin.refresh')}
          </button>
          <button onClick={() => router.push('/')}
            className="text-xs text-neutral-500 hover:text-neutral-800 flex items-center gap-1">
            <ArrowLeft className="h-3 w-3" />
            {t('admin.back')}
          </button>
        </div>
      </div>

      {/* stats */}
      <div className="grid grid-cols-3 gap-3 mb-6">
        <div className="bg-white border border-neutral-200 rounded-lg p-4">
          <div className="text-xs text-neutral-500">{t('admin.stat_orders')}</div>
          <div className="text-2xl font-semibold text-neutral-900 mt-1">{orders.length}</div>
        </div>
        <div className="bg-white border border-neutral-200 rounded-lg p-4">
          <div className="text-xs text-neutral-500">{t('admin.stat_leads')}</div>
          <div className="text-2xl font-semibold text-neutral-900 mt-1">{leads.length}</div>
        </div>
        <div className="bg-white border border-neutral-200 rounded-lg p-4">
          <div className="text-xs text-neutral-500">{t('admin.stat_revenue')}</div>
          <div className="text-2xl font-semibold text-emerald-600 mt-1">¥{totalRevenue.toFixed(0)}</div>
        </div>
      </div>

      {/* 激活表单 */}
      <section className="bg-white border border-neutral-200 rounded-xl p-5 mb-6">
        <h2 className="text-sm font-medium text-neutral-900 mb-3 flex items-center gap-1.5">
          <CreditCard className="h-4 w-4" />
          {t('admin.activate_title')}
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-2">
          <input
            placeholder={t('admin.activate_email_label')}
            value={activateEmail}
            onChange={(e) => setActivateEmail(e.target.value)}
            className="px-3 py-2 border border-neutral-200 rounded-md text-sm md:col-span-2"
          />
          <select
            value={activateChannel}
            onChange={(e) => setActivateChannel(e.target.value)}
            className="px-3 py-2 border border-neutral-200 rounded-md text-sm"
          >
            <option value="wxpay">{t('admin.channel_wxpay')}</option>
            <option value="usdt">USDT-TRC20</option>
            <option value="card">{t('admin.channel_card')}</option>
            <option value="admin_grant">{t('admin.channel_admin_grant')}</option>
          </select>
          <button onClick={handleActivate} disabled={loading}
            className="bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium py-2 rounded-md disabled:opacity-50 flex items-center justify-center gap-1">
            <CheckCircle2 className="h-4 w-4" />
            {t('admin.activate_submit')}
          </button>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-2 mt-2">
          <input
            placeholder={t('admin.activate_proof_label')}
            value={activateProof}
            onChange={(e) => setActivateProof(e.target.value)}
            className="px-3 py-2 border border-neutral-200 rounded-md text-xs"
          />
          <input
            placeholder={t('admin.activate_notes_label')}
            value={activateNotes}
            onChange={(e) => setActivateNotes(e.target.value)}
            className="px-3 py-2 border border-neutral-200 rounded-md text-xs"
          />
        </div>
      </section>

      {/* C4: 激活码管理 */}
      <section className="bg-white border border-neutral-200 rounded-xl p-5 mb-6">
        <h2 className="text-sm font-medium text-neutral-900 mb-3 flex items-center gap-1.5">
          <CreditCard className="h-4 w-4" />
          {t('admin.codes_title')}
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-2 mb-3">
          <input
            type="number"
            placeholder={t('admin.codes_count_label')}
            value={genCount}
            onChange={(e) => setGenCount(parseInt(e.target.value || '0'))}
            min={1}
            max={200}
            className="px-3 py-2 border border-neutral-200 rounded-md text-sm"
          />
          <input
            type="number"
            placeholder={t('admin.codes_duration_label')}
            value={genDuration}
            onChange={(e) => setGenDuration(parseInt(e.target.value || '365'))}
            min={1}
            max={3650}
            className="px-3 py-2 border border-neutral-200 rounded-md text-sm"
          />
          <input
            placeholder={t('admin.codes_notes_label')}
            value={genNote}
            onChange={(e) => setGenNote(e.target.value)}
            className="px-3 py-2 border border-neutral-200 rounded-md text-sm md:col-span-1"
          />
          <button onClick={handleGenerateCodes} disabled={codesLoading}
            className="bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium py-2 rounded-md disabled:opacity-50">
            {t('admin.codes_generate_btn')} {genCount} {t('admin.codes_count_suffix')}
          </button>
        </div>
        <div className="border-t border-neutral-100 pt-3">
          <h3 className="text-xs font-medium text-neutral-600 mb-2">{t('admin.codes_list_title')}</h3>
          {codesLoading && codes.length === 0 ? (
            <div className="text-xs text-neutral-400">{t('admin.codes_loading')}</div>
          ) : codes.length === 0 ? (
            <div className="text-xs text-neutral-400">{t('admin.codes_empty')}</div>
          ) : (
            <div className="max-h-64 overflow-auto divide-y divide-neutral-100">
              {codes.map((c) => (
                <div key={c.id} className="py-2 flex items-center justify-between text-xs">
                  <div className="flex-1 truncate">
                    <div className="font-mono text-neutral-700">
                      {c.used_by_user_id
                        ? <span className="text-green-700">PROMO-****-USED</span>
                        : <span>{c.code}</span>}
                    </div>
                    <div className="text-neutral-500 text-[11px]">
                      {c.used_by_user_id
                        ? t('admin.code_used', { uid: c.used_by_user_id, time: c.used_at ?? '' })
                        : t('admin.code_unused', { days: c.duration_days, date: c.expires_at.slice(0, 10) })}
                      {c.note && <span className="ml-2 text-neutral-400">· {c.note}</span>}
                    </div>
                  </div>
                  {!c.used_by_user_id && (
                    <button
                      onClick={() => handleRevokeCode(c.code)}
                      className="ml-2 px-2 py-1 text-red-600 hover:bg-red-50 rounded text-[11px]"
                    >
                      {t('admin.codes_revoke_btn')}
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </section>

      {/* upgrade leads */}
      <section className="bg-white border border-neutral-200 rounded-xl p-5 mb-6">
        <h2 className="text-sm font-medium text-neutral-900 mb-3 flex items-center gap-1.5">
          <Mail className="h-4 w-4" />
          {t('admin.stat_leads')} 
        </h2>
        {leads.length === 0 ? (
          <p className="text-xs text-neutral-500 py-3 text-center">{t('admin.empty')}</p>
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="text-left text-neutral-500 border-b border-neutral-100">
                <th className="py-2 font-medium">{t('common.id')}</th>
                <th className="py-2 font-medium">{t('admin.th.email')}</th>
                <th className="py-2 font-medium">{t('admin.th.contact')}</th>
                <th className="py-2 font-medium">{t('admin.th.time')}</th>
              </tr>
            </thead>
            <tbody>
              {leads.map((l) => (
                <tr key={l.id} className="border-b border-neutral-50 hover:bg-neutral-50">
                  <td className="py-2 text-neutral-400">{l.id}</td>
                  <td className="py-2 text-neutral-900">{l.email}</td>
                  <td className="py-2 text-neutral-700">{l.contact ?? '-'}</td>
                  <td className="py-2 text-neutral-500 text-[10px]">{l.created_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {/* v0.7.0+: 用户管理 (退款/解绑/封号) */}
      <section className="bg-white border border-neutral-200 rounded-xl p-5 mb-6">
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-sm font-medium text-neutral-900 flex items-center gap-1.5">
            <Users className="h-4 w-4" />
            用户管理（退款 / 解绑 / 封号）
          </h2>
          <button
            onClick={refreshUsers}
            disabled={usersLoading || !token}
            className="flex items-center gap-1 px-2 py-1 text-xs rounded bg-neutral-100 hover:bg-neutral-200 disabled:opacity-50"
          >
            <RefreshCw className={`h-3 w-3 ${usersLoading ? 'animate-spin' : ''}`} />
            刷新
          </button>
        </div>
        {users.length === 0 ? (
          <p className="text-xs text-neutral-500 py-3 text-center">{usersLoading ? '加载中…' : '点击刷新加载用户'}</p>
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="text-left text-neutral-500 border-b border-neutral-100">
                <th className="py-2 font-medium">{t('common.id')}</th>
                <th className="py-2 font-medium">邮箱</th>
                <th className="py-2 font-medium">会员</th>
                <th className="py-2 font-medium">机器</th>
                <th className="py-2 font-medium">本月</th>
                <th className="py-2 font-medium">状态</th>
                <th className="py-2 font-medium">操作</th>
              </tr>
            </thead>
            <tbody>
              {users.map((u) => (
                <tr key={u.id} className="border-b border-neutral-50 hover:bg-neutral-50">
                  <td className="py-2 text-neutral-400">{u.id}</td>
                  <td className="py-2 text-neutral-900">{u.email}</td>
                  <td className="py-2">
                    <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${u.membership === 'member' ? 'bg-amber-100 text-amber-700' : 'bg-neutral-100 text-neutral-600'}`}>
                      {u.membership}
                    </span>
                  </td>
                  <td className="py-2 text-neutral-500 font-mono text-[10px]">{u.machine_id || '—'}</td>
                  <td className="py-2 text-neutral-700">{u.month_meetings_used}</td>
                  <td className="py-2">
                    <span className={`px-1.5 py-0.5 rounded text-[10px] ${u.is_active ? 'bg-green-100 text-green-700' : 'bg-red-100 text-red-700'}`}>
                      {u.is_active ? '正常' : '封禁'}
                    </span>
                  </td>
                  <td className="py-2 space-x-1">
                    {u.membership === 'member' && (
                      <button onClick={() => handleRefund(u.id, u.email)} className="px-1.5 py-0.5 bg-amber-100 text-amber-700 rounded text-[10px] hover:bg-amber-200">
                        退款
                      </button>
                    )}
                    {u.machine_id && (
                      <button onClick={() => handleUnbind(u.id, u.email)} className="px-1.5 py-0.5 bg-blue-100 text-blue-700 rounded text-[10px] hover:bg-blue-200">
                        解绑
                      </button>
                    )}
                    <button onClick={() => handleToggleActive(u.id, u.email, !u.is_active)} className={`px-1.5 py-0.5 rounded text-[10px] ${u.is_active ? 'bg-red-100 text-red-700 hover:bg-red-200' : 'bg-green-100 text-green-700 hover:bg-green-200'}`}>
                      {u.is_active ? '封号' : '解封'}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {/* orders */}
      <section className="bg-white border border-neutral-200 rounded-xl p-5">
        <h2 className="text-sm font-medium text-neutral-900 mb-3 flex items-center gap-1.5">
          <Users className="h-4 w-4" />
          {t('admin.orders_title')}
        </h2>
        {orders.length === 0 ? (
          <p className="text-xs text-neutral-500 py-3 text-center">{t('admin.empty')}</p>
        ) : (
          <table className="w-full text-xs">
            <thead>
              <tr className="text-left text-neutral-500 border-b border-neutral-100">
                <th className="py-2 font-medium">{t('common.id')}</th>
                <th className="py-2 font-medium">{t('admin.th.email')}</th>
                <th className="py-2 font-medium">{t('admin.th.amount')}</th>
                <th className="py-2 font-medium">{t('admin.th.channel')}</th>
                <th className="py-2 font-medium">{t('admin.th.proof')}</th>
                <th className="py-2 font-medium">{t('admin.th.time')}</th>
              </tr>
            </thead>
            <tbody>
              {orders.map((o) => (
                <tr key={o.id} className="border-b border-neutral-50 hover:bg-neutral-50">
                  <td className="py-2 text-neutral-400">{o.id}</td>
                  <td className="py-2 text-neutral-900">{o.email}</td>
                  <td className="py-2 text-emerald-600 font-medium">¥{(o.amount_cents / 100).toFixed(0)}</td>
                  <td className="py-2">
                    <span className="px-1.5 py-0.5 bg-blue-100 text-blue-700 rounded text-[10px]">
                      {o.channel}
                    </span>
                  </td>
                  <td className="py-2 text-neutral-600 font-mono text-[10px]">{o.proof ?? '-'}</td>
                  <td className="py-2 text-neutral-500 text-[10px]">{o.created_at}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
