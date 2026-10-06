<script setup lang="ts">
/**
 * 第三方插件主面板壳：沙箱 iframe。
 *
 * 安全边界（这里是宿主侧唯一的 IPC 出口）：
 * - iframe 只加 `sandbox="allow-scripts"`，**不加 `allow-same-origin`** → 不透明源，
 *   插件页既访问不到 `parent`，也拿不到任何 Tauri 对象
 * - 插件页只能 `parent.postMessage`；宿主对每条消息做**来源校验 + 协议版本校验**
 * - 唯一的转出通道是 `plugin_call`：后端把它转发进 Extism WASM 沙箱，宿主能力
 *   （日志 / KV / 文件 / HTTP）由 host function 按清单声明的权限逐次校验后放行
 *
 * 桥协议（`v` 恒为 PLUGIN_BRIDGE_VERSION）：
 *   插件 → 宿主：`{ v, kind: 'ready' }`、`{ v, kind: 'call', reqId, func, input }`
 *   宿主 → 插件：`{ v, reqId, ok, data }` / `{ v, reqId, ok:false, error }`、
 *                `{ v, kind: 'theme', dark, wasm }`
 *
 * 宿主保留方法（`func` 以 `__host_` 开头，由本面板处理，**不进 WASM**）：
 *   `__host_dialog_open` / `__host_dialog_save` —— 文件对话框（见 `runHostFunc`）
 *
 * `plugin_call` 的返回码语义（插件页据此区分「后端没实现」与「执行出错」）：
 *   200 成功 · 501 插件没有 WASM 后端或运行时未启用 · 500 执行期错误
 */
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { callPlugin, fetchPluginFsRoots, fetchPluginSettings } from '@/core/plugin/api'
import { PLUGIN_SETTINGS_CHANGED_EVENT, pluginRuntimeInfo } from '@/core/plugin/registry'
import { subscribe, type Unsubscribe } from '@/core/runtime'
import { open as openDialog, save as saveDialog } from '@/core/runtime'
import { pluginAssetUrl } from '@/core/plugin/url'
import { PLUGIN_BRIDGE_VERSION } from '@/core/plugin/types'
import { isDark } from '@/composables/useDark'

const props = defineProps<{
  /** 插件 ID（同时是 plugin:// 路径首段） */
  pluginId: string
  /** 包内入口页面相对路径 */
  entry: string
}>()

/** 握手等待时长（超时只提示，不阻断渲染） */
const READY_TIMEOUT_MS = 3000

const iframeRef = ref<HTMLIFrameElement | null>(null)
const ready = ref(false)
const hint = ref('')
const src = pluginAssetUrl(props.pluginId, props.entry)

let timer: ReturnType<typeof setTimeout> | undefined
/** 设置变更订阅的取消函数（面板卸载即失效，订阅状态不需要持久化） */
let settingsUnsub: Unsubscribe | null = null

/** 设置变更广播的载荷 */
interface SettingsChanged {
  id: string
  values: Record<string, unknown>
}

interface BridgeMessage {
  v?: number
  kind?: string
  reqId?: number
  func?: string
  input?: unknown
}

/** 向插件页发送一条宿主消息（仅使用 iframe 自己的 contentWindow） */
function postToPlugin(payload: Record<string, unknown>) {
  iframeRef.value?.contentWindow?.postMessage(payload, '*')
}

/** 宿主保留方法：`func` 以 `__host_` 开头的一律不进 WASM，由本面板直接处理 */
const HOST_FUNCS = new Set(['__host_dialog_open', '__host_dialog_save'])

/**
 * 路径归一化：统一分隔符、折叠 `.` 与 `..`、去尾斜杠、忽略大小写。
 *
 * **必须折叠 `..`**：只做前缀比较的话 `{root}/a/../b` 会「看起来在范围内」；有了排除区
 * （其它插件的数据目录）之后，这个洞会直接变成绕过隔离的入口。比较两侧都过这个函数，
 * 所以折叠不会误伤正常路径（UNC 的前导空段在两侧同样被丢掉）。
 */
