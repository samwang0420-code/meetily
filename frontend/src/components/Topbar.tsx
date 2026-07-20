'use client';

import * as React from 'react';
import { useRouter, usePathname } from 'next/navigation';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import * as Popover from '@radix-ui/react-popover';
import { useTranslation, Locale } from '@/i18n';
import { useAuth } from '@/contexts/AuthContext';
import {
  Check, ChevronDown, Globe, Settings, LogOut, User as UserIcon,
  Sparkles, Command, Search, Mic, Plus, X
} from 'lucide-react';

/* ─────────────────────────────────────────────────────────────────
   Topbar v2 (2026-Q3) — Linear / Raycast / Vercel 设计语言
   核心改造：
   - 全宽 sticky + 半透明 bg + 16px blur backdrop
   - 左侧 34px brand mark + 12px 字间距产品名 (Inter Geist 自定义)
   - 中央 Cmd+K 命令面板触发器 (替代搜索框, 减少视觉噪音)
   - 右侧 icon-only 圆形 ghost button (无圆角胶囊)
   - 单色 / 多色 token: 专注模式不抢戏
───────────────────────────────────────────────────────────────── */

const CRUMB_LABEL: Record<Locale, Record<string, string>> = {
  zh: {
    '': '工作台',
    home: '工作台',
    notes: '会议笔记',
    'meeting-details': '会议详情',
    settings: '设置',
    'settings/hotwords': '热词',
    account: '账户',
    login: '登录',
    register: '注册',
    summary: 'AI 纪要',
    transcripts: '转写记录',
  },
  en: {
    '': 'Workspace',
    home: 'Workspace',
    notes: 'Meeting Notes',
    'meeting-details': 'Meeting Details',
    settings: 'Settings',
    'settings/hotwords': 'Hotwords',
    account: 'Account',
    login: 'Sign in',
    register: 'Sign up',
    summary: 'AI Summary',
    transcripts: 'Transcripts',
  },
};

const LOCALE_LABEL: Record<Locale, string> = { zh: '中文', en: 'English' };
const LOCALE_SUB: Record<Locale, string> = { zh: '简体中文', en: 'English (US)' };

