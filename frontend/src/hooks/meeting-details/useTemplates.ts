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

export function useTemplates(initialTemplateId?: string | null) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const [availableTemplates, setAvailableTemplates] = useState<AvailableTemplate[]>([]);
  // §123: 优先用 initialTemplateId (从 meeting.template_id 传), 否则 fallback standard_meeting
  const [selectedTemplate, setSelectedTemplate] = useState<string>(
    initialTemplateId && initialTemplateId.trim() !== '' ? initialTemplateId : 'standard_meeting'
  );

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

    // §123: 选了法律/医学模板 → 检查热词是否已配. 没配的话 toast 提醒 (不阻塞选模板)
    if (templateId === 'legal_consultation' || templateId === 'medical_consultation') {
      const requiredPack = templateId === 'legal_consultation' ? 'legal' : 'medical';
      const session = (typeof window !== 'undefined') ? window.localStorage.getItem('lixianhuiji.session') : null;
      if (session) {
        invokeTauri<{ builtin: string; custom: string; enabled: boolean }>('hotwords_get', { session })
          .then((cfg) => {
            const okPacks = templateId === 'legal_consultation'
              ? ['legal', 'sogou_legal', 'legacy_legal']
              : ['medical', 'sogou_medical', 'legacy_medical'];
            if (cfg.builtin === 'none' || !okPacks.includes(cfg.builtin)) {
              safeToast.warning(
                t('summary.template_hotwords_missing', { template: tmpl?.name ?? templateName ?? templateId }),
                {
                  description: t('summary.template_hotwords_missing_desc', { pack: requiredPack }),
                  duration: 8000,
                }
              );
            }
          })
          .catch((e) => {
            console.warn('[useTemplates] hotwords_get failed, skip reminder:', e);
          });
      }
    }
  }, [t, userTier, availableTemplates]);

  return {
    availableTemplates,
    selectedTemplate,
    handleTemplateSelection,
    userTier,
  };
}
