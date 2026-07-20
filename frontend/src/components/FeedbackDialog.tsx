"use client";

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';
import { Lightbulb, Bug, Frown, MessageSquare, Send, X } from 'lucide-react';

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

const SUPPORT_EMAIL = 'sam.wang01@icloud.com';

interface FeedbackDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  defaultType?: FeedbackType;
}

type FeedbackType = 'bug' | 'feature' | 'experience' | 'other';

const TYPE_OPTIONS: { value: FeedbackType; label: string; desc: string; icon: React.ReactNode; color: string }[] = [
  {
    value: 'bug',
    label: '功能故障 / Bug',
    desc: '录音、转录、保存有异常',
    icon: <Bug className="h-4 w-4" />,
    color: 'text-red-600 bg-red-50 border-red-200',
  },
  {
    value: 'feature',
    label: '功能建议',
    desc: '想要的新功能 / 改进',
    icon: <Lightbulb className="h-4 w-4" />,
    color: 'text-amber-600 bg-amber-50 border-amber-200',
  },
  {
    value: 'experience',
    label: '体验问题',
    desc: '操作不顺手 / 视觉不舒服',
    icon: <Frown className="h-4 w-4" />,
    color: 'text-blue-600 bg-blue-50 border-blue-200',
  },
  {
    value: 'other',
    label: '其他',
    desc: '商务合作 / 媒体咨询 / 其他',
    icon: <MessageSquare className="h-4 w-4" />,
    color: 'text-neutral-600 bg-neutral-50 border-neutral-200',
  },
];

