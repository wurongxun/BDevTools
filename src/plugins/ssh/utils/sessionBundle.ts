/**
 * P3 会话导入导出的单一真源：把「会话 + 分组 + 转发规则 + 偏好」打成一个 JSON 包。
 *
 * 关键坑：转发规则里存的是 `connectionId`，而导入会话会重新分配自增 id。
 * 因此导出时额外写 `connectionName`，导入时按名字重新绑定；找不到同名会话的规则直接丢弃，
 * 绝不让规则挂到一台不相干的机器上。
 */
import type { RemoteConnection, SessionBundle } from '../types'
import { mergeForwardRules, readAllForwardRules } from './forwardRules'

export interface BundleStore {
  allConnections: RemoteConnection[]
  connectionGroups: string[]
  importConnections(json: string): Promise<number>
  addGroup(name: string): boolean
}

export interface BundlePreferences {
  preferencesBundle(): Record<string, unknown>
  // Pinia store 解包后的方法签名带具体字面量类型，这里放宽以免结构赋值时参数逆变报错
  applyPreferencesBundle(bundle: any, merge: boolean): void
}

export function buildSessionBundle(store: BundleStore, prefs: BundlePreferences): SessionBundle {
  const all = store.allConnections
  const connections = all.map(({ id: _id, status: _status, ...rest }) => rest)
  const forwardRules = readAllForwardRules().map(r => ({
    ...r,
    connectionName: all.find(c => c.id === r.connectionId)?.name || ''
  }))
  return {
    kind: 'bdevtools-ssh-sessions',
    version: 2,
    connections: connections as unknown as SessionBundle['connections'],
    groups: [...store.connectionGroups],
    forwardRules,
    ...prefs.preferencesBundle()
  } as SessionBundle
}

export interface BundleImportResult {
  count: number
  rules: number
  error: string
}

export async function importSessionBundle(
  text: string,
  store: BundleStore,
  prefs: BundlePreferences
): Promise<BundleImportResult> {
  let parsed: any
  try { parsed = JSON.parse(text) } catch { return { count: 0, rules: 0, error: '不是合法的 JSON' } }
  const count = await store.importConnections(text)
  const incoming: any[] = Array.isArray(parsed?.forwardRules) ? parsed.forwardRules : []
  const mapped = incoming
    .map(r => {
      const target = store.allConnections.find(c => c.name === r?.connectionName)
      return target ? { ...r, connectionId: target.id } : null
    })
    .filter(Boolean)
  const rules = mergeForwardRules(mapped as any)
  prefs.applyPreferencesBundle(parsed || {}, true)
  if (Array.isArray(parsed?.groups)) {
    for (const g of parsed.groups) if (typeof g === 'string') store.addGroup(g)
  }
  return { count, rules, error: '' }
}
