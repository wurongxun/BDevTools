<template>
  <!-- 插件管理 -->
  <section v-show="tab === 'manage'" class="settings-group">
    <p class="settings-group-desc">
      内置插件随应用编译，不可卸载；第三方插件以包形式安装（plugin.json + 页面资源 + 可选 plugin.wasm），
      页面跑在不透明源的沙箱 iframe 中——只占用主面板，不提供侧栏、不接管标签条；
      后端逻辑跑在 Extism WASM 沙箱中，宿主能力（日志 / KV / 文件 / HTTP）按清单声明逐次校验后放行。
      安装后默认在所有窗口启用（窗口预设里已显式勾选插件清单的会自动追加该插件）。
    </p>
    <p class="settings-group-desc">
      活动栏显示哪些插件，事实源是<b>本窗口所属窗口预设</b>的插件清单（即「窗口设置」里的
      「启用插件」勾选，空清单 = 全部）。卡片右侧的 <b>加入 / 移出</b> 是这份清单的快捷开关：
      改完立即生效，并写回该窗口预设 —— 以后用这个预设打开的新窗口也按新清单显示。
      {{ presetScopeHint }}
    </p>

    <!-- WASM 运行时状态：第三方插件的后端逻辑跑在这里 -->
    <div class="plugin-runtime" :class="{ 'is-off': !runtimeReady }">
      <el-icon class="plugin-runtime-ic"><Cpu /></el-icon>
      <div class="plugin-runtime-body">
        <div class="plugin-runtime-title">
          WASM 运行时：{{ runtimeReady ? runtimeName : '未启用' }}
        </div>
        <div v-if="runtimeReady" class="plugin-runtime-meta">
          单次调用 ≤ {{ runtimeTimeoutSec }}s / {{ runtimeFuelYi }} 亿指令 ·
          线性内存 ≤ {{ pluginRuntimeInfo?.max_memory_mb }}MB ·
          HTTP 响应 ≤ {{ pluginRuntimeInfo?.max_http_response_mb }}MB ·
          单次文件读 ≤ {{ pluginRuntimeInfo?.max_fs_read_mb }}MB ·
          流式块 ≤ {{ pluginRuntimeInfo?.max_fs_chunk_mb }}MB（最多 {{ pluginRuntimeInfo?.max_fs_handles }} 个句柄）
        </div>
        <div v-else class="plugin-runtime-meta">
          本次构建关闭了 wasm-runtime：带 WASM 后端的插件无法执行，只有沙箱页面可用
          （调用会返回「运行时未启用」）。
        </div>
        <!-- 调试开关：插件调用出错时导出线性内存快照（编译期选项，切换后自动重编） -->
        <div v-if="runtimeReady" class="plugin-runtime-switch">
          <el-switch
            :model-value="memdumpOnError"
            :loading="memdumpBusy"
            :disabled="memdumpBusy"
            @change="toggleMemdump"
          />
          <span class="plugin-runtime-switch-label">出错时导出内存快照</span>
          <span class="plugin-runtime-switch-hint">
            排查用：调用出错时把插件线性内存（≤ {{ pluginRuntimeInfo?.max_memory_mb }}MB）
            写进其私有数据目录，反复出错会反复写盘，别长期开着
          </span>
        </div>
      </div>
    </div>

    <div class="plugin-toolbar">
      <app-button type="primary" :loading="pluginBusy" @click="installPluginFrom('zip')">
        从 ZIP 包安装
      </app-button>
      <app-button :loading="pluginBusy" @click="installPluginFrom('dir')">从目录安装</app-button>
      <span class="plugin-count">
        共 {{ pluginListItems.length }} 个插件（第三方 {{ thirdPartyPluginCount }} 个）·
        本窗口活动栏显示 {{ windowVisibleCount }} 个
      </span>
    </div>

    <div class="plugin-manage-list">
      <div
        v-for="(item, index) in pluginListItems"
        :key="item.id"
        class="plugin-card"
        :class="{ 'is-disabled': !item.enabled }"
      >
        <div class="plugin-order">
          <app-button
            size="small" circle :icon="Top" title="上移"
            :disabled="pluginBusy || index === 0 || !item.enabled"
            @click="movePlugin(item.id, -1)"
          />
          <app-button
            size="small" circle :icon="Bottom" title="下移"
            :disabled="pluginBusy || index === pluginListItems.length - 1 || !item.enabled"
            @click="movePlugin(item.id, 1)"
          />
        </div>

        <div class="plugin-info">
          <div class="plugin-name-row">
            <span class="plugin-name">{{ item.label }}</span>
            <el-tag v-if="item.source === 'builtin'" size="small" type="info">内置</el-tag>
            <el-tag v-else size="small" type="primary">第三方</el-tag>
            <el-tag v-if="item.source === 'third-party'" size="small">{{ item.version }}</el-tag>
            <el-tag v-if="!item.enabled" size="small" type="warning">已禁用</el-tag>
            <el-tag v-else-if="item.pinned" size="small" type="info" title="宿主常驻入口，不可禁用">常驻</el-tag>
            <el-tag
              v-if="item.hasWasm"
              size="small"
              :type="runtimeReady ? 'success' : 'danger'"
            >
              {{ runtimeReady ? '含 WASM 后端' : 'WASM 未启用' }}
            </el-tag>
            <!-- 入口形态：声明了 standaloneWindow 的插件，点活动栏图标直接开独立窗口 -->
            <el-tag v-if="item.standaloneWindow" size="small">独立窗口</el-tag>
          </div>
          <div class="plugin-meta">
            <span class="plugin-id">{{ item.id }}</span>
            <template v-if="item.description">
              <span class="plugin-sep">|</span>
              <span>{{ item.description }}</span>
            </template>
            <template v-if="item.source === 'third-party'">
              <span class="plugin-sep">|</span>
              <span>{{ formatPluginSize(item.sizeBytes) }}</span>
            </template>
          </div>
          <div v-if="item.permissions.length" class="plugin-perms">
            <span class="plugin-perms-label">权限声明</span>
            <span v-for="perm in item.permissions" :key="perm" class="plugin-perm">{{ perm }}</span>
          </div>
          <!-- 上一次调用的资源占用（宿主本次会话没调用过则不显示） -->
          <div v-if="item.lastStats" class="plugin-stats">
            <span class="plugin-perms-label">上次调用</span>
            <span class="plugin-stat" :class="{ 'is-fail': !item.lastStats.ok }">
              {{ statsSummary(item.lastStats) }}
            </span>
            <span v-if="item.lastStats.memdump_path" class="plugin-stat-path">
              {{ item.lastStats.memdump_path }}
            </span>
          </div>
        </div>

        <div class="plugin-ops">
          <app-button
            v-if="item.source === 'third-party' && item.hasWasm"
            size="small" plain
            @click="loadStats(item.id)"
          >
            资源
          </app-button>
          <!-- 本窗口的插件清单开关：写回**本窗口所属窗口预设**的 enabledPlugins
               （与右边的启用/禁用是两件事 —— 那个是全局的，会影响所有窗口）。
               宿主常驻入口（设置，pinnedBottom）固定在活动栏底部、不受清单影响，故不给。 -->
          <app-button
            v-if="!item.pinned"
            size="small" plain
            :type="isWindowPluginVisible(item.id) ? '' : 'primary'"
            :disabled="pluginBusy || !!windowPluginBlockReason(item)"
            :title="windowPluginTitle(item)"
            @click="toggleWindowPlugin(item)"
          >
            {{ isWindowPluginVisible(item.id) ? '移出' : '加入' }}
          </app-button>
          <!-- 宿主常驻入口（设置）不给开关：关掉它就没有插件管理页面了 -->
          <el-switch
            v-if="!item.pinned"
            :model-value="item.enabled"
            :disabled="pluginBusy"
            :loading="pluginBusy"
            @change="togglePluginEnabled(item.id, $event)"
          />
          <!-- 只有清单声明了 contributes.configuration 的插件才有：设置在宿主统一管理 -->
          <app-button
            v-if="item.settings.length"
            size="small" plain
            @click="openPluginSettings(item)"
          >
            设置
          </app-button>
          <app-button
            v-if="item.source === 'third-party'"
            size="small" type="danger" plain :disabled="pluginBusy"
            @click="askUninstallPlugin(item)"
          >
            卸载
          </app-button>
        </div>
      </div>
    </div>
  </section>

  <!-- 插件开发：第三方插件开发文档（原文在 docs/plugin-dev/，构建期打进来） -->
  <div v-show="tab === 'dev'" class="plugin-dev-pane">
    <PluginDevDocs />
  </div>

  <!-- 插件卸载确认 -->
  <GlobalDialog v-model="showUninstallDialog" title="卸载插件" width="40%">
    <div class="plugin-uninstall">
      <div class="uninstall-text">
        确定卸载「{{ pendingUninstall?.label }}」（<span class="plugin-id">{{ pendingUninstall?.id }}</span>）吗？
      </div>
      <el-checkbox v-model="uninstallRemoveData">
        同时删除该插件在所有工作区的私有数据（不可恢复）
      </el-checkbox>
    </div>
    <template #footer>
      <app-button @click="showUninstallDialog = false">取消</app-button>
      <app-button type="danger" :loading="pluginBusy" @click="confirmUninstallPlugin">卸载</app-button>
    </template>
  </GlobalDialog>

  <!-- 插件设置：清单 contributes.configuration 声明的项，宿主统一渲染 -->
  <GlobalDialog
    v-model="showSettingsDialog"
    :title="`插件设置 · ${settingsPlugin?.label ?? ''}`"
    width="45%"
  >
    <div v-if="settingsPlugin" class="plugin-settings">
      <div v-for="def in settingsPlugin.settings" :key="def.key" class="plugin-setting-row">
        <div class="plugin-setting-head">
          <span class="plugin-setting-title">{{ def.title }}</span>
          <span class="plugin-setting-key">{{ def.key }}</span>
        </div>
        <div v-if="def.description" class="plugin-setting-desc">{{ def.description }}</div>

        <el-input
          v-if="def.type === 'string'"
          :model-value="String(settingsValues[def.key] ?? '')"
          size="small"
          @update:model-value="(v: string | number) => setSettingValue(def.key, v)"
        />
        <el-switch
          v-else-if="def.type === 'boolean'"
          :model-value="settingsValues[def.key] === true"
          @change="(v: boolean) => setSettingValue(def.key, v)"
        />
        <el-input-number
          v-else-if="def.type === 'number'"
          :model-value="Number(settingsValues[def.key] ?? 0)"
          size="small"
          :min="def.min" :max="def.max"
          @update:model-value="(v: number | undefined) => setSettingValue(def.key, v ?? 0)"
        />
        <el-select
          v-else
          :model-value="String(settingsValues[def.key] ?? '')"
          size="small"
          @update:model-value="(v: unknown) => setSettingValue(def.key, String(v ?? ''))"
        >
          <el-option v-for="opt in def.enum ?? []" :key="opt" :label="opt" :value="opt" />
        </el-select>
      </div>
    </div>
    <template #footer>
      <app-button @click="showSettingsDialog = false">取消</app-button>
      <app-button type="primary" :loading="settingsBusy" @click="confirmPluginSettings">
        保存
      </app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { Bottom, Cpu, Top } from '@element-plus/icons-vue'
