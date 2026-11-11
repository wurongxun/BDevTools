/**
 * 插件激活耗时统计（懒激活的观测面）。
 *
 * ## 为什么需要它
 *
 * 懒激活把「启动成本」换成了「首次打开成本」：启动快了，但用户不知道慢是谁造成的。
 * 没有这份记录，插件性能问题只能靠猜（VS Code 的「扩展运行概况」就是为这件事存在的）。
 *
 * 口径：**一次记录 = 一个面板 chunk 的首次加载**。异步组件只在真正挂载时才去拉 chunk，
 * 且 KeepAlive 会缓存实例，所以这里的数字≈「用户第一次用到这个插件等了多久」。
 * 重复挂载不会再记一条（Vue 不会重新 resolve 已加载的异步组件）。
 *
 * ## 刻意不做
 *
 * - **不做埋点聚合 / 不落盘**：一次会话内的观察数据，落盘就要处理版本与清理，
 *   收益却只是「上次启动很慢」，不如当场看。
 * - **不统计 `onActivate` 回调**：那是插件自己的初始化，插件没义务上报，
 *   宿主也不该替它计时（口径混在一起就分不清是 chunk 慢还是插件自己慢）。
 */
import { ref } from 'vue'

/** 一次激活（一块面板的首次加载） */
export interface ActivationRecord {
  pluginId: string
  /** 面板局部名：`sidebar` / `main` / `dock` / `tabBar`（与贡献条目的局部 ID 对齐） */
  part: string
  /** 加载耗时（毫秒） */
  ms: number
  /** 发生时刻（`Date.now()`） */
  at: number
}

/** 一个插件的激活汇总 */
export interface PluginActivationSummary {
  pluginId: string
  /** 各面板耗时合计（多数插件只有一两块面板） */
  totalMs: number
  /** 明细（按发生顺序） */
  parts: ActivationRecord[]
}

const records = ref<ActivationRecord[]>([])

/** 全部激活记录（只读视图，按发生顺序） */
export const activationRecords = records

/** 记一次激活（由 `lazyView` 调用，业务代码不要自己调） */
export function recordActivation(pluginId: string, part: string, ms: number): void {
  // 同一块面板理论上只加载一次；真记重了就保留第一条（那才是用户等的那次）
  if (records.value.some(r => r.pluginId === pluginId && r.part === part)) {
    return
  }
  records.value = [...records.value, { pluginId, part, ms: Math.round(ms * 100) / 100, at: Date.now() }]
}

/**
 * 按插件聚合的耗时排行（耗时多的在前）。
 *
 * 只列出**已激活**的插件 —— 没激活就是没花时间，显示 0 会让人以为它免费。
 */
export function activationSummary(): PluginActivationSummary[] {
  const byPlugin = new Map<string, ActivationRecord[]>()
  for (const r of records.value) {
    const list = byPlugin.get(r.pluginId)
    if (list) list.push(r)
    else byPlugin.set(r.pluginId, [r])
  }
  return [...byPlugin.entries()]
    .map(([pluginId, parts]) => ({
      pluginId,
      totalMs: Math.round(parts.reduce((sum, p) => sum + p.ms, 0) * 100) / 100,
      parts,
    }))
    .sort((a, b) => b.totalMs - a.totalMs)
}

/**
 * 某插件的激活耗时（毫秒）；**未激活返回 `null`**。
 *
 * 返回 `null` 而不是 0：0 会跟「快到测不出来」混淆，而「还没加载过」是完全不同的事实。
 */
export function activationMsOf(pluginId: string): number | null {
  const hit = activationSummary().find(s => s.pluginId === pluginId)
  return hit ? hit.totalMs : null
}

/** 面板局部名的中文标签（错误提示与耗时展示用） */
export const PART_LABELS: Record<string, string> = {
  sidebar: '侧栏面板',
  main: '主面板',
  dock: '停靠面板',
  tabBar: '标签栏',
}
