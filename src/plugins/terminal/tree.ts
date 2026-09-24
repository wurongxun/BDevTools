/**
 * 分屏树操作（纯函数）
 *
 * 树节点只有两种：`leaf`（一个窗格）与 `split`（二分：`a` + `b` + 比例）。
 * 之所以用二叉树而不是 N 叉 + sizes 数组：分割条数 = 叶子数 - 1，
 * 但二叉树让「删除一个窗格后把兄弟上提」这件事变成 3 行代码，
 * 不需要重新分配 N-1 个比例。代价是深树时分割条会变多，实际使用中深度 ≤ 4 足够。
 */
import type { PaneNode, TabState } from './types'

/** 遍历所有叶子窗格 id（先序遍历，与视觉顺序一致） */
export function eachLeaf(node: PaneNode, fn: (paneId: string) => void) {
  if (node.kind === 'leaf') {
    fn(node.paneId)
    return
  }
  eachLeaf(node.a, fn)
  eachLeaf(node.b, fn)
}

/** 收集所有叶子窗格 id */
export function leafIds(node: PaneNode): string[] {
  const out: string[] = []
  eachLeaf(node, (id) => out.push(id))
  return out
}

/** 树里的第一个叶子（用于「激活窗格被关闭后选谁」） */
export function firstLeaf(node: PaneNode): string {
  return node.kind === 'leaf' ? node.paneId : firstLeaf(node.a)
}

/** 树里包含多少个叶子 */
export function leafCount(node: PaneNode): number {
  return node.kind === 'leaf' ? 1 : leafCount(node.a) + leafCount(node.b)
}

/**
 * 把 `paneId` 这个叶子替换成 `next` 子树。
 * 找不到时原样返回（调用方自己判断是否命中）。
 */
export function replaceLeaf(node: PaneNode, paneId: string, next: PaneNode): PaneNode {
  if (node.kind === 'leaf') {
    return node.paneId === paneId ? next : node
  }
  return {
    ...node,
    a: replaceLeaf(node.a, paneId, next),
    b: replaceLeaf(node.b, paneId, next),
  }
}

/**
 * 删除一个叶子；若其父节点只剩一个子节点，则把兄弟节点上提。
 * 整棵树只剩这一个叶子时返回 `null`（调用方据此关闭整个标签）。
 */
export function removeLeaf(node: PaneNode, paneId: string): PaneNode | null {
  if (node.kind === 'leaf') {
    return node.paneId === paneId ? null : node
  }
  const a = removeLeaf(node.a, paneId)
  const b = removeLeaf(node.b, paneId)
  if (a === null && b === null) return null
  // 一侧被删空：把另一侧上提（不留下单子节点的 split）
  if (a === null) return b
  if (b === null) return a
  return { ...node, a, b }
}

/** 调整某个 split 的比例（按路径定位，避免同结构子树误改） */
export function setRatio(node: PaneNode, path: number[], ratio: number): PaneNode {
  if (node.kind === 'leaf') return node
  if (path.length === 0) {
    return { ...node, ratio: Math.min(0.85, Math.max(0.15, ratio)) }
  }
  const [head, ...rest] = path
  return head === 0
    ? { ...node, a: setRatio(node.a, rest, ratio) }
    : { ...node, b: setRatio(node.b, rest, ratio) }
}

/** 找到包含某叶子的 split 路径（0 = a 侧，1 = b 侧）；找不到返回 null */
export function pathToLeaf(node: PaneNode, paneId: string): number[] | null {
  if (node.kind === 'leaf') return node.paneId === paneId ? [] : null
  const inA = pathToLeaf(node.a, paneId)
  if (inA) return [0, ...inA]
  const inB = pathToLeaf(node.b, paneId)
  if (inB) return [1, ...inB]
  return null
}

/** 该叶子是否还在树里 */
export function hasLeaf(node: PaneNode, paneId: string): boolean {
  return pathToLeaf(node, paneId) !== null
}

/** 新建标签的初始状态（单窗格、无分屏） */
export function singlePaneTab(tabId: string, paneId: string, title: string): TabState {
  return {
    id: tabId,
    title,
    root: { kind: 'leaf', paneId },
    activePaneId: paneId,
  }
}