import { open, publish } from '@/core/runtime'
import { fetchPluginSettings, savePluginSettings } from '@/core/plugin/api'
import { PLUGIN_SETTINGS_CHANGED_EVENT } from '@/core/plugin/registry'
import { currentPresetId } from '@/core/windowPresets'
import useStore from '@/store/index'
import GlobalDialog from '@/components/GlobalDialog.vue'
import {
  installThirdPartyPlugin,
  movePluginInOrder,
  pluginListItems,
  pluginRuntimeInfo,
  refreshPluginLastStats,
  refreshPluginRuntimeInfo,
  setPluginMemdumpEnabled,
  setPluginEnabled,
  uninstallThirdPartyPlugin,
} from '@/core/plugin/registry'
import type { CallStats, PluginListItem } from '@/core/plugin/types'
import NotificationUtil from '@/utils/NotificationUtil'
import PluginDevDocs from '../PluginDevDocs.vue'
import {
  ensureWindowPresetsLoaded,
  findPreset,
  loadWindowPresets,
  toggleWindowPluginInCurrentWindow,
} from './useWindowPresetsStore'
import type { PluginTab } from './types'

const props = defineProps<{
  /** 插件分区是否处于激活状态（父级用 v-show 切分区，面板本身常驻挂载） */
  active: boolean
  /** 插件分区内部页签（页签按钮在父级的顶部固定条里） */
  tab: PluginTab
}>()

