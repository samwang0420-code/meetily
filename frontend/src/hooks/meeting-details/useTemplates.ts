import { useState, useEffect, useCallback } from 'react';
import { invoke as invokeTauri } from '@tauri-apps/api/core';
import { safeToast } from '@/lib/safeToast';
import Analytics from '@/lib/analytics';
import { useTranslation } from '@/i18n';
import { useAuth } from '@/contexts/AuthContext';

export interface AvailableTemplate {
  id: string;
  name: string;
  description: string;
  required_tier: 'free' | 'member';
}

export function useTemplates() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const [availableTemplates, setAvailableTemplates] = useState<AvailableTemplate[]>([]);
  const [selectedTemplate, setSelectedTemplate] = useState<string>('standard_meeting');

  const userTier: 'free' | 'member' = user?.membership ?? 'free';

  useEffect(() => {
    let cancelled = false;
    const fetchTemplates = async () => {
      try {
        // v0.7.0+: 传 user_tier, 后端过滤掉 member 模板
        const templates = await invokeTauri('api_list_templates', {
          userTier,
        }) as AvailableTemplate[];
        if (cancelled) return;
        console.log('[useTemplates] got', templates.length, 'templates for tier', userTier);
        setAvailableTemplates(templates);
      } catch (error) {
        console.error('Failed to fetch templates:', error);
      }
    };
    fetchTemplates();
    return () => { cancelled = true; };
  }, [userTier]);

  // v0.7.0+: 兼容旧调用 (id, name) 也能工作 — 从 availableTemplates 找 required_tier
  const handleTemplateSelection = useCallback((templateId: string, templateName?: string) => {
    const tmpl = availableTemplates.find((t) => t.id === templateId);
    const required = tmpl?.required_tier ?? 'free';
    if (required === 'member' && userTier !== 'member') {
      safeToast.error(t('summary.template_pro_required', { template: tmpl?.name ?? templateName ?? templateId }));
      return;
    }
    setSelectedTemplate(templateId);
    safeToast.success(t('summary.template_selected'), {
      description: t('summary.using_template', { template: tmpl?.name ?? templateName ?? templateId }),
    });
    Analytics.trackFeatureUsed('template_selected');
  }, [t, userTier, availableTemplates]);

  return {
    availableTemplates,
    selectedTemplate,
    handleTemplateSelection,
    userTier,
  };
}
