'use client';

import React from 'react';

// 离线会记 v0.6.10+: 用户协议
// 简单 EULA — 本软件 "按原样" 提供, 不保证无错

export default function TermsPage() {
  return (
    <div className="max-w-3xl mx-auto p-6 space-y-4 text-neutral-900">
      <button
        type="button"
        onClick={() => { try { window.history.length > 1 ? window.history.back() : (window.location.href = '/'); } catch { window.location.href = '/'; } }}
        className="flex items-center gap-1 text-sm text-neutral-600 hover:text-neutral-900"
      >
        <span aria-hidden>←</span>
        <span>{`返回`}</span>
      </button>
      <h1 className="text-2xl font-semibold tracking-tight text-neutral-900">用户协议 / EULA</h1>
      <p className="text-sm text-neutral-500">
        最后更新: 2026-07-18 · 离线会记 (Meetily) 团队
      </p>

      <h2 className="mt-6 text-lg font-medium text-neutral-900">一、许可</h2>
      <p className="text-sm text-neutral-700">
        离线会记按以下方式许可:
      </p>
      <ul className="text-sm text-neutral-700 list-disc pl-6 space-y-1">
        <li>个人/企业使用: <strong className="font-semibold text-neutral-900">¥88 永久买断</strong> (绑定 1 台机器)</li>
        <li>源码: <a href="https://github.com/meetily/meetily" className="text-blue-600 hover:underline">GitHub</a> 开源 (MIT)</li>
        <li>许可证不可转让: 同一许可证不可在多台机器同时使用</li>
      </ul>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">二、免责声明</h2>
      <ul className="text-sm text-neutral-700 list-disc pl-6 space-y-1">
        <li>本软件按"原样"提供, 无任何明示或暗示的保证</li>
        <li>开发者不对转写准确度、摘要质量承担法律责任</li>
        <li>用户应保留原始录音作为会议记录的法律依据</li>
        <li>不适用于医疗诊断、法律意见、金融决策等关键场景</li>
      </ul>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">三、会员条款</h2>
      <ul className="text-sm text-neutral-700 list-disc pl-6 space-y-1">
        <li>会员费一次性, 永久有效 (无订阅)</li>
        <li>换机器可以申请 1 次免费迁移, 之后每次 ¥20</li>
        <li>7 天内 (含) 未深度使用可全额退款</li>
        <li>违反使用条款 (破解 / 滥用) 时, 开发者保留撤销资格</li>
        <li>硬件识别码 (machine_id) 基于主板 UUID + 主机名生成, 仅在本机 SQLite 保存, 不上传任何服务器</li>
        <li>极旧硬件 (例如 2010 前无 UUID 的设备) 可能读取失败, 此时会生成随机标识, 重装系统后可能丢失授权 — 如遇此情况请联系客服 <a href="mailto:sam.wang01@icloud.com" className="text-blue-600 hover:underline">sam.wang01@icloud.com</a> 人工迁移</li>
      </ul>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">四、数据归属</h2>
      <p className="text-sm text-neutral-700">
        用户保留其在本软件内生成的所有数据 (音频、转写、摘要) 的全部权利.
        软件仅作为本地工具辅助处理.
      </p>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">五、争议解决</h2>
      <p className="text-sm text-neutral-700">
        适用中国大陆法律. 争议优先通过友好协商解决, 协商不成提交开发者所在地法院.
      </p>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">六、联系方式</h2>
      <p className="text-sm text-neutral-700">
        任何条款问题, 邮件 <a href="mailto:sam.wang01@icloud.com" className="text-blue-600 hover:underline">sam.wang01@icloud.com</a>.
      </p>
    </div>
  );
}
