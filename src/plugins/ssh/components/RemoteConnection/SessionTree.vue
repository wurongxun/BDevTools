<template>
  <div class="session-tree">
    <div
      v-for="item in flat"
      :key="item.key"
      class="tree-row"
      :class="{
        'tree-group': item.type === 'group',
        'tree-conn': item.type === 'conn',
        'tree-active': item.type === 'conn' && item.conn.id === activeConnId
      }"
      :style="{ paddingLeft: `${8 + item.depth * 14}px` }"
      :title="item.type === 'group' ? '点击展开 / 折叠；点击色点设置分组颜色' : '点击打开终端；点击色条设置会话颜色'"
      @click="onRowClick(item)"
    >
      <template v-if="item.type === 'group'">
        <span class="tree-arrow">{{ collapsed.includes(item.path) ? '▸' : '▾' }}</span>
        <span
          class="tree-dot"
          :style="{ background: prefs.groupColor(item.path) || 'transparent' }"
          @click.stop="emit('setGroupColor', item.path)"
        />
        <span class="tree-label">{{ item.name }}</span>
        <span class="tree-count">{{ item.count }}</span>
      </template>
      <template v-else>
        <span
          class="tree-bar"
          :style="{ background: item.conn.color || 'transparent' }"
          @click.stop="emit('setConnColor', item.conn)"
        />
        <span class="tree-label">{{ item.conn.name }}</span>
        <span class="tree-addr">{{ item.conn.username }}@{{ item.conn.address }}</span>
      </template>
    </div>
    <div v-if="!flat.some(i => i.type === 'conn')" class="tree-empty">没有匹配的会话</div>
  </div>
</template>

<script setup lang="ts">
/**
 * P3 多级分组树。`favorites_name` 仍是一列字符串，用 `/`（兼容 Xshell 的 `\`）表示层级，
 * 因此老库无需迁移即可获得多级分组；列表视图继续按原样显示完整路径。
 */
import { computed, ref } from 'vue'
import { useSshPreferencesStore } from '../../store'
import type { RemoteConnection } from '../../types'

const props = defineProps<{
  connections: RemoteConnection[]
  groups: string[]
  searchQuery: string
  activeConnId: number | null
}>()

const emit = defineEmits<{
  open: [conn: RemoteConnection]
  setConnColor: [conn: RemoteConnection]
  setGroupColor: [path: string]
}>()

const prefs = useSshPreferencesStore()
const collapsed = ref<string[]>([])

interface Node {
  name: string
  path: string
  children: Map<string, Node>
  conns: RemoteConnection[]
}

type FlatItem =
  | { key: string; type: 'group'; path: string; name: string; depth: number; count: number }
  | { key: string; type: 'conn'; conn: RemoteConnection; depth: number }

function splitPath(raw: string): string[] {
  const parts = (raw || '默认分组').split(/[/\\]/).map(s => s.trim()).filter(Boolean)
  return parts.length ? parts : ['默认分组']
}

function ensureNode(root: Node, parts: string[]): Node {
  let node = root
  let path = ''
  for (const part of parts) {
    path = path ? `${path}/${part}` : part
    if (!node.children.has(part)) {
      node.children.set(part, { name: part, path, children: new Map(), conns: [] })
    }
    node = node.children.get(part)!
  }
  return node
}

function countConns(node: Node): number {
  let total = node.conns.length
  for (const child of node.children.values()) total += countConns(child)
  return total
}

const tree = computed<Node>(() => {
  const root: Node = { name: '', path: '', children: new Map(), conns: [] }
  const q = props.searchQuery.trim().toLowerCase()
  const visible = props.connections.filter(c => {
    if (!q) return true
    return `${c.name} ${c.address} ${c.username} ${c.description || ''} ${c.favorites_name || ''}`
      .toLowerCase().includes(q)
  })
  const paths = new Set<string>(props.groups)
  for (const conn of visible) paths.add(conn.favorites_name || '默认分组')
  for (const path of paths) ensureNode(root, splitPath(path))
  for (const conn of visible) {
    ensureNode(root, splitPath(conn.favorites_name || '默认分组')).conns.push(conn)
  }
  return root
})

const flat = computed<FlatItem[]>(() => {
  const out: FlatItem[] = []
  const walk = (node: Node, depth: number) => {
    for (const child of node.children.values()) {
      out.push({
        key: `g:${child.path}`, type: 'group', path: child.path,
        name: child.name, depth, count: countConns(child)
      })
      if (!collapsed.value.includes(child.path)) {
        for (const conn of child.conns) {
          out.push({ key: `c:${conn.id}`, type: 'conn', conn, depth: depth + 1 })
        }
        walk(child, depth + 1)
      }
    }
  }
  walk(tree.value, 0)
  return out
})

function onRowClick(item: FlatItem) {
  if (item.type === 'conn') {
    emit('open', item.conn)
    return
  }
  const idx = collapsed.value.indexOf(item.path)
  if (idx >= 0) collapsed.value.splice(idx, 1)
  else collapsed.value.push(item.path)
}
</script>

<style scoped lang="scss">
.session-tree { font-size: 13px; user-select: none; }
.tree-row {
  display: flex; align-items: center; gap: 6px; min-height: 26px; padding-right: 8px;
  border-radius: 3px; cursor: pointer;
  &:hover { background: rgba(128, 128, 128, 0.12); }
}
.tree-active { background: rgba(64, 158, 255, 0.16); }
.tree-group { font-weight: 500; }
.tree-arrow { width: 12px; color: #888; }
.tree-dot {
  width: 10px; height: 10px; border-radius: 50%; border: 1px solid rgba(128,128,128,0.5);
  flex-shrink: 0; cursor: pointer;
}
.tree-bar { width: 4px; height: 15px; border-radius: 2px; flex-shrink: 0; cursor: pointer; }
.tree-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.tree-count, .tree-addr { margin-left: auto; font-size: 11px; color: #888; white-space: nowrap; }
.tree-addr { max-width: 46%; overflow: hidden; text-overflow: ellipsis; }
.tree-empty { color: #999; text-align: center; padding: 18px; font-size: 12px; }
</style>