/** 当前窗口的布局 store：本窗口显示清单（enabledPlugins）的唯一读写口 */
const { useLayoutStore } = useStore()

/** 插件操作进行中（安装/卸载/启停/排序期间禁用交互，避免并发写盘） */
const pluginBusy = ref(false)
/** 卸载确认弹窗 */
const showUninstallDialog = ref(false)
/** 卸载时是否一并清理插件私有数据 */
const uninstallRemoveData = ref(false)
/** 待卸载的插件（弹窗展示用） */
const pendingUninstall = ref<PluginListItem | null>(null)

/** 已安装的第三方插件数量 */
const thirdPartyPluginCount = computed(
  () => pluginListItems.value.filter(p => p.source === 'third-party').length,
)

/**
 * 本窗口活动栏当前显示的插件数。
 *
 * 口径对齐真实的图标数：既要「在本窗口的显示清单里」，也要「已启用」——被禁用的插件已从
 * `pluginManager` 注销，即使留在清单里也不会画图标。`pinned`（设置）固定在底部，不计入。
 */
const windowVisibleCount = computed(
  () =>
    pluginListItems.value.filter(
      p => !p.pinned && p.enabled && useLayoutStore.isPluginVisibleInWindow(p.id),
    ).length,
)

/**
 * 本窗口是否显示该插件（活动栏图标 / 侧栏面板）。
 *
 * 事实源是本窗口的显示清单 `layoutStore.enabledPlugins` —— 它就是**本窗口所属窗口预设**
 * 的 `enabledPlugins`：「加入 / 移出」会同时改两边（见 `useWindowPresetsStore`）。
 */
