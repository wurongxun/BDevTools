<template>
  <!-- 插件管理 -->
  <section v-show="tab === 'manage'" class="settings-group">
    <p class="settings-group-desc">{{ $t('settingsPlugin.plugin.manageDesc1') }}</p>
    <p class="settings-group-desc">{{ $t('settingsPlugin.plugin.manageDesc2') }}</p>
    <p class="settings-group-desc">
      {{ $t('settingsPlugin.plugin.manageDesc3') }}
      {{ presetScopeHint }}
    </p>

    <!-- WASM 运行时状态：第三方插件的后端逻辑跑在这里 -->
    <div class="plugin-runtime" :class="{ 'is-off': !runtimeReady }">
      <el-icon class="plugin-runtime-ic"><Cpu /></el-icon>
      <div class="plugin-runtime-body">
        <div class="plugin-runtime-title">
          {{ $t('settingsPlugin.plugin.wasmRuntime') }}：{{ runtimeReady ? runtimeName : $t('settingsPlugin.plugin.wasmDisabled') }}
        </div>
        <div v-if="runtimeReady" class="plugin-runtime-meta">
          {{ $t('settingsPlugin.plugin.wasmMeta', { timeout: runtimeTimeoutSec, fuel: runtimeFuelYi, mem: pluginRuntimeInfo?.max_memory_mb, http: pluginRuntimeInfo?.max_http_response_mb, fs: pluginRuntimeInfo?.max_fs_read_mb, chunk: pluginRuntimeInfo?.max_fs_chunk_mb, handles: pluginRuntimeInfo?.max_fs_handles }) }}
        </div>
        <div v-else class="plugin-runtime-meta">
          {{ $t('settingsPlugin.plugin.wasmOffDesc') }}
        </div>
        <!-- 调试开关：插件调用出错时导出线性内存快照（编译期选项，切换后自动重编） -->
        <div v-if="runtimeReady" class="plugin-runtime-switch">
          <el-switch
            :model-value="memdumpOnError"
            :loading="memdumpBusy"
            :disabled="memdumpBusy"
            @change="toggleMemdump"
          />
          <span class="plugin-runtime-switch-label">{{ $t('settingsPlugin.plugin.memdumpLabel') }}</span>
          <span class="plugin-runtime-switch-hint">
            {{ $t('settingsPlugin.plugin.memdumpHint', { mb: pluginRuntimeInfo?.max_memory_mb }) }}
          </span>
        </div>
      </div>
    </div>

    <div class="plugin-toolbar">
      <app-button type="primary" :loading="pluginBusy" @click="installPluginFrom('zip')">
        {{ $t('settingsPlugin.plugin.installFromZip') }}
      </app-button>
      <app-button :loading="pluginBusy" @click="installPluginFrom('dir')">{{ $t('settingsPlugin.plugin.installFromDir') }}</app-button>
      <span class="plugin-count">
        {{ $t('settingsPlugin.plugin.pluginCount', { total: pluginListItems.length, tp: thirdPartyPluginCount, n: windowVisibleCount }) }}
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
            size="small" circle :icon="Top" :title="$t('settingsPlugin.plugin.moveUp')"
            :disabled="pluginBusy || index === 0 || !item.enabled"
            @click="movePlugin(item.id, -1)"
          />
          <app-button
            size="small" circle :icon="Bottom" :title="$t('settingsPlugin.plugin.moveDown')"
            :disabled="pluginBusy || index === pluginListItems.length - 1 || !item.enabled"
            @click="movePlugin(item.id, 1)"
          />
        </div>

        <div class="plugin-info">
          <div class="plugin-name-row">
            <span class="plugin-name">{{ item.label }}</span>
            <el-tag v-if="item.source === 'builtin'" size="small" type="info">{{ $t('settingsPlugin.plugin.tagBuiltin') }}</el-tag>
            <el-tag v-else size="small" type="primary">{{ $t('settingsPlugin.plugin.tagThirdParty') }}</el-tag>
            <el-tag v-if="item.source === 'third-party'" size="small">{{ item.version }}</el-tag>
            <el-tag v-if="!item.enabled" size="small" type="warning">{{ $t('settingsPlugin.plugin.tagDisabled') }}</el-tag>
            <el-tag
              v-else-if="item.pinned"
              size="small"
              type="info"
              :title="$t('settingsPlugin.plugin.tagPinnedTitle')"
            >{{ $t('settingsPlugin.plugin.tagPinned') }}</el-tag>
            <el-tag
              v-if="item.hasWasm"
              size="small"
              :type="runtimeReady ? 'success' : 'danger'"
            >
              {{ runtimeReady ? $t('settingsPlugin.plugin.tagWasmOn') : $t('settingsPlugin.plugin.tagWasmOff') }}
            </el-tag>
            <!-- 入口形态：声明了 standaloneWindow 的插件，点活动栏图标直接开独立窗口 -->
            <el-tag v-if="item.standaloneWindow" size="small">{{ $t('settingsPlugin.plugin.tagStandalone') }}</el-tag>
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
          <!-- 依赖：声明了谁，以及当前哪些不可用（没打包 / 没安装 / 被停用）。
               宿主不自动补依赖，所以「缺了什么」必须让用户一眼看到 -->
          <div v-if="item.dependencies.length" class="plugin-perms">
            <span class="plugin-perms-label">{{ $t('settingsPlugin.plugin.depsLabel') }}</span>
            <span
              v-for="dep in item.dependencies"
              :key="dep"
              class="plugin-perm"
              :class="{ 'is-missing': item.missingDependencies.includes(dep) }"
              :title="item.missingDependencies.includes(dep) ? $t('settingsPlugin.plugin.depMissingTitle', { dep }) : dep"
            >
              {{ dep }}
            </span>
            <span v-if="item.missingDependencies.length" class="plugin-deps-warn">
              {{ $t('settingsPlugin.plugin.depsWarn', { n: item.missingDependencies.length }) }}
            </span>
          </div>
          <!-- 激活耗时：懒激活之后「装得多」不再等于「启动慢」，这个数字说明慢是谁造成的。
               未激活显示「未激活」而不是 0 —— 0 会跟「快到测不出来」混淆 -->
          <div class="plugin-perms">
            <span class="plugin-perms-label">{{ $t('settingsPlugin.plugin.activationLabel') }}</span>
            <span class="plugin-stat">
              {{ item.activationMs === null ? $t('settingsPlugin.plugin.activationOff') : `${item.activationMs} ms` }}
            </span>
          </div>
          <div v-if="item.permissions.length" class="plugin-perms">
            <span class="plugin-perms-label">{{ $t('settingsPlugin.plugin.permsLabel') }}</span>
            <span v-for="perm in item.permissions" :key="perm" class="plugin-perm">{{ perm }}</span>
          </div>
          <!-- 页面能力：**可收紧**（只能减，不能加）。生效口径 = 声明 − 收紧，
               收紧后该项对应的 CSP 指令消失，下一次加载插件页生效。 -->
          <div v-if="item.pageCapabilities.length" class="plugin-perms">
            <span class="plugin-perms-label">{{ $t('settingsPlugin.plugin.pageCapsLabel') }}</span>
            <button
              v-for="cap in item.pageCapabilities"
              :key="cap"
              type="button"
              class="plugin-cap"
              :class="{ 'is-denied': item.deniedPageCapabilities.includes(cap) }"
              :title="capTitle(cap, item)"
              :disabled="pluginBusy"
              @click="togglePluginCap(item, cap)"
            >
              {{ capLabel(cap) }}
            </button>
          </div>
          <div v-if="deniedCapsOf(item).length" class="plugin-caps-warn">
            <span class="plugin-caps-warn-text">
              {{ $t('settingsPlugin.plugin.capsWarn', { n: deniedCapsOf(item).length }) }}
            </span>
            <app-button size="small" plain :disabled="pluginBusy" @click="restorePluginCaps(item)">
              {{ $t('settingsPlugin.plugin.restoreDefault') }}
            </app-button>
          </div>
          <!-- 上一次调用的资源占用（宿主本次会话没调用过则不显示） -->
          <div v-if="item.lastStats" class="plugin-stats">
            <span class="plugin-perms-label">{{ $t('settingsPlugin.plugin.lastCallLabel') }}</span>
            <span class="plugin-stat" :class="{ 'is-fail': !item.lastStats.ok }">
              {{ statsSummary(item.lastStats) }}
            </span>
            <span v-if="item.lastStats.memdump_path" class="plugin-stat-path">
              {{ item.lastStats.memdump_path }}
            </span>
          </div>
          <!-- 熔断：解释「为什么这个插件现在点不动」—— 冷却期内后端直接拒调用，
               页面上什么都不说的话，用户只会以为界面坏了 -->
          <div v-if="item.lastStats?.circuit_cooldown_ms" class="plugin-caps-warn">
            <span class="plugin-caps-warn-text">
              {{ $t('settingsPlugin.plugin.circuitWarn', { n: item.lastStats.consec_failures, sec: Math.ceil(item.lastStats.circuit_cooldown_ms / 1000) }) }}
            </span>
          </div>
        </div>

        <div class="plugin-ops">
          <app-button
            v-if="item.source === 'third-party' && item.hasWasm"
            size="small" plain
            @click="loadStats(item.id)"
          >
            {{ $t('settingsPlugin.plugin.btnStats') }}
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
            {{ isWindowPluginVisible(item.id) ? $t('settingsPlugin.plugin.btnRemove') : $t('settingsPlugin.plugin.btnAdd') }}
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
            {{ $t('settingsPlugin.plugin.btnSettings') }}
          </app-button>
          <app-button
            v-if="item.source === 'third-party'"
            size="small" type="danger" plain :disabled="pluginBusy"
            @click="askUninstallPlugin(item)"
          >
            {{ $t('settingsPlugin.plugin.btnUninstall') }}
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
  <GlobalDialog v-model="showUninstallDialog" :title="$t('settingsPlugin.plugin.uninstallTitle')" width="40%">
    <div class="plugin-uninstall">
      <div class="uninstall-text">
        {{ $t('settingsPlugin.plugin.uninstallText', { label: pendingUninstall?.label, id: pendingUninstall?.id }) }}
      </div>
      <el-checkbox v-model="uninstallRemoveData">
        {{ $t('settingsPlugin.plugin.uninstallRemoveData') }}
      </el-checkbox>
    </div>
    <template #footer>
      <app-button @click="showUninstallDialog = false">{{ $t('settingsPlugin.common.cancel') }}</app-button>
      <app-button type="danger" :loading="pluginBusy" @click="confirmUninstallPlugin">{{ $t('settingsPlugin.plugin.btnUninstall') }}</app-button>
    </template>
  </GlobalDialog>

  <!-- 插件设置：清单 contributes.configuration 声明的项，宿主统一渲染 -->
  <GlobalDialog
    v-model="showSettingsDialog"
    :title="$t('settingsPlugin.plugin.settingsTitle', { label: settingsPlugin?.label ?? '' })"
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
      <app-button @click="showSettingsDialog = false">{{ $t('settingsPlugin.common.cancel') }}</app-button>
      <app-button type="primary" :loading="settingsBusy" @click="confirmPluginSettings">
        {{ $t('settingsPlugin.common.save') }}
      </app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Bottom, Cpu, Top } from '@element-plus/icons-vue'
