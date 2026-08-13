'use client';

import React from 'react';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import Topbar from '@/components/Topbar';
import { useTranslation } from '@/i18n';

interface MainContentProps {
  children: React.ReactNode;
}

const MainContent: React.FC<MainContentProps> = ({ children }) => {
  const { isCollapsed } = useSidebar();
  const { locale } = useTranslation();
  // 登录 / 注册页 不显示顶栏 + 不挤压 sidebar
  const isAuthRoute = typeof window !== 'undefined' && (
    window.location.pathname.startsWith('/login') ||
    window.location.pathname.startsWith('/register')
  );

  return (
    <main className={`flex-1 h-screen overflow-y-auto transition-all duration-300 ${isAuthRoute ? '' : (isCollapsed ? 'ml-[68px]' : 'ml-[252px]')}`}>
      {!isAuthRoute && <Topbar />}
      <div className={isAuthRoute ? '' : 'px-8 py-6'}>{children}</div>
    </main>
  );
};

export default MainContent;
