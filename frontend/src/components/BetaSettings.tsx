"use client"

import { CheckCircle2 } from "lucide-react"
import { useTranslation } from "@/i18n"


export function BetaSettings() {
  const { t } = useTranslation();

  // v0.7.x: importAndRetranscribe 已毕业到正式功能, Beta 开关全部下架.
  // 保留页面以便后续添加新 beta feature 时复用布局.
  return (
    <div className="space-y-6">
      <div className="p-6 bg-white rounded-lg border border-gray-200 shadow-sm">
        <div className="flex items-start gap-3">
          <CheckCircle2 className="h-5 w-5 text-green-600 flex-shrink-0 mt-0.5" />
          <div>
            <h3 className="text-lg font-semibold text-gray-900">所有 Beta 功能已转为正式功能</h3>
            <p className="mt-2 text-sm text-gray-600">
              「导入音频 & 重新转录」功能已毕业, 无需再手动开启. 直接使用 Sidebar 的上传按钮或拖拽音频文件即可.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
