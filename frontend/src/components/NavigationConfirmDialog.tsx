'use client';

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { AlertTriangle, Loader2 } from 'lucide-react';
import type { PendingNav } from '@/hooks/useNavigationGuard';

export type NavigationConfirmDialogProps = {
  pendingNav: PendingNav | null;
  title: string;
  description: string;
  confirmText: string;
  cancelText: string;
  onConfirm: () => void;
  onCancel: () => void;
};

export function NavigationConfirmDialog({
  pendingNav,
  title,
  description,
  confirmText,
  cancelText,
  onConfirm,
  onCancel,
}: NavigationConfirmDialogProps) {
  const open = pendingNav !== null;

  // 用户描述: 不同导航来源显示不同提示
  const getDescription = () => {
    if (pendingNav?.type === 'beforeunload') {
      return description + ' (关闭/刷新浏览器)';
    }
    if (pendingNav?.type === 'popstate') {
      return description + ' (浏览器后退)';
    }
    return description;
  };

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!o) onCancel(); }}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <div className="flex items-start gap-3">
            <div className="mt-0.5 flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-full bg-amber-100">
              <AlertTriangle className="h-5 w-5 text-amber-600" />
            </div>
            <div className="flex-1">
              <DialogTitle className="text-[15px] font-semibold text-gray-900">
                {title}
              </DialogTitle>
              <DialogDescription className="mt-1.5 text-[13px] leading-[1.6] text-gray-600">
                {getDescription()}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>
        <DialogFooter className="mt-2 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <Button
            variant="outline"
            onClick={onCancel}
            className="w-full sm:w-auto"
            data-testid="navigation-guard-cancel"
          >
            <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
            {cancelText}
          </Button>
          <Button
            variant="destructive"
            onClick={onConfirm}
            className="w-full sm:w-auto"
            data-testid="navigation-guard-confirm"
          >
            {confirmText}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
