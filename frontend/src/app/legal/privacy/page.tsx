'use client';

import React from 'react';
import { openExternalUrl } from '@/lib/openExternalUrl';

// 离线会记 v0.6.10+: 隐私政策页面
// 主要内容: 100% 本地处理, 不上传任何数据
// 给 C5 用 (GDPR / 国内合规)

export default function PrivacyPage() {
  return (
    <div className="max-w-3xl mx-auto p-6 prose prose-neutral dark:prose-invert">
      <h1 className="text-2xl font-semibold tracking-tight text-neutral-900">隐私政策</h1>
      <p className="text-sm text-neutral-500">
        最后更新: 2026-07-18 · 离线会记 (Meetily) 团队
      </p>

      <h2 className="mt-6 text-lg font-medium text-neutral-900">一、数据处理原则</h2>
      <p className="text-sm text-neutral-700">
        离线会记的设计原则是: <strong>你的会议数据应当留在你设备上, 由你掌控</strong>.
        默认情况下, 我们不上传任何音频、转写、摘要到外部服务器.
      </p>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">二、我们不收集什么</h2>
      <ul className="text-sm text-neutral-700 list-disc pl-6 space-y-1">
        <li>不收集你的会议音频</li>
        <li>不收集你的转写文字</li>
        <li>不收集你的摘要内容</li>
        <li>不收集你的录音文件路径</li>
        <li>不收集你的使用行为</li>
      </ul>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">三、本地存储</h2>
      <ul className="text-sm text-neutral-700 list-disc pl-6 space-y-1">
        <li>用户账号信息 (邮箱 + 密码哈希) — 仅在本机 SQLite</li>
        <li>会员状态 — 仅在本机数据库</li>
        <li>热词配置 — 仅在本机数据库</li>
        <li>会议元数据 — 仅在本机 IndexedDB</li>
      </ul>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">三之二、本地行为分析 (v0.7.0+)</h2>
      <p className="text-sm text-neutral-700">
        为帮助我们改进产品, 离线会记会在本机 SQLite 中记录以下事件 (<code>analytics_events</code> 表):
      </p>
      <ul className="text-sm text-neutral-700 list-disc pl-6 space-y-1">
        <li>功能使用: 录音开始/停止、摘要生成、模板选择等</li>
        <li>错误信息: 摘要生成失败、模型加载失败等 (不含原始数据)</li>
        <li>设备/版本信息: OS、架构、应用版本号</li>
      </ul>
      <p className="text-sm text-neutral-700 mt-2">
        <strong>承诺</strong>: 这些数据<strong>仅保存在本机数据库</strong>, 不会上传到任何服务器.
        你可以随时在"设置 → 隐私 → 本地行为分析"里关闭此功能, 关闭后不会再写入新事件.
      </p>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">三之三、崩溃日志 (v0.7.0+)</h2>
      <p className="text-sm text-neutral-700">
        如果软件发生严重错误 (panic), 我们会在本地的 <code>crashes/</code> 目录写入一份崩溃报告:
      </p>
      <ul className="text-sm text-neutral-700 list-disc pl-6 space-y-1">
        <li>路径: <code>~/Library/Application Support/tech.yanjingai.app/crashes/</code> (macOS) / <code>%APPDATA%\tech.yanjingai.app\crashes\</code> (Windows) / <code>~/.local/share/tech.yanjingai.app/crashes/</code> (Linux)</li>
        <li>内容: 时间戳、版本号、操作系统、panic 信息、Rust 调用栈</li>
        <li>不包含: 你的会议音频、转写文字、摘要内容</li>
        <li>保留策略: 仅保留最近 50 个崩溃文件, 超出自动清理</li>
      </ul>
      <p className="text-sm text-neutral-700 mt-2">
        <strong>承诺</strong>: 崩溃日志同样<strong>仅保存在本机</strong>, 不上传. 如果你主动邮件发送给我们用于问题排查, 我们仅用于修复该具体问题, 不做其他用途.
      </p>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">四、可选的非本地操作</h2>
      <p className="text-sm text-neutral-700">
        离线会记默认使用本地 ASR 模型, 但用户可在设置里切换到云端 ASR 模型 (Deepgram, OpenAI 等).
        切换时, 音频会被发送到对应 API. 这是用户<strong>主动</strong>选择的行为, 默认不会发生.
      </p>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">五、付费账户</h2>
      <p className="text-sm text-neutral-700">
        付费数据 (邮箱、支付凭证) 仅通过客户主动沟通 (微信 / 邮件) 收集, 仅用于激活 Pro 会员,
        不会用于任何第三方营销或数据共享.
      </p>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">六、你的权利 (GDPR 风格)</h2>
      <ul className="text-sm text-neutral-700 list-disc pl-6 space-y-1">
        <li>随时导出你所有数据 (账号设置里提供导出按钮)</li>
        <li>随时删除账号 (联系我们 / 或在账号设置里自删除)</li>
        <li>随时撤回账号信息 (会员激活后会保留最小必要数据用于会员状态查询)</li>
      </ul>

      <h2 className="mt-4 text-lg font-medium text-neutral-900">七、联系方式</h2>
      <p className="text-sm text-neutral-700">
        任何隐私问题, 邮件 <a href="mailto:sam.wang01@icloud.com" onClick={(e) => { e.preventDefault(); openExternalUrl('mailto:sam.wang01@icloud.com'); }} className="text-blue-600 hover:underline cursor-pointer">sam.wang01@icloud.com</a>,
        我们会在 7 个工作日内回复.
      </p>

      <p className="mt-8 text-xs text-neutral-400">
        本政策与仓库根目录 <code>PRIVACY_POLICY.md</code> 同步.
      </p>
    </div>
  );
}