function isWindowPluginVisible(id: string): boolean {
  return useLayoutStore.isPluginVisibleInWindow(id)
}

/** 本窗口所属的窗口预设 ID / 是否已解析（按钮可用性要在渲染期同步判定，故提前解析） */
const thisWindowPresetId = ref<string | null>(null)
const presetResolved = ref(false)
/** 当前窗口的预设：改完清单后 `allPresets` 会换对象，用 computed 保证不持有旧引用 */
const currentPreset = computed(() => findPreset(thisWindowPresetId.value))

/** 描述里那句「这份清单属于哪个预设」（读不到就直说，别让用户以为这是全局开关） */
const presetScopeHint = computed(() => {
  if (!presetResolved.value) return ''
  const preset = currentPreset.value
  if (!preset) return '本窗口不属于任何窗口预设，这份清单无法保存。'
  return `当前对应预设「${preset.name}」。`
})

/**
 * 「加入 / 移出」当前不可用的原因（空串 = 可点）。
 *
 * 禁用理由必须写清楚，不做「点了没反应」的哑操作：
 *  - 本窗口不在任何预设里 ⇒ 没有可落盘的地方；
 *  - 专注模式预设恰好一个插件（预设保存时的校验），两个方向都不可用；
 *  - 未启用的插件已从 `pluginManager` 注销，加进清单也画不出图标；
 *  - 移出最后一个会把清单清空，而空清单 = 显示全部 —— 等于全都又回来了。
 */
