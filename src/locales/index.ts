/**
 * locales/i18n.ts
 *
 * 框架文档: https://vue-i18n.intlify.dev/
 */
import { createI18n } from 'vue-i18n'

import EN from '@/locales/en'
import ZH_CN from '@/locales/zh-CN'
import ZH_TW from '@/locales/zh-TW'

/** 应用支持的语言标识（与 messages key 一致） */
export type AppLocale = 'zh_CN' | 'zh_TW' | 'zh_HK' | 'en'

/** 探测系统语言作为初始 locale（用户未设置过语言时） */
function detectInitialLocale(): AppLocale {
  const nav = navigator.language
  if (nav.startsWith('zh')) {
    return /TW|HK|MO/i.test(nav) ? 'zh_TW' : 'zh_CN'
  }
  if (nav.startsWith('en')) {
    return 'en'
  }
  return 'zh_CN'
}

const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  // 初始值取系统语言；main.ts 挂载前会用 settings.json 里保存的语言覆盖
  locale: detectInitialLocale(),
  fallbackLocale: 'zh_CN',
  messages: {
    en: EN,
    zh_CN: ZH_CN,
    zh_TW: ZH_TW,
    zh_HK: ZH_TW
  }
})

export default i18n