function normPath(p: string): string {
  const out: string[] = []
  for (const seg of p.replace(/\\/g, '/').split('/')) {
    if (seg === '' || seg === '.') continue
    if (seg === '..') {
      out.pop()
      continue
    }
    out.push(seg)
  }
  return out.join('/').toLowerCase()
}

/** 路径是否落在任一作用域内（作用域目录本身也算在内） */
function withinRoots(p: string, roots: string[]): boolean {
  if (roots.length === 0) return false
  const n = normPath(p)
  return roots.some((r) => {
    const root = normPath(r)
    return n === root || n.startsWith(root + '/')
  })
}

/**
 * 执行宿主保留方法（当前只有文件对话框）。
 *
 * 刻意放在**前端层**发起，而不是做成 WASM 的 host function：模态对话框若在 host function
 * 里阻塞，会和 `plugin_call` 的异步任务、webview 主线程互相卡住；前端层天然异步，不存在死锁。
 *
 * 作用域约束：返回路径必须落在清单声明的 `fs` 作用域内。对话框返回的是字符串、本身不授予
 * 访问权，但允许它指向任意位置会误导用户（选了却读不了），所以这里直接拒绝而不是透传。
 * 判定用的根目录由宿主给出（与 `host_fs_open` 同一份），前端只做前缀比较，不重解析清单。
 *
 * 声明了 `fs:read:*` / `fs:write:*` 的插件走**通配模式**：任意路径都能选，只剩「排除区」
 * （`deny`：其它插件的数据目录 / 内置插件数据 / 插件安装根）这一道闸 —— 排除区与后端
 * `host_fs_open` 用的是同一份列表，否则用户会选到「前端放行、后端拒绝」的路径。
 */
async function runHostFunc(
  func: string,
  input: unknown,
): Promise<{ ok: true; data: unknown } | { ok: false; error: string }> {
  const kind = func === '__host_dialog_save' ? 'save' : 'open'
  const perm = kind === 'save' ? 'fs:write' : 'fs:read'
  const roots = await fetchPluginFsRoots(props.pluginId)
  // 通配作用域（`fs:read:*` / `fs:write:*`）：任意路径，只剩排除区这一道闸
  const any = kind === 'save' ? roots.writeAny : roots.readAny
  const allowed = kind === 'save' ? roots.write : roots.read
  const deny = Array.isArray(roots.deny) ? roots.deny : []
  if (!any && allowed.length === 0) {
    return { ok: false, error: `插件未声明任何 ${perm} 作用域，不能使用宿主文件对话框` }
  }
  /** 该路径是否可被选中：排除区优先 → 通配放行 → 否则逐个前缀比较 */
  const pickable = (p: string): boolean => {
    if (withinRoots(p, deny)) return false
    return any || withinRoots(p, allowed)
  }
  const opt = (input ?? {}) as Record<string, unknown>
  const filters = Array.isArray(opt.filters)
    ? (opt.filters as { name: string; extensions: string[] }[])
    : undefined
  // 插件给的起始目录也要过同一道闸：否则它能先把对话框指到排除区里
  const wanted = typeof opt.defaultPath === 'string' ? opt.defaultPath : undefined
  const defaultPath = wanted && pickable(wanted) ? wanted : allowed[0]

  const picked =
    kind === 'save'
      ? await saveDialog({ filters, defaultPath })
      : await openDialog({ multiple: opt.multiple === true, filters, defaultPath })

  // 用户取消 → null（插件必须处理，这是最常见的空指针来源）
  if (picked === null || picked === undefined) return { ok: true, data: null }
  const paths = Array.isArray(picked) ? picked : [picked]
  const rejected = paths.filter((x) => !pickable(x))
  if (rejected.length > 0) {
    const why = paths.some((x) => withinRoots(x, deny))
      ? '其它插件的数据目录与插件安装目录不可访问'
      : `不在插件声明的 ${perm} 作用域内`
    return { ok: false, error: `所选路径${why}：${rejected.join('、')}` }
  }
  return { ok: true, data: Array.isArray(picked) ? paths : paths[0] }
}

/** 同步宿主主题（与 WASM 运行时可用性）给插件页，插件页自行决定是否消费 */
function pushTheme() {
  postToPlugin({
    v: PLUGIN_BRIDGE_VERSION,
    kind: 'theme',
    dark: isDark.value,
    // 插件页可据此禁用「依赖后端」的按钮，而不是等到调用失败才发现
    wasm: pluginRuntimeInfo.value?.available === true,
  })
}