function windowPluginBlockReason(item: PluginListItem): string {
  if (!presetResolved.value) return '正在读取窗口预设…'
  if (!currentPreset.value) return '本窗口不属于任何窗口预设，这份清单无法保存'
  if (currentPreset.value.focusMode) {
    return '专注模式下只能保留一个插件：先在「窗口设置」里关掉专注模式'
  }
  if (!item.enabled && !isWindowPluginVisible(item.id)) {
    return '插件已禁用：先启用，再加入本窗口'
  }
  if (isWindowPluginVisible(item.id) && useLayoutStore.resolveWindowPluginList().length <= 1) {
    return '至少保留一个插件'
  }
  return ''
}

/** 按钮 tooltip：把「为什么点不了」和「点完影响谁」说清楚（作用域最容易误解） */
function windowPluginTitle(item: PluginListItem): string {
  const blocked = windowPluginBlockReason(item)
  if (blocked) return blocked
  const where = currentPreset.value ? `「${currentPreset.value.name}」预设` : '窗口预设'
  return isWindowPluginVisible(item.id)
    ? `从本窗口活动栏移出，并写回${where}的插件清单（该预设以后打开的窗口也不再显示）`
    : `加入本窗口活动栏，并写回${where}的插件清单（该预设以后打开的窗口也会显示）`
}

/**
 * 加入 / 移出本窗口的插件（= 编辑本窗口所属窗口预设的插件清单）。
 *
 * 视图与落盘在 `useWindowPresetsStore` 里一次做完，这里只管交互反馈：失败给原因，成功给结论。
 */
async function toggleWindowPlugin(item: PluginListItem) {
  pluginBusy.value = true
  try {
    const res = await toggleWindowPluginInCurrentWindow(item.id)
    if (!res.ok) {
      NotificationUtil.error(res.reason)
      return
    }
    NotificationUtil.success(
      res.visible ? `「${item.label}」已加入本窗口活动栏` : `「${item.label}」已移出本窗口活动栏`,
    )
  } catch (e) {
    NotificationUtil.error(`更新窗口插件清单失败：${pluginErrorText(e)}`)
  } finally {
    pluginBusy.value = false
  }
}

/** WASM 运行时是否可用（未读到 / 构建期关闭都为 false） */
const runtimeReady = computed(() => pluginRuntimeInfo.value?.available === true)
const runtimeName = computed(() => pluginRuntimeInfo.value?.runtime || 'WASM')
/** 以「秒」展示单次调用超时 */
const runtimeTimeoutSec = computed(() =>
  Math.round((pluginRuntimeInfo.value?.timeout_ms ?? 0) / 1000),
)
/** 以「亿指令」展示 fuel 上限 */
const runtimeFuelYi = computed(() =>
  Math.round((pluginRuntimeInfo.value?.fuel ?? 0) / 100_000_000),
)

/** 出错内存快照开关（读后端 settings.json 的 plugin_memdump_on_error） */
const memdumpOnError = computed(() => pluginRuntimeInfo.value?.memdump_on_error === true)
/** 出错快照开关切换中 */
const memdumpBusy = ref(false)

// 进入插件管理页时刷新一次统计（宿主只在真正调用过之后才有记录）
watch(
  () => props.active && props.tab === 'manage',
  visible => {
    if (visible) void refreshAllStats()
  },
  { immediate: true },
)

