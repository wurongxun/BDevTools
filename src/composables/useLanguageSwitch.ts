/**
 * useLanguageSwitch —— 全局语言切换的统一入口。
 *
 * 菜单栏、状态栏、设置页三处共用同一套逻辑，避免各写一份 store 句柄与广播：
 *   1) userStore.setLanguage 切换 vue-i18n 的 locale
 *   2) 持久化到 settings.json 的 `language`
 *   3) publish('language-changed') 广播给其他窗口（main.ts 订阅后同步 setLanguage）
 *
 * 与 settings/AppGeneralPane.vue 中的 languageChange 行为保持一致。
 */
import { computed } from 'vue'
import useStore from '@/store'
import { load, publish, type Store } from '@/core/runtime'
import type { AppLocale } from '@/locales'

export interface LanguageOption {
  value: AppLocale
  /** 本地化显示名对应的 i18n key（settingsPlugin.languages.*） */
  labelKey: string
}

/** 支持的语言列表，与 settings/AppGeneralPane 中的 LANGUAGE_OPTIONS 保持一致 */
export const LANGUAGE_OPTIONS: LanguageOption[] = [
  { value: 'zh_CN', labelKey: 'settingsPlugin.languages.zh_CN' },
  { value: 'zh_TW', labelKey: 'settingsPlugin.languages.zh_TW' },
  { value: 'en', labelKey: 'settingsPlugin.languages.en' }
]

/** 模块级共享的 settings.json 句柄，三处共用同一份，避免重复 load */
let settingsStore: Store | undefined

export function useLanguageSwitch() {
  const { appConfig: userStore } = useStore()

  /** 当前语言（appConfigInfo store 中的 language，与 i18n.locale 同步） */
  const currentLanguage = computed(() => userStore.language as AppLocale)

  async function ensureStore(): Promise<Store | undefined> {
    if (!settingsStore) {
      settingsStore = await load('settings.json', { autoSave: false, defaults: {} })
    }
    return settingsStore
  }

  /**
   * 切换语言：立即生效（i18n） + 落盘 + 广播。
   * 与已选语言相同则跳过，省掉一次无意义的写盘与广播。
   */
  async function switchLanguage(lang: AppLocale) {
    if (lang === currentLanguage.value) return
    // 1) 切 i18n（store 的 setLanguage 内部同步 i18n.global.locale）
    userStore.setLanguage(lang)
    // 2) 持久化
    const s = await ensureStore()
    if (s) {
      await s.set('language', { value: lang })
      await s.save()
    }
    // 3) 广播给其他窗口（main.ts 订阅后调用 setLanguage 同步）
    await publish('language-changed', { language: lang })
  }

  return { currentLanguage, LANGUAGE_OPTIONS, switchLanguage }
}
