/**
 * P2 遗留：转发规则的持久化真源（原来只有 TunnelPanel.vue 内联读写 localStorage）。
 * 抽出来供会话导入导出复用，避免两边各写一份 JSON 解析导致字段漂移。
 *
 * 规则只在 localStorage，不会随应用启动自动开监听端口（安全口径见路线图 §5.1）。
 */
import type { ForwardRule } from '../types'

export const FORWARD_RULES_KEY = 'ssh-tunnel-rules'

export function readAllForwardRules(): ForwardRule[] {
  try {
    const raw = localStorage.getItem(FORWARD_RULES_KEY)
    const all = raw ? JSON.parse(raw) : []
    return Array.isArray(all) ? all.filter(r => r && typeof r.id === 'string') : []
  } catch {
    return []
  }
}

/** 只替换指定连接的规则，其余连接的规则原样保留 */
export function writeMineForConnection(connectionId: number, mine: ForwardRule[]): void {
  try {
    const others = readAllForwardRules().filter(r => r.connectionId !== connectionId)
    localStorage.setItem(FORWARD_RULES_KEY, JSON.stringify([...others, ...mine]))
  } catch { /* localStorage 不可用时仅内存生效 */ }
}

/** 导入：按 id 去重追加（id 冲突时重新编号，不覆盖已有规则） */
export function mergeForwardRules(incoming: ForwardRule[]): number {
  const all = readAllForwardRules()
  const ids = new Set(all.map(r => r.id))
  let added = 0
  for (const rule of incoming) {
    if (!rule || typeof rule.id !== 'string' || typeof rule.connectionId !== 'number') continue
    const id = ids.has(rule.id) ? `${rule.id}-${Date.now().toString(36)}-${added}` : rule.id
    ids.add(id)
    all.push({ ...rule, id })
    added++
  }
  try {
    localStorage.setItem(FORWARD_RULES_KEY, JSON.stringify(all))
  } catch { /* ignore */ }
  return added
}