/**
 * 解析「本窗口属于哪个窗口预设」——「加入 / 移出」写的就是它的插件清单。
 *
 * 读注册表可能失败（辅助窗口压根不在注册表里，属于正常情况），所以失败只降级：
 * 按钮变成不可点并说明原因，页面其余部分照常可用。
 */
onMounted(async () => {
  try {
    await ensureWindowPresetsLoaded()
    thisWindowPresetId.value = await currentPresetId()
  } catch (e) {
    console.warn('读取窗口预设失败，「加入 / 移出」将不可用:', e)
  } finally {
    presetResolved.value = true
  }
})

/** 切换「插件调用出错时导出线性内存快照」 */
async function toggleMemdump(value: unknown) {
  const on = Boolean(value)
  memdumpBusy.value = true
  try {
    await setPluginMemdumpEnabled(on)
    NotificationUtil.success(
      on ? '已开启出错内存快照：插件调用出错时导出线性内存' : '已关闭出错内存快照',
    )
  } catch (e) {
    NotificationUtil.error(`更新出错快照开关失败：${pluginErrorText(e)}`)
    // 失败时把开关拉回后端真实状态，避免界面显示与磁盘配置不一致
    await refreshPluginRuntimeInfo()
  } finally {
    memdumpBusy.value = false
  }
}

/** 按需拉取某插件的调用统计（无记录时给出明确提示而不是空白） */
// -------------------------------------------------- 插件设置（contributes.configuration）
// 值存 settings.json 的 plugin_settings：能随宿主设置统一导出 / 重置，
// 宿主也不必去解读插件自己的 KV 结构。
const showSettingsDialog = ref(false)
const settingsBusy = ref(false)
const settingsPlugin = ref<PluginListItem | null>(null)
const settingsValues = ref<Record<string, unknown>>({})

/** 控件写回（模板统一走它，避免在模板上做类型断言） */
function setSettingValue(key: string, v: unknown) {
  settingsValues.value = { ...settingsValues.value, [key]: v }
}

async function openPluginSettings(item: PluginListItem) {
  settingsPlugin.value = item
  settingsBusy.value = true
  try {
    settingsValues.value = await fetchPluginSettings(item.id)
    showSettingsDialog.value = true
  } catch (e) {
    NotificationUtil.error(e instanceof Error ? e.message : '读取插件设置失败')
  } finally {
    settingsBusy.value = false
  }
}

async function confirmPluginSettings() {
  const item = settingsPlugin.value
  if (!item) return
  settingsBusy.value = true
  try {
    await savePluginSettings(item.id, settingsValues.value)
    // 广播给已打开的插件面板：它不必重新握手就能拿到新值
    await publish(PLUGIN_SETTINGS_CHANGED_EVENT, {
      id: item.id,
      values: { ...settingsValues.value },
    })
    showSettingsDialog.value = false
  } catch (e) {
    NotificationUtil.error(e instanceof Error ? e.message : '保存插件设置失败')
  } finally {
    settingsBusy.value = false
  }
}

async function loadStats(id: string) {
  const s = await refreshPluginLastStats(id)
  if (!s) NotificationUtil.info('该插件在本次会话中还没有调用记录')
}

/** 拉取全部「带 WASM 后端的第三方插件」的调用统计（进入插件管理页时静默执行） */
async function refreshAllStats() {
  const ids = pluginListItems.value
    .filter(p => p.source === 'third-party' && p.hasWasm)
    .map(p => p.id)
  if (!ids.length) return
  await Promise.all(ids.map(id => refreshPluginLastStats(id)))
}

/** 字节数格式化（统计口径用；null 表示「拿不到」，由调用方决定怎么显示） */
function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) return '—'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

