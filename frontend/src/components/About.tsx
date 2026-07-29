import React, { useState, useEffect } from "react";
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { Shield, Cpu, Wallet, Globe2, Sparkles, Loader2, CheckCircle2, ArrowUpRight, Mic, FileText, Lock, Mail, Copy, Send } from "lucide-react";
import { UpdateDialog } from "./UpdateDialog";
import { FeedbackDialog } from "./FeedbackDialog";
import { updateService, UpdateInfo } from '@/services/updateService';
import { Button } from './ui/button';
import { toast } from 'sonner';
import { safeToast } from '@/lib/safeToast';

export function About() {
    const [currentVersion, setCurrentVersion] = useState<string>('0.6.7');
    const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
    const [isChecking, setIsChecking] = useState(false);
    const [showUpdateDialog, setShowUpdateDialog] = useState(false);
    const [feedbackOpen, setFeedbackOpen] = useState(false);

    useEffect(() => {
        getVersion().then(setCurrentVersion).catch(console.error);
    }, []);

    // 离线会记 v0.6.11: 客服联系方式 (sam.wang01@icloud.com)
    const SUPPORT_EMAIL = 'sam.wang01@icloud.com';

    const handleContactClick = async () => {
        // 优先尝试 mailto: 唤起本机邮件客户端; 失败 fallback 浏览器
        const mailtoUrl = `mailto:${SUPPORT_EMAIL}?subject=${encodeURIComponent('离线会记 - 商务定制咨询')}&body=${encodeURIComponent('您好,\n我想为团队咨询本地 AI 工具定制。\n\n[请简要描述团队规模 / 行业 / 核心需求]\n')}`;
        try {
            await invoke('open_external_url', { url: mailtoUrl });
        } catch (error) {
            console.error('Failed to open mailto:', error);
            // fallback: 浏览器打开
            try {
                await invoke('open_external_url', { url: `https://mail.google.com/mail/?view=cm&to=${SUPPORT_EMAIL}` });
            } catch (e2) {
                console.error('Failed to open browser mail:', e2);
            }
        }
    };

    const handleSupportClick = async () => {
        const mailtoUrl = `mailto:${SUPPORT_EMAIL}?subject=${encodeURIComponent('离线会记 - 用户反馈')}&body=${encodeURIComponent('版本 v0.6.11 · macOS\n问题描述:\n')}`;
        try {
            await invoke('open_external_url', { url: mailtoUrl });
        } catch (error) {
            console.error('Failed to open support mailto:', error);
        }
    };

    const handleCheckForUpdates = async () => {
        setIsChecking(true);
        try {
            const info = await updateService.checkForUpdates(true);
            setUpdateInfo(info);
            if (info.available) {
                setShowUpdateDialog(true);
            } else {
                safeToast.success('你已经在运行最新版本');
            }
        } catch (error: any) {
            console.error('Failed to check for updates:', error);
            safeToast.error('检查更新失败: ' + (error.message || 'Unknown error'));
        } finally {
            setIsChecking(false);
        }
    };

    const features = [
        {
            icon: Lock,
            title: '隐私优先',
            desc: '所有录音、语音识别、AI 纪要全程本地运行, 音频与文本绝不离开你的设备。',
            accent: 'from-app-recording/15 to-app-recording/5 text-app-recording',
        },
        {
            icon: Cpu,
            title: '模型自由',
            desc: '默认使用本地开源 SenseVoice / Paraformer, 也可接入 Ollama 或任意 OpenAI 兼容 API。',
            accent: 'from-app-transcript/15 to-app-transcript/5 text-app-transcript',
        },
        {
            icon: Wallet,
            title: '成本可控',
            desc: '无需支付云端按分钟计费, 模型一次下载永久本地使用, 后续零调用费。',
            accent: 'from-app-summary/15 to-app-summary/5 text-app-summary-deep',
        },
        {
            icon: Globe2,
            title: '全平台兼容',
            desc: '钉钉、飞书、腾讯会议、Zoom、Teams、WebEx 任意会议软件都能转录。',
            accent: 'from-blue-500/15 to-blue-500/5 text-blue-600',
        },
    ];

    const stats = [
        { value: '100%', label: '本地运算' },
        { value: '6+', label: '支持平台' },
        { value: '< 200ms', label: '转录延迟' },
        { value: '∞', label: '免费使用' },
    ];

    return (
        <div className="h-[80vh] overflow-y-auto bg-gradient-to-b from-transparent to-neutral-50/40 dark:to-neutral-950/40">
            <div className="max-w-2xl mx-auto px-6 py-8 space-y-8">

                {/* Hero - 居中 logo + 标题 + 副标题 + 版本号 */}
                <section className="text-center pt-2">
                    <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-gradient-to-br from-app-transcript to-app-summary shadow-lg shadow-app-transcript/20 mb-4">
                        <Mic className="w-7 h-7 text-white" strokeWidth={2} />
                    </div>
                    <h1 className="text-2xl font-semibold tracking-tight text-neutral-900 dark:text-neutral-50">
                        离线会记
                    </h1>
                    <p className="mt-2 text-[14px] text-neutral-500 dark:text-neutral-400 max-w-md mx-auto leading-relaxed">
                        会议转录与 AI 纪要全程本地运算, 隐私数据从不离开你的设备。
                    </p>
                    <div className="mt-3 inline-flex items-center gap-1.5 rounded-full border border-neutral-200/80 dark:border-neutral-800 bg-white/60 dark:bg-neutral-900/60 px-2.5 py-1 text-[11px] font-medium text-neutral-500 dark:text-neutral-400">
                        <span className="h-1.5 w-1.5 rounded-full bg-emerald-500"></span>
                        v{currentVersion} · MIT 协议
                    </div>
                </section>

                {/* Stats - 4 个等宽数据 */}
                <section className="grid grid-cols-4 gap-2 rounded-xl border border-neutral-200/80 dark:border-neutral-800 bg-white dark:bg-neutral-900 p-4">
                    {stats.map((s) => (
                        <div key={s.label} className="text-center">
                            <div className="text-lg font-semibold text-neutral-900 dark:text-neutral-50 tracking-tight">
                                {s.value}
                            </div>
                            <div className="text-[11px] text-neutral-500 dark:text-neutral-400 mt-0.5">
                                {s.label}
                            </div>
                        </div>
                    ))}
                </section>

                {/* Features - 4 个优势卡片, 统一样式 */}
                <section>
                    <div className="flex items-baseline justify-between mb-3 px-1">
                        <h2 className="text-[13px] font-semibold uppercase tracking-wider text-neutral-500 dark:text-neutral-400">
                          核心优势
                        </h2>
                        <span className="text-[11px] text-neutral-400">v0.6 重构</span>
                    </div>
                    <div className="grid grid-cols-2 gap-3">
                        {features.map((f) => {
                            const Icon = f.icon;
                            return (
                                <div
                                    key={f.title}
                                    className="rounded-xl border border-neutral-200/80 dark:border-neutral-800 bg-white dark:bg-neutral-900 p-4 hover:border-neutral-300 dark:hover:border-neutral-700 transition-colors"
                                >
                                    <div className={`inline-flex w-9 h-9 items-center justify-center rounded-lg bg-gradient-to-br ${f.accent} mb-3`}>
                                        <Icon className="w-4 h-4" strokeWidth={2} />
                                    </div>
                                    <h3 className="text-[13.5px] font-semibold text-neutral-900 dark:text-neutral-50 mb-1.5">
                                        {f.title}
                                    </h3>
                                    <p className="text-[12px] text-neutral-500 dark:text-neutral-400 leading-relaxed">
                                        {f.desc}
                                    </p>
                                </div>
                            );
                        })}
                    </div>
                </section>

                {/* Coming Soon - 渐变条 */}
                <section className="rounded-xl border border-app-summary/30 bg-gradient-to-r from-app-summary/10 via-app-summary/5 to-transparent p-4 flex items-start gap-3">
                    <div className="flex-shrink-0 mt-0.5">
                        <Sparkles className="w-4 h-4 text-app-summary-deep" />
                    </div>
                    <div className="flex-1">
                        <div className="text-[12.5px] font-semibold text-app-summary-deep mb-0.5">
                            即将推出
                        </div>
                        <p className="text-[12px] text-neutral-700 dark:text-neutral-300 leading-relaxed">
                            本机 AI 助理自动生成待办、跟进项, 持续迭代国产 ASR 与企业管控能力。
                        </p>
                    </div>
                </section>

                {/* Check Update - 中等按钮 */}
                <section className="flex flex-col items-center gap-2 py-2">
                    <Button
                        onClick={handleCheckForUpdates}
                        disabled={isChecking}
                        variant="outline"
                        size="sm"
                        className="gap-1.5 text-[12.5px]"
                    >
                        {isChecking ? (
                            <>
                                <Loader2 className="h-3.5 w-3.5 animate-spin" />
                                检查中...
                            </>
                        ) : (
                            <>
                                <CheckCircle2 className="h-3.5 w-3.5" />
                                检查更新
                            </>
                        )}
                    </Button>
                    {updateInfo?.available && (
                        <div className="text-[12px] text-blue-600 dark:text-blue-400">
                            有新版本: v{updateInfo.version}
                        </div>
                    )}
                </section>

                {/* CTA - 联系定制 */}
                <section className="rounded-xl border border-neutral-200/80 dark:border-neutral-800 bg-white dark:bg-neutral-900 p-5 text-center">
                    <h3 className="text-[15px] font-semibold text-neutral-900 dark:text-neutral-50 mb-1.5">
                        准备为团队打造专属本地 AI 工具?
                    </h3>
                    <p className="text-[12.5px] text-neutral-500 dark:text-neutral-400 mb-3.5 leading-relaxed max-w-md mx-auto">
                        如果你正在为律所 / 金融 / 研发 / 国企构建隐私优先的本地 AI 工具, 我们可以为你定制。
                    </p>
                    <button
                        onClick={handleContactClick}
                        className="inline-flex items-center gap-1.5 px-4 py-2 bg-neutral-900 hover:bg-neutral-800 text-white text-[12.5px] font-medium rounded-md transition-colors shadow-sm hover:shadow-md dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-100"
                    >
                        联系我们定制
                        <ArrowUpRight className="h-3.5 w-3.5" />
                    </button>
                </section>

                {/* v0.6.11: 联系客服 / 意见反馈 (独立入口) */}
                <section className="rounded-xl border border-blue-200/80 bg-blue-50/50 dark:border-blue-900/40 dark:bg-blue-950/20 p-5">
                    <div className="flex items-start justify-between gap-3 mb-2">
                        <div>
                            <h3 className="text-[15px] font-semibold text-neutral-900 dark:text-neutral-50">
                                联系客服 / 意见反馈
                            </h3>
                            <p className="text-[12.5px] text-neutral-500 dark:text-neutral-400 mt-1 leading-relaxed">
                                使用中遇到问题 / 想要新功能 — 直接告诉我们, 通常 24h 内回复。
                            </p>
                        </div>
                        <Send className="h-5 w-5 text-blue-500 shrink-0" />
                    </div>
                    <div className="flex flex-col sm:flex-row sm:items-center gap-2 mt-3">
                        <button
                            onClick={() => setFeedbackOpen(true)}
                            className="inline-flex items-center justify-center gap-1.5 px-4 py-2.5 bg-blue-600 hover:bg-blue-700 text-white text-[13px] font-medium rounded-md transition-colors shadow-sm"
                        >
                            <Send className="h-3.5 w-3.5" />
                            提交结构化反馈
                        </button>
                        <button
                            onClick={handleSupportClick}
                            className="inline-flex items-center justify-center gap-1.5 px-4 py-2.5 border border-neutral-300 dark:border-neutral-700 hover:bg-white dark:hover:bg-neutral-800 text-[13px] font-medium rounded-md transition-colors"
                            title="直接发邮件, 模板由你手写"
                        >
                            <Mail className="h-3.5 w-3.5" />
                            直接发邮件
                        </button>
                        <button
                            onClick={async () => {
                                try {
                                    await navigator.clipboard.writeText(SUPPORT_EMAIL);
                                    safeToast.success('邮箱已复制');
                                } catch (e) {
                                    console.error(e);
                                    safeToast.error('复制失败');
                                }
                            }}
                            className="inline-flex items-center justify-center gap-1.5 px-3 py-2.5 border border-neutral-200 hover:bg-neutral-50 text-[12.5px] font-medium rounded-md transition-colors"
                            title="复制邮箱地址"
                        >
                            <Copy className="h-3 w-3" />
                            复制邮箱
                        </button>
                    </div>
                </section>

                {/* Footer */}
                <div className="pt-2 text-center">
                    <p className="text-[11px] text-neutral-400">
                        基于开源 Zackriya-Solutions/meetily 二次改造, 保留 MIT 版权声明
                    </p>
                </div>
            </div>

            

            <FeedbackDialog


              open={feedbackOpen}


              onOpenChange={setFeedbackOpen}


              defaultType="feature"


            />



            <UpdateDialog
                open={showUpdateDialog}
                onOpenChange={setShowUpdateDialog}
                updateInfo={updateInfo}
            />
        </div>
    );
}