/* ─── 1. Language Switcher ─── icon-only ghost button + Popover menu */
function LanguageSwitcher() {
  const { locale, setLocale, t } = useTranslation();
  return (
    <Popover.Root>
      <Popover.Trigger asChild>
        <button
          aria-label={t('topbar.switch_language')}
          className="
            group flex h-9 w-9 items-center justify-center rounded-md
            text-neutral-600 transition-colors
            hover:bg-neutral-100 hover:text-neutral-900
            focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40
            dark:text-neutral-400 dark:hover:bg-neutral-800 dark:hover:text-neutral-100
          "
        >
          <Globe className="h-[18px] w-[18px]" strokeWidth={1.75} />
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          align="end"
          sideOffset={8}
          className="
            z-50 w-56 overflow-hidden rounded-lg
            border border-neutral-200/80 bg-white/95 p-1 shadow-lg shadow-neutral-900/5
            backdrop-blur
            dark:border-neutral-800 dark:bg-neutral-900/95 dark:shadow-black/40
          "
        >
          <div className="px-3 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-wider text-neutral-400">
            {t('topbar.language_title')}
          </div>
          {(['zh', 'en'] as Locale[]).map((code) => {
            const active = code === locale;
            return (
              <button
                key={code}
                onClick={() => setLocale(code)}
                className={`
                  flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-sm
                  transition-colors
                  ${active
                    ? 'bg-blue-50 text-blue-700 dark:bg-blue-500/15 dark:text-blue-300'
                    : 'text-neutral-700 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800'}
                `}
              >
                <span className="flex flex-col">
                  <span className="text-[13px] font-medium">{LOCALE_LABEL[code]}</span>
                  <span className="text-[11px] text-neutral-500 dark:text-neutral-400">{LOCALE_SUB[code]}</span>
                </span>
                {active && <Check className="h-4 w-4 text-blue-600 dark:text-blue-400" strokeWidth={2.5} />}
              </button>
            );
          })}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}

/* ─── 2. Quick Record FAB (top-right primary action, never hidden) ─── */
function RecordButton() {
  const { t, locale } = useTranslation();
  return (
    <button
      onClick={() => window.dispatchEvent(new CustomEvent('lixianhuiji:toggle-recording'))}
      className="
        group flex h-9 items-center gap-2 rounded-md px-3
        bg-neutral-900 text-white
        transition-all
        hover:bg-neutral-800 hover:shadow-md hover:shadow-neutral-900/10
        focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40
        dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-100
      "
    >
      <Mic className="h-[15px] w-[15px]" strokeWidth={2} />
      <span className="hidden text-[12.5px] font-medium tracking-tight sm:inline">
        {t('topbar.record')}
      </span>
      <kbd className="
        ml-1 hidden rounded border border-white/20 bg-white/10 px-1 text-[10px] font-mono
        opacity-0 transition-opacity group-hover:opacity-100
        md:inline-block
      ">
        ⌘R
      </kbd>
    </button>
  );
}

/* ─── 3. User Menu ─── avatar-only trigger, kbd accelerator hint */
function UserMenu() {
  const { t, locale } = useTranslation();
  const router = useRouter();
  const { user, logout } = useAuth();

  if (!user) {
    return (
      <button
        onClick={() => router.push('/login')}
        className="
          flex h-9 items-center gap-2 rounded-md px-3
          bg-blue-600 text-[13px] font-medium text-white
          transition-colors hover:bg-blue-700
          focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40
        "
      >
        <UserIcon className="h-[15px] w-[15px]" strokeWidth={2} />
        <span>{t('topbar.signin')}</span>
      </button>
    );
  }

  const isPro = user.membership === 'member';
  const initials = (user.display_name || user.email).slice(0, 2).toUpperCase();

  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button
          aria-label={t('topbar.user_menu')}
          className="
            group relative flex h-9 w-9 items-center justify-center rounded-full
            ring-1 ring-neutral-200 transition-all
            hover:ring-neutral-300
            focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/40
            dark:ring-neutral-800 dark:hover:ring-neutral-700
          "
        >
          {isPro ? (
            <span className="
              flex h-full w-full items-center justify-center rounded-full
              bg-gradient-to-br from-amber-400 via-amber-500 to-orange-500
              text-[11px] font-bold text-white
            ">
              <Sparkles className="h-3.5 w-3.5" strokeWidth={2.5} />
            </span>
          ) : (
            <span className="
              flex h-full w-full items-center justify-center rounded-full
              bg-neutral-900 text-[11px] font-semibold text-white
              dark:bg-white dark:text-neutral-900
            ">
              {initials}
            </span>
          )}
          {isPro && (
            <span className="
              absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full
              bg-amber-400 ring-2 ring-white dark:ring-neutral-950
            " />
          )}
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={8}
          className="
            z-50 w-64 overflow-hidden rounded-lg
            border border-neutral-200/80 bg-white/95 p-1 shadow-lg shadow-neutral-900/5
            backdrop-blur
            dark:border-neutral-800 dark:bg-neutral-900/95 dark:shadow-black/40
          "
        >
          <div className="px-3 py-2.5">
            <div className="flex items-center justify-between gap-2">
              <div className="min-w-0 flex-1">
                <div className="truncate text-[13px] font-medium text-neutral-900 dark:text-neutral-100">
                  {user.display_name || user.email.split('@')[0]}
                </div>
                <div className="truncate text-[11px] text-neutral-500 dark:text-neutral-400">{user.email}</div>
              </div>
              {isPro && (
                <span className="
                  inline-flex items-center gap-1 rounded-full
                  bg-amber-50 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-amber-700
                  dark:bg-amber-500/15 dark:text-amber-300
                ">
                  <Sparkles className="h-2.5 w-2.5" strokeWidth={2.5} />
                  Pro
                </span>
              )}
            </div>
          </div>
          <DropdownMenu.Separator className="my-1 h-px bg-neutral-100 dark:bg-neutral-800" />
          <DropdownMenu.Item
            onClick={() => router.push('/account')}
            className="
              flex cursor-pointer items-center gap-2.5 rounded-md px-3 py-1.5 text-[13px]
              text-neutral-700 outline-none
              data-[highlighted]:bg-neutral-100
              dark:text-neutral-300 dark:data-[highlighted]:bg-neutral-800
            "
          >
            <UserIcon className="h-[15px] w-[15px] text-neutral-400" strokeWidth={1.75} />
            {t('topbar.account_membership')}
          </DropdownMenu.Item>
          <DropdownMenu.Item
            onClick={() => router.push('/settings')}
            className="
              flex cursor-pointer items-center gap-2.5 rounded-md px-3 py-1.5 text-[13px]
              text-neutral-700 outline-none
              data-[highlighted]:bg-neutral-100
              dark:text-neutral-300 dark:data-[highlighted]:bg-neutral-800
            "
          >
            <Settings className="h-[15px] w-[15px] text-neutral-400" strokeWidth={1.75} />
            {t('topbar.settings')}
          </DropdownMenu.Item>
          <DropdownMenu.Separator className="my-1 h-px bg-neutral-100 dark:bg-neutral-800" />
          <DropdownMenu.Item
            onClick={async () => { await logout(); router.push('/'); }}
            className="
              flex cursor-pointer items-center gap-2.5 rounded-md px-3 py-1.5 text-[13px]
              text-red-600 outline-none
              data-[highlighted]:bg-red-50
              dark:text-red-400 dark:data-[highlighted]:bg-red-500/15
            "
          >
            <LogOut className="h-[15px] w-[15px]" strokeWidth={1.75} />
            {t('account.logout')}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

/* ─── 4. Search Input (顶部可输入搜索, 派发 lixianhuiji:search-query 事件) ─── */
function CommandTrigger() {
  const { locale, t } = useTranslation();
  const [query, setQuery] = React.useState('');
  const [shortcut, setShortcut] = React.useState('Ctrl K');
  React.useEffect(() => {
    if (typeof navigator === 'undefined') return;
    const isMac = /Mac|iPhone|iPad/i.test(navigator.platform);
    setShortcut(isMac ? '⌘ K' : 'Ctrl K');
  }, []);

  // 输入时派发事件, Sidebar 监听并执行搜索
  React.useEffect(() => {
    const t = setTimeout(() => {
      window.dispatchEvent(new CustomEvent('lixianhuiji:search-query', { detail: query }));
    }, 200);
    return () => clearTimeout(t);
  }, [query]);

  return (
    <div
      className="
        group flex h-9 w-full max-w-[420px] items-center gap-2.5 rounded-md
        border border-neutral-200 bg-neutral-50/60 px-3
        transition-all
        focus-within:border-blue-500 focus-within:bg-white focus-within:ring-2 focus-within:ring-blue-500/20
        dark:border-neutral-800 dark:bg-neutral-900/60 dark:focus-within:bg-neutral-900
      "
    >
      <Search className="h-[15px] w-[15px] text-neutral-400" strokeWidth={2} />
      <input
        type="text"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={t('topbar.search_placeholder')}
        className="flex-1 bg-transparent text-[13px] text-neutral-900 placeholder:text-neutral-500 outline-none dark:text-neutral-100"
      />
      {query && (
        <button
          onClick={() => setQuery('')}
          className="rounded p-0.5 text-neutral-400 hover:bg-neutral-100 hover:text-neutral-700 dark:hover:bg-neutral-800"
          aria-label={t('topbar.clear_search')}
        >
          <X className="h-3 w-3" />
        </button>
      )}
      <kbd className="
        hidden rounded border border-neutral-200 bg-white px-1.5 py-0.5
        text-[10px] font-mono text-neutral-500
        dark:border-neutral-700 dark:bg-neutral-800 dark:text-neutral-400
        sm:inline-block
      ">
        {shortcut}
      </kbd>
    </div>
  );
}

/* ─── 5. Status Pill ─── pure indicator, no text, just dot + tooltip */
function StatusPill() {
  const { locale, t } = useTranslation();
  return (
    <div className="
      group relative flex h-9 items-center gap-2 rounded-md px-2.5
      text-[12px] text-neutral-600
      transition-colors hover:bg-neutral-50
      dark:text-neutral-400 dark:hover:bg-neutral-900
    ">
      <span className="relative flex h-2 w-2">
        <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-app-transcript/60 opacity-60" />
        <span className="relative inline-flex h-2 w-2 rounded-full bg-app-transcript" />
      </span>
      <span className="hidden text-[12px] font-medium tracking-tight lg:inline">
        {t('topbar.status_on')}
      </span>
      {/* Tooltip on hover */}
      <div className="
        pointer-events-none absolute right-0 top-full mt-1.5
        hidden whitespace-nowrap rounded-md bg-neutral-900 px-2 py-1 text-[11px] text-white shadow-md
        group-hover:block dark:bg-neutral-800 dark:text-neutral-100
      ">
        {t('topbar.status_tooltip')}
      </div>
    </div>
  );
}

/* ─── 6. Brand Mark ─── wordmark + tiny version chip */
function BrandMark() {
  const pathname = usePathname() ?? '';
  const { locale } = useTranslation();

  // Hide crumb on home for visual calm; otherwise show segmented breadcrumb
  const seg = pathname.split('/').filter(Boolean)[0] ?? '';
  const crumb = CRUMB_LABEL[locale][seg];

  return (
    <div className="flex min-w-0 items-center gap-2">
      {crumb && (
        <>
          <span className="text-neutral-300 dark:text-neutral-700">/</span>
          <span className="truncate text-[13px] text-neutral-600 dark:text-neutral-400">{crumb}</span>
        </>
      )}
    </div>
  );
}

/* ─── Topbar ─── */
export default function Topbar() {
  return (
    <header
      className="
        sticky top-0 z-30 flex h-14 items-center gap-3
        border-b border-neutral-200/80
        bg-white/70 px-4
        backdrop-blur-md backdrop-saturate-150
        supports-[backdrop-filter]:bg-white/60
        dark:border-neutral-800/80 dark:bg-neutral-950/70
        dark:supports-[backdrop-filter]:bg-neutral-950/60
      "
    >
      {/* Left: brand */}
      <div className="flex shrink-0 items-center">
        <BrandMark />
      </div>

      {/* Center: command palette */}
      <div className="flex flex-1 justify-center px-4">
        <CommandTrigger />
      </div>

      {/* Right: actions */}
      <div className="flex shrink-0 items-center gap-1">
        <StatusPill />
        <RecordButton />
        <span className="mx-1 h-5 w-px bg-neutral-200 dark:bg-neutral-800" />
        <LanguageSwitcher />
        <UserMenu />
      </div>
    </header>
  );
}