/** 上次调用统计摘要（单行文字，供插件卡片展示） */
function statsSummary(s: CallStats): string {
  const parts: string[] = [`${s.func}()`, s.ok ? '成功' : '失败']
  if (s.fuel_used != null) parts.push(`${(s.fuel_used / 1e6).toFixed(1)}M 指令`)
  // 进程增量是近似口径：插件线性内存属于宿主进程，负数（回收多于分配）按 0 计
  if (s.mem_delta_bytes != null) parts.push(`进程内存 +${formatBytes(s.mem_delta_bytes)}`)
  if (s.rss_delta_bytes != null) parts.push(`驻留 +${formatBytes(s.rss_delta_bytes)}`)
  if (s.memdump_bytes != null) parts.push(`出错快照 ${formatBytes(s.memdump_bytes)}`)
  return parts.join(' · ')
}

/** 插件包体积格式化 */
function formatPluginSize(bytes: number): string {
  if (!bytes) return '0 B'
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

/** 插件操作失败统一提示（后端返回的 msg 已是可读中文） */
function pluginErrorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

/**
 * 安装 / 卸载插件后把「本窗口清单」对齐到预设。
 *
 * 后端在装 / 卸时会顺带改各预设的**显式**插件清单（装则追加、卸则摘除），所以必须重读一次
 * 磁盘：否则内存里那份会在下一次整份写回时把后端的改动覆盖掉。重读后应用到本窗口，
 * 保证「本窗口清单 == 本窗口所属预设的清单」这条不变量不被破坏。
 */
async function syncCurrentWindowPluginListFromPreset() {
  try {
    await loadWindowPresets()
  } catch (e) {
    console.warn('重读窗口预设失败，本窗口清单可能与预设不一致:', e)
    return
  }
  const preset = currentPreset.value
  if (preset) useLayoutStore.setWindowPluginList(preset.enabledPlugins)
}

/** 安装插件：ZIP 包或已解开的插件目录 */
async function installPluginFrom(kind: 'zip' | 'dir') {
  const picked = await open({
    multiple: false,
    directory: kind === 'dir',
    title: kind === 'zip' ? '选择插件包（.zip）' : '选择插件目录',
    ...(kind === 'zip' ? { filters: [{ name: '插件包', extensions: ['zip'] }] } : {}),
  })
  // 用户取消
  if (!picked || typeof picked !== 'string') return
  pluginBusy.value = true
  try {
    const item = await installThirdPartyPlugin(picked)
    await syncCurrentWindowPluginListFromPreset()
    NotificationUtil.success(`插件「${item.name}」安装成功`)
  } catch (e) {
    NotificationUtil.error(`安装失败：${pluginErrorText(e)}`)
  } finally {
    pluginBusy.value = false
  }
}

/** 启用 / 禁用插件（内置与第三方同一入口；与窗口预设里的插件勾选区分命名） */
async function togglePluginEnabled(id: string, value: unknown) {
  pluginBusy.value = true
  try {
    await setPluginEnabled(id, Boolean(value))
  } catch (e) {
    NotificationUtil.error(`更新插件状态失败：${pluginErrorText(e)}`)
  } finally {
    pluginBusy.value = false
  }
}

/** 在统一顺序里上移 / 下移 */
async function movePlugin(id: string, delta: number) {
  pluginBusy.value = true
  try {
    await movePluginInOrder(id, delta)
  } catch (e) {
    NotificationUtil.error(`调整顺序失败：${pluginErrorText(e)}`)
  } finally {
    pluginBusy.value = false
  }
}

/** 打开卸载确认弹窗 */
function askUninstallPlugin(item: PluginListItem) {
  pendingUninstall.value = item
  uninstallRemoveData.value = false
  showUninstallDialog.value = true
}

/** 确认卸载 */
async function confirmUninstallPlugin() {
  const item = pendingUninstall.value
  if (!item) return
  pluginBusy.value = true
  try {
    await uninstallThirdPartyPlugin(item.id, uninstallRemoveData.value)
    await syncCurrentWindowPluginListFromPreset()
    NotificationUtil.success(`插件「${item.label}」已卸载`)
    showUninstallDialog.value = false
    pendingUninstall.value = null
  } catch (e) {
    NotificationUtil.error(`卸载失败：${pluginErrorText(e)}`)
  } finally {
    pluginBusy.value = false
  }
}
</script>

<style lang="scss" scoped>
@use '@/styles/theme.scss' as theme;

.plugin-dev-pane {
  flex: 1;
  min-height: 0;
  padding: var(--space-3) var(--space-4) var(--space-4);
  box-sizing: border-box;
}

// ---- 工具栏 ----
.plugin-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}

