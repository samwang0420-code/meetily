'use client';

import React from 'react';

/**
 * v0.6.10+: 卡片级 ErrorBoundary
 *
 * 问题背景: 工作台 RecentMeetings 区域偶尔因为某条 meeting 的字段为 null
 * (老数据 / 迁移脚本写入) 触发 React #321 (Cannot read properties of null)
 * 整张 main 树销毁, 用户必须重启 app.
 *
 * 解决: 给每张 meeting card 包一层卡级别 boundary, 单条崩了不影响其它卡片
 * 也不影响 main 树. fallback 渲染一个"该会议加载失败"的占位卡, 仍然
 * 可被点击但不跳转.
 *
 * 设计约束:
 * - 不依赖 react-error-boundary (仓库没装)
 * - componentDidCatch 不调 toast (参考 src/components/ErrorBoundary.tsx 注释)
 * - 失败信息打 console + localStorage, 用户粘 console 给我查即可
 */

interface CBState {
  failed: boolean;
  msg: string;
}

export class CardBoundary extends React.Component<
  { children: React.ReactNode; title?: string },
  CBState
> {
  state: CBState = { failed: false, msg: '' };

  static getDerivedStateFromError(error: Error): Partial<CBState> {
    return { failed: true, msg: error?.message || String(error) };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    const stack = String(error?.stack || '').slice(0, 4000);
    const comp = String(info?.componentStack || '').slice(0, 4000);
    console.error('[CardBoundary]', this.props.title || 'card', error,
      '\nstack:\n', stack, '\ncomponentStack:\n', comp);
    try {
      const buf = JSON.parse(localStorage.getItem('card-boundary-log') || '[]');
      buf.push({
        ts: Date.now(),
        title: this.props.title || 'card',
        msg: this.state.msg,
        stack: stack.slice(0, 1000),
        comp: comp.slice(0, 1000),
      });
      localStorage.setItem('card-boundary-log', JSON.stringify(buf.slice(-10)));
    } catch {}
  }

  render() {
    if (this.state.failed) {
      return (
        <div className="flex flex-col items-start gap-2 rounded-lg border border-red-200 bg-red-50/50 p-4 text-left">
          <div className="flex w-full items-center gap-2 text-[11px] text-red-500">
            <span className="rounded-full bg-red-100 px-2 py-0.5 font-medium uppercase tracking-wider">
              渲染失败
            </span>
            <span className="text-red-400">已隔离, 不影响其它会议</span>
          </div>
          <div className="line-clamp-2 text-[13px] text-red-700">
            {this.props.title || '未命名会议'}
          </div>
          <div className="text-[10px] text-red-400">
            请复制 console 报错给我们 (cmd+opt+I)
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
