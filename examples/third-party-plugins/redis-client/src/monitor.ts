// 监控解析：把 INFO 文本拆成可读的分区；提取内存 / 客户端 / 命令速率等关键指标。

import type { InfoSection, MemoryUsage } from './types'

export function parseInfo(text: string): InfoSection[] {
  const sections: InfoSection[] = []
  let current: InfoSection | null = null
  text.split('\n').forEach((line) => {
    const t = line.trim()
    if (!t) return
    if (t.startsWith('#')) {
      current = { name: t.slice(1).trim(), fields: [] }
      sections.push(current)
      return
    }
    const i = t.indexOf(':')
    if (i >= 0 && current) {
      current.fields.push({ k: t.slice(0, i), v: t.slice(i + 1) })
    }
  })
  return sections
}

export function computeMemoryUsage(text: string): MemoryUsage {
  const sections = parseInfo(text)
  const mem = sections.find((s) => s.name === 'Memory')
  let usedBytes = 0
  let maxBytes = 0
  if (mem) {
    const used = mem.fields.find((f) => f.k === 'used_memory')
    const max = mem.fields.find((f) => f.k === 'maxmemory')
    if (used) usedBytes = parseInt(used.v, 10) || 0
    if (max && parseInt(max.v, 10) > 0) maxBytes = parseInt(max.v, 10)
  }
  const ratio = maxBytes > 0 ? usedBytes / maxBytes : 0
  return { usedBytes, maxBytes, ratio }
}

export function infoField(text: string, section: string, key: string): string | null {
  const s = parseInfo(text).find((x) => x.name === section)
  if (!s) return null
  const f = s.fields.find((x) => x.k === key)
  return f ? f.v : null
}