.plugin-count {
  margin-left: auto;
  font-size: var(--font-size-base);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

// ---- WASM 运行时状态条 ----
.plugin-runtime {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 12px;
  margin-bottom: 12px;
  border-radius: 6px;

  @include theme.useTheme {
    // 用 sidebarBg 而不是 hoverColor：hoverColor 是「悬停/选中」语义
    // （暗色下是饱和蓝 #4b6eaf），拿来当静态底色会变成一大块高亮蓝。
    background-color: theme.getVar('sidebarBg');
    border: 1px solid theme.getVar('borderColor');
    color: theme.getVar('activityBarActive');
  }

  &.is-off {
    @include theme.useTheme {
      color: var(--el-color-danger);
    }
  }
}

.plugin-runtime-ic {
  flex: none;
  margin-top: 2px;
}

.plugin-runtime-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.plugin-runtime-title {
  font-size: var(--font-size-base);
  font-weight: 600;
}

.plugin-runtime-meta {
  font-size: var(--font-size-sm);
  line-height: 1.5;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

// 运行时横幅里的调试开关行（出错内存快照）
.plugin-runtime-switch {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 6px;
  font-size: var(--font-size-sm);
}

.plugin-runtime-switch-label {
  font-weight: 500;
}

.plugin-runtime-switch-hint {
  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

// ---- 插件卡片列表 ----
.plugin-manage-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.plugin-settings {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.plugin-setting-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.plugin-setting-head {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.plugin-setting-title {
  font-size: var(--font-size-base);

  @include theme.useTheme {
    color: theme.getVar('textPrimary');
  }
}

.plugin-setting-key {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-small);

  @include theme.useTheme {
    color: theme.getVar('textTertiary');
  }
}

.plugin-setting-desc {
  font-size: var(--font-size-small);
  line-height: 1.5;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.plugin-card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 6px;

  @include theme.useTheme {
    // 与 .settings-group-body 同源（bgColor + 细边框）；
    // 这里原本用 hoverColor 当底色，暗色下每张卡都是一整块高亮蓝
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
  }

  &.is-disabled {
    opacity: 0.6;
  }
}

.plugin-order {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: none;
}

.plugin-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.plugin-name-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
}

.plugin-name {
  font-size: var(--font-size-md);
  font-weight: 500;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.plugin-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  font-size: var(--font-size-base);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.plugin-id {
  font-family: var(--font-family-mono);
}

.plugin-sep {
  opacity: 0.5;
}

.plugin-perms {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 6px;
  font-size: var(--font-size-base);
}

.plugin-perms-label {
  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.plugin-perm {
  padding: 0 6px;
  border-radius: 3px;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    color: theme.getVar('textSecondary');
  }
}

.plugin-ops {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: none;
}

// 插件卡片里的「上次调用」资源占用行
.plugin-stats {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 6px;
  font-size: var(--font-size-base);
}

.plugin-stat {
  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &.is-fail {
    @include theme.useTheme {
      color: theme.getVar('dangerText');
    }
  }
}

// 出错快照路径（标识符类文本：全局 13px 下 `_` 几乎不可见，必须用等宽字体）
.plugin-stat-path {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  word-break: break-all;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

// ---- 卸载确认弹窗 ----
.plugin-uninstall {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.uninstall-text {
  font-size: var(--font-size-md);
  line-height: 1.6;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}
</style>
