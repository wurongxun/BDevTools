// 值对比工具：文本行 diff（LCS）与 JSON 结构 diff。

export type DiffLine = { kind: 'eq' | 'add' | 'del'; text: string }

/** 基于 LCS 的两组行差异，返回带标记的行序列（用于并排/统一展示）。 */
export function calcLineDiff(aLines: string[], bLines: string[]): DiffLine[] {
  const n = aLines.length
  const m = bLines.length
  const dp: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0))
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      if (aLines[i] === bLines[j]) dp[i][j] = dp[i + 1][j + 1] + 1
      else dp[i][j] = Math.max(dp[i + 1][j], dp[i][j + 1])
    }
  }
  const out: DiffLine[] = []
  let i = 0
  let j = 0
  while (i < n && j < m) {
    if (aLines[i] === bLines[j]) {
      out.push({ kind: 'eq', text: aLines[i] })
      i++
      j++
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      out.push({ kind: 'del', text: aLines[i] })
      i++
    } else {
      out.push({ kind: 'add', text: bLines[j] })
      j++
    }
  }
  while (i < n) out.push({ kind: 'del', text: aLines[i++] })
  while (j < m) out.push({ kind: 'add', text: bLines[j++] })
  return out
}

export type JsonChange = {
  path: string
  type: 'added' | 'removed' | 'changed'
  from?: any
  to?: any
}

/** 递归比较两棵 JSON 对象/数组，返回按路径的差异列表。 */
export function calcJsonDiff(a: any, b: any, base = ''): JsonChange[] {
  const out: JsonChange[] = []
  const keyOf = (k: string | number) => (base ? base + '.' + k : String(k))
  if (Array.isArray(a) && Array.isArray(b)) {
    const len = Math.max(a.length, b.length)
    for (let idx = 0; idx < len; idx++) {
      const ka = keyOf(idx)
      if (idx >= a.length) out.push({ path: ka, type: 'added', to: b[idx] })
      else if (idx >= b.length) out.push({ path: ka, type: 'removed', from: a[idx] })
      else if (!shallowEqual(a[idx], b[idx])) {
        if (isContainer(a[idx]) && isContainer(b[idx])) out.push(...calcJsonDiff(a[idx], b[idx], ka))
        else out.push({ path: ka, type: 'changed', from: a[idx], to: b[idx] })
      }
    }
    return out
  }
  if (isContainer(a) && isContainer(b)) {
    const keys = new Set([...Object.keys(a), ...Object.keys(b)])
    for (const k of keys) {
      const kb = keyOf(k)
      if (!(k in a)) out.push({ path: kb, type: 'added', to: b[k] })
      else if (!(k in b)) out.push({ path: kb, type: 'removed', from: a[k] })
      else if (!shallowEqual(a[k], b[k])) {
        if (isContainer(a[k]) && isContainer(b[k])) out.push(...calcJsonDiff(a[k], b[k], kb))
        else out.push({ path: kb, type: 'changed', from: a[k], to: b[k] })
      }
    }
    return out
  }
  // 基础类型或混合
  if (a !== b) out.push({ path: base || '(root)', type: 'changed', from: a, to: b })
  return out
}

function isContainer(v: any): boolean {
  return v !== null && typeof v === 'object'
}
function shallowEqual(a: any, b: any): boolean {
  if (a === b) return true
  return JSON.stringify(a) === JSON.stringify(b)
}