import { open, publish } from '@/core/runtime'
import { fetchPluginSettings, savePluginSettings } from '@/core/plugin/api'
import { PLUGIN_SETTINGS_CHANGED_EVENT } from '@/core/plugin/registry'
import { currentPresetId } from '@/core/windowPresets'
import useStore from '@/store'
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
  setPluginPageCapsDenied,
  uninstallThirdPartyPlugin,
} from '@/core/plugin/registry'
import { PAGE_CAPABILITY_LABELS, type CallStats, type PluginListItem } from '@/core/plugin/types'
import NotificationUtil from '@/utils/NotificationUtil'
import PluginDevDocs from '@/plugins/settings/app/PluginDevDocs.vue'
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

/** i18n：模板走 $t()，脚本里的提示文案走 t() */
const { t } = useI18n()

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
  if (!preset) return t('settingsPlugin.plugin.presetNoScope')
  return t('settingsPlugin.plugin.presetBelongsTo', { name: preset.name })
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
  if (!presetResolved.value) return t('settingsPlugin.plugin.readingPreset')
  if (!currentPreset.value) return t('settingsPlugin.plugin.notInPreset')
  if (currentPreset.value.focusMode) {
    return t('settingsPlugin.plugin.focusModeBlock')
  }
  if (!item.enabled && !isWindowPluginVisible(item.id)) {
    return t('settingsPlugin.plugin.disabledBlock')
  }
  if (isWindowPluginVisible(item.id) && useLayoutStore.resolveWindowPluginList().length <= 1) {
    return t('settingsPlugin.plugin.keepOneBlock')
  }
  return ''
}