/** 首次下发插件设置（清单 `contributes.configuration`；没声明就是空对象，不下发） */
async function pushSettings() {
  try {
    const values = await fetchPluginSettings(props.pluginId)
    if (Object.keys(values).length === 0) return
    postToPlugin({ v: PLUGIN_BRIDGE_VERSION, kind: 'settings', data: values })
  } catch {
    // 拿不到设置不该影响面板本身：插件页会回落到自己的默认值
  }
}

function markReady() {
  ready.value = true
  hint.value = ''
  if (timer) {
    clearTimeout(timer)
    timer = undefined
  }
  pushTheme()
  pushSettings()
}

async function onMessage(e: MessageEvent) {
  const frame = iframeRef.value
  // 来源校验：不是本面板 iframe 发来的一律丢弃（其它窗口/其它插件/任意脚本都进不来）
  if (!frame || e.source !== frame.contentWindow) return
  const data = e.data as BridgeMessage | null
  if (!data || typeof data !== 'object') return
  // 协议版本校验：版本不匹配说明对端不是本协议，直接忽略
  if (data.v !== PLUGIN_BRIDGE_VERSION) return

  if (data.kind === 'ready') {
    markReady()
    return
  }
  if (data.kind !== 'call' || typeof data.reqId !== 'number') return

  const func = typeof data.func === 'string' ? data.func : ''
  // 宿主保留方法：文件对话框等宿主侧能力，不进 WASM
  if (HOST_FUNCS.has(func)) {
    const payload = await runHostFunc(func, data.input)
    postToPlugin({ v: PLUGIN_BRIDGE_VERSION, reqId: data.reqId, ...payload })
    return
  }
  // 其余一律走 WASM 桥（插件的真实后端打包在自己的 plugin.wasm 里，经 plugin_call 调用）
  const reply = await callPlugin(props.pluginId, func, data.input)
  const payload = reply.code === 200
    ? { ok: true, data: reply.data }
    : { ok: false, error: reply.msg || '插件调用失败' }
  postToPlugin({ v: PLUGIN_BRIDGE_VERSION, reqId: data.reqId, ...payload })
}

watch(isDark, () => {
  if (ready.value) pushTheme()
})

onMounted(async () => {
  window.addEventListener('message', onMessage)
  // 设置页改完设置 → 广播给已打开的面板，插件页不必重新握手就能拿到新值
  settingsUnsub = await subscribe<SettingsChanged>(PLUGIN_SETTINGS_CHANGED_EVENT, (p) => {
    if (!p || p.id !== props.pluginId) return
    if (ready.value) {
      postToPlugin({ v: PLUGIN_BRIDGE_VERSION, kind: 'settings', data: p.values })
    }
  })
  timer = setTimeout(() => {
    if (!ready.value) {
      hint.value = `插件页面未在 ${READY_TIMEOUT_MS / 1000} 秒内完成 ready 握手：页面需在加载后发送 { v: ${PLUGIN_BRIDGE_VERSION}, kind: 'ready' }。`
    }
  }, READY_TIMEOUT_MS)
})

onBeforeUnmount(() => {
  window.removeEventListener('message', onMessage)
  settingsUnsub?.()
  if (timer) clearTimeout(timer)
})
</script>

<template>
  <div class="sandbox-panel">
    <div v-if="hint" class="sandbox-hint" role="status">{{ hint }}</div>
    <iframe
      ref="iframeRef"
      class="sandbox-frame"
      :src="src"
      :title="`第三方插件页面 ${pluginId}`"
      sandbox="allow-scripts"
      referrerpolicy="no-referrer"
    />
  </div>
</template>

<style scoped>
.sandbox-panel {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.sandbox-hint {
  flex: none;
  padding: 8px 12px;
  font-size: var(--font-size-base);
  line-height: 1.5;
  border-bottom: 1px solid var(--el-border-color-lighter);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.sandbox-frame {
  flex: 1;
  width: 100%;
  min-height: 0;
  border: 0;
  display: block;
  background: transparent;
}
</style>