export function FeedbackDialog({ open, onOpenChange, defaultType = 'feature' }: FeedbackDialogProps) {
  const [type, setType] = useState<FeedbackType>(defaultType);
  const [title, setTitle] = useState('');
  const [detail, setDetail] = useState('');
  const [contact, setContact] = useState('');
  const [includeDiagnostics, setIncludeDiagnostics] = useState(true);
  const [submitting, setSubmitting] = useState(false);

  const reset = () => {
    setType(defaultType);
    setTitle('');
    setDetail('');
    setContact('');
    setIncludeDiagnostics(true);
  };

  const handleSubmit = async () => {
    // 校验
    if (!title.trim()) {
      safeToast.error('请填写一句话标题');
      return;
    }
    if (!detail.trim() || detail.trim().length < 10) {
      safeToast.error('详情至少 10 个字, 帮我们快速定位');
      return;
    }

    setSubmitting(true);
    try {
      // 构建结构化邮件内容
      const typeLabel = TYPE_OPTIONS.find(o => o.value === type)?.label ?? type;
      const lines: string[] = [];
      lines.push(`类型: ${typeLabel}`);
      lines.push(`版本: 离线会记 v0.6.11 · macOS`);
      lines.push(`时间: ${new Date().toLocaleString('zh-CN')}`);
      if (contact.trim()) lines.push(`联系方式: ${contact.trim()}`);
      lines.push('');
      lines.push('--- 反馈详情 ---');
      lines.push(`标题: ${title.trim()}`);
      lines.push('');
      lines.push(detail.trim());
      lines.push('');
      if (includeDiagnostics) {
        lines.push('--- 环境信息 (供参考) ---');
        lines.push(`User Agent: ${navigator.userAgent}`);
        lines.push(`Language: ${navigator.language}`);
        lines.push(`Viewport: ${window.innerWidth}x${window.innerHeight}`);
        lines.push(`Online: ${navigator.onLine}`);
      }
      const body = lines.join('\n');

      const subject = `[${typeLabel}] ${title.trim()}`;
      const mailtoUrl = `mailto:${SUPPORT_EMAIL}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;

      try {
        await invoke('open_external_url', { url: mailtoUrl });
        safeToast.success('已唤起邮件客户端, 发送后我们会尽快回复', {
          description: `收件人: ${SUPPORT_EMAIL}`,
          duration: 5000,
        });
      } catch (e) {
        console.warn('mailto 唤起失败, fallback 复制到剪贴板:', e);
        try {
          await navigator.clipboard.writeText(`收件人: ${SUPPORT_EMAIL}\n主题: ${subject}\n\n${body}`);
          safeToast.success('邮件唤起失败, 内容已复制到剪贴板', {
            description: `请打开邮箱粘贴发送至 ${SUPPORT_EMAIL}`,
            duration: 6000,
          });
        } catch (e2) {
          safeToast.error('请手动复制反馈内容并发送至 ' + SUPPORT_EMAIL);
        }
      }
      reset();
      onOpenChange(false);
    } catch (e: any) {
      safeToast.error('提交失败: ' + (e?.message ?? String(e)));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!o) reset(); onOpenChange(o); }}>
      <DialogContent className="sm:max-w-[560px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="text-[16px] flex items-center gap-2">
            <Send className="h-4 w-4 text-blue-600" />
            意见反馈
          </DialogTitle>
          <DialogDescription className="text-[12.5px]">
            你的反馈直接发到产品负责人邮箱 — 通常 24h 内回复。
          </DialogDescription>
        </DialogHeader>

        {/* 类型选择 */}
        <div className="space-y-1.5">
          <Label className="text-[12.5px]">反馈类型</Label>
          <div className="grid grid-cols-2 gap-2">
            {TYPE_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                type="button"
                onClick={() => setType(opt.value)}
                className={`flex items-start gap-2 rounded-lg border px-2.5 py-2 text-left transition-all ${
                  type === opt.value
                    ? opt.color + ' ring-1 ring-offset-0 ring-current'
                    : 'border-neutral-200 hover:border-neutral-300 hover:bg-neutral-50/50'
                }`}
              >
                <span className={`mt-0.5 shrink-0 ${type === opt.value ? '' : 'text-neutral-500'}`}>
                  {opt.icon}
                </span>
                <div className="min-w-0 flex-1">
                  <div className="text-[12.5px] font-medium leading-tight">{opt.label}</div>
                  <div className="text-[10.5px] text-neutral-500 leading-tight mt-0.5">{opt.desc}</div>
                </div>
              </button>
            ))}
          </div>
        </div>

        {/* 标题 */}
        <div className="space-y-1.5">
          <Label htmlFor="fb-title" className="text-[12.5px]">
            一句话标题 <span className="text-red-500">*</span>
          </Label>
          <Input
            id="fb-title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="例: 录音停止后字幕没刷新"
            maxLength={80}
            className="text-[13px]"
          />
        </div>

        {/* 详情 */}
        <div className="space-y-1.5">
          <Label htmlFor="fb-detail" className="text-[12.5px]">
            详细描述 <span className="text-red-500">*</span>
            <span className="ml-1.5 text-[10.5px] text-neutral-400">(至少 10 字, 推荐 30-200 字)</span>
          </Label>
          <Textarea
            id="fb-detail"
            value={detail}
            onChange={(e) => setDetail(e.target.value)}
            placeholder={'请描述:\n• 你在做什么操作\n• 期待看到什么\n• 实际看到什么\n• 截图路径 (可选, 贴在下面)'}
            rows={6}
            maxLength={2000}
            className="text-[13px] resize-none box-border max-w-full"
          />
          <div className="flex justify-between items-center text-[10.5px] text-neutral-400">
            <span>{detail.length} / 2000 字</span>
            {detail.length > 1500 && <span className="text-amber-600">接近字数上限</span>}
          </div>
        </div>

        {/* 联系方式 */}
        <div className="space-y-1.5">
          <Label htmlFor="fb-contact" className="text-[12.5px]">
            联系方式 <span className="text-[10.5px] text-neutral-400">(可选, 便于追问细节)</span>
          </Label>
          <Input
            id="fb-contact"
            value={contact}
            onChange={(e) => setContact(e.target.value)}
            placeholder="微信号 / 邮箱 / 手机 (任一)"
            maxLength={120}
            className="text-[13px]"
          />
        </div>

        {/* 诊断信息 */}
        <div className="flex items-start gap-2 rounded-lg bg-neutral-50 p-2.5">
          <input
            type="checkbox"
            id="fb-diag"
            checked={includeDiagnostics}
            onChange={(e) => setIncludeDiagnostics(e.target.checked)}
            className="mt-0.5 h-3.5 w-3.5 cursor-pointer accent-blue-600"
          />
          <label htmlFor="fb-diag" className="text-[11px] text-neutral-600 leading-snug cursor-pointer">
            附带环境信息 (浏览器版本、屏幕尺寸等) — 帮助我们快速复现问题
          </label>
        </div>

        <DialogFooter className="gap-2 sm:gap-2">
          <Button
            variant="ghost"
            onClick={() => onOpenChange(false)}
            disabled={submitting}
            className="flex-1"
          >
            取消
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={submitting || !title.trim() || detail.trim().length < 10}
            className="flex-1 bg-blue-600 hover:bg-blue-700 text-white"
          >
            {submitting ? '提交中…' : (
              <>
                <Send className="h-3.5 w-3.5 mr-1.5" />
                发送反馈
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