/** 按钮 tooltip：把「为什么点不了」和「点完影响谁」说清楚（作用域最容易误解） */
function windowPluginTitle(item: PluginListItem): string {
  const blocked = windowPluginBlockReason(item)
  if (blocked) return blocked
  const where = currentPreset.value
    ? t('settingsPlugin.plugin.presetClause', { name: currentPreset.value.name })
    : t('settingsPlugin.plugin.presetGeneric')
  return isWindowPluginVisible(item.id)
    ? t('settingsPlugin.plugin.removeFromWindowTitle', { preset: where })
    : t('settingsPlugin.plugin.addToWindowTitle', { preset: where })
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
      res.visible
        ? t('settingsPlugin.plugin.addedToWindow', { label: item.label })
        : t('settingsPlugin.plugin.removedFromWindow', { label: item.label }),
    )
  } catch (e) {
    NotificationUtil.error(t('settingsPlugin.plugin.updateWindowListFailed', { msg: pluginErrorText(e) }))
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
      on ? t('settingsPlugin.plugin.memdumpOn') : t('settingsPlugin.plugin.memdumpOff'),
    )
  } catch (e) {
    NotificationUtil.error(t('settingsPlugin.plugin.memdumpToggleFailed', { msg: pluginErrorText(e) }))
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
    NotificationUtil.error(e instanceof Error ? e.message : t('settingsPlugin.plugin.readSettingsFailed'))
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
    NotificationUtil.error(e instanceof Error ? e.message : t('settingsPlugin.plugin.saveSettingsFailed'))
  } finally {
    settingsBusy.value = false
  }
}

async function loadStats(id: string) {
  const s = await refreshPluginLastStats(id)
  if (!s) NotificationUtil.info(t('settingsPlugin.plugin.noCallStats'))
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
  const parts: string[] = [`${s.func}()`, s.ok ? t('settingsPlugin.plugin.statsOk') : t('settingsPlugin.plugin.statsFail')]
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
    title: kind === 'zip' ? t('settingsPlugin.plugin.pickZip') : t('settingsPlugin.plugin.pickDir'),
    ...(kind === 'zip'
      ? { filters: [{ name: t('settingsPlugin.plugin.zipFilter'), extensions: ['zip'] }] }
      : {}),
  })
  // 用户取消
  if (!picked || typeof picked !== 'string') return
  pluginBusy.value = true
  try {
    const item = await installThirdPartyPlugin(picked)
    await syncCurrentWindowPluginListFromPreset()
    NotificationUtil.success(t('settingsPlugin.plugin.installSuccess', { name: item.name }))
  } catch (e) {
    NotificationUtil.error(t('settingsPlugin.plugin.installFailed', { msg: pluginErrorText(e) }))
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
    NotificationUtil.error(t('settingsPlugin.plugin.toggleEnabledFailed', { msg: pluginErrorText(e) }))
  } finally {
    pluginBusy.value = false
  }
}

// ==================== 页面能力收紧（只能减，不能加） ====================

/** 页面能力的中文名（拿不到就显示原始键，便于对新能力做兜底） */
function capLabel(cap: string): string {
  return PAGE_CAPABILITY_LABELS[cap] ?? cap
}

/** 真正生效的收紧项：settings.json 里可能存着插件压根没声明的能力（收紧它是空操作，不该计数） */
function deniedCapsOf(item: PluginListItem): string[] {
  return item.deniedPageCapabilities.filter(c => item.pageCapabilities.includes(c))
}

function capTitle(cap: string, item: PluginListItem): string {
  const on = !item.deniedPageCapabilities.includes(cap)
  return t('settingsPlugin.plugin.capTitleText', {
    cap,
    state: on ? t('settingsPlugin.plugin.capEnabledState') : t('settingsPlugin.plugin.capDisabledState'),
  })
}

/** 收紧 / 恢复单项页面能力 */
async function togglePluginCap(item: PluginListItem, cap: string) {
  pluginBusy.value = true
  try {
    const cur = item.deniedPageCapabilities
    const next = cur.includes(cap) ? cur.filter(c => c !== cap) : [...cur, cap]
    await setPluginPageCapsDenied(item.id, next)
  } catch (e) {
    NotificationUtil.error(t('settingsPlugin.plugin.saveCapFailed', { msg: pluginErrorText(e) }))
  } finally {
    pluginBusy.value = false
  }
}

/** 恢复插件自己声明的全部页面能力（清空收紧集合） */
async function restorePluginCaps(item: PluginListItem) {
  pluginBusy.value = true
  try {
    await setPluginPageCapsDenied(item.id, [])
  } catch (e) {
    NotificationUtil.error(t('settingsPlugin.plugin.restoreCapFailed', { msg: pluginErrorText(e) }))
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
    NotificationUtil.error(t('settingsPlugin.plugin.moveFailed', { msg: pluginErrorText(e) }))
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
    NotificationUtil.success(t('settingsPlugin.plugin.uninstallSuccess', { label: item.label }))
    showUninstallDialog.value = false
    pendingUninstall.value = null
  } catch (e) {
    NotificationUtil.error(t('settingsPlugin.plugin.uninstallFailed', { msg: pluginErrorText(e) }))
  } finally {
    pluginBusy.value = false
  }
}
</script>

<style lang="scss" scoped>
@use '@/styles/theme' as theme;

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

/* 页面能力：可点击收紧的 chip（原生 button —— 需要 hover / disabled / focus 的原生语义） */
.plugin-cap {
  padding: 0 6px;
  border-radius: 3px;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  cursor: pointer;
  border: 1px solid transparent;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    color: theme.getVar('textColor');
    border-color: theme.getVar('borderColor');
  }

  &:hover:not(:disabled) {
    @include theme.useTheme {
      background-color: theme.getVar('hoverColor');
    }
  }

  &:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  /* 被收紧：划掉 + 降饱和，让人一眼看出"这项被我关了" */
  &.is-denied {
    text-decoration: line-through;

    @include theme.useTheme {
      color: theme.getVar('textSecondary');
    }
  }
}

.plugin-caps-warn {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--font-size-sm);

  @include theme.useTheme {
    color: theme.getVar('dangerText');
  }
}

.plugin-caps-warn-text {
  flex: 1;
  min-width: 0;
}

// 依赖里「当前不可用」的那一项
.plugin-perm.is-missing {
  @include theme.useTheme {
    color: theme.getVar('dangerText');
  }
}

.plugin-deps-warn {
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('dangerText');
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
