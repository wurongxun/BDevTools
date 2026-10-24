<template>
  <section class="settings-group">
    <h3 class="settings-group-title">{{ $t('settingsPlugin.window.sectionTitle') }}</h3>
    <p class="settings-group-desc">
      {{ $t('settingsPlugin.window.desc') }}
    </p>

    <!-- 窗口预设列表 -->
    <div class="window-preset-list">
      <div v-for="preset in allPresets" :key="preset.id" class="window-preset-card">
        <div class="preset-info">
          <div class="preset-name-row">
            <span class="preset-name">{{ preset.name }}</span>
            <el-tag v-if="preset.id === thisWindowPresetId" size="small" type="primary" effect="dark"
              :title="$t('settingsPlugin.window.tagCurrentTitle')">{{ $t('settingsPlugin.window.tagCurrent') }}</el-tag>
            <el-tag v-if="preset.id === 'main'" size="small" type="primary">{{ $t('settingsPlugin.window.tagMain') }}</el-tag>
            <el-tag v-if="preset.openOnStartup && preset.id !== 'main'" size="small" type="success">{{ $t('settingsPlugin.window.tagStartup') }}</el-tag>
            <el-tag v-if="preset.focusMode" size="small" type="warning">{{ $t('settingsPlugin.window.tagFocus') }}</el-tag>
            <el-tag v-if="presetOccupier(preset)" size="small" type="info"
              :title="presetLockHint(preset)">{{ $t('settingsPlugin.window.tagOccupied') }}</el-tag>
          </div>
          <div class="preset-meta">
            <span>{{ $t('settingsPlugin.window.metaPlugins') }}: {{ preset.enabledPlugins.length > 0 ? preset.enabledPlugins.join(', ') : $t('settingsPlugin.window.metaAll') }}</span>
            <span class="preset-sep">|</span>
            <span>{{ preset.width }} × {{ preset.height }}</span>
            <template v-if="preset.workspaceId">
              <span class="preset-sep">|</span>
              <span>{{ $t('settingsPlugin.window.metaWorkspace') }}: {{ presetWorkspaceName(preset) }}</span>
            </template>
          </div>
        </div>
        <div class="preset-actions">
          <!-- 正被别的窗口运行：编辑权在那边（见 useWindowPresetsStore 的「占用者」一节），
               这里只给一个「切过去」的动作，省得用户自己去窗口列表里找 -->
          <app-button v-if="presetOccupier(preset)" size="small" circle :icon="Switch" type="warning"
            :title="focusOccupierTitle(preset)" :aria-label="focusOccupierTitle(preset)"
            @click="focusOccupierWindow(preset)" />
          <!-- 启动时打开：绿=开 / 灰=关，直接落盘，免进弹窗 -->
          <app-button size="small" circle :icon="VideoPlay" :type="preset.openOnStartup ? 'success' : 'info'"
            :disabled="!!presetOccupier(preset) || isLastStartupPreset(preset)" :aria-pressed="preset.openOnStartup"
            :title="presetActionTitle(preset, startupToggleTitle(preset))"
            :aria-label="presetActionTitle(preset, startupToggleTitle(preset))"
            @click="toggleStartup(preset)" />
          <app-button size="small" @click="editWindowPreset(preset)" :icon="Edit" circle
            :disabled="!!presetOccupier(preset)" :title="presetActionTitle(preset, $t('settingsPlugin.window.editAction'))" />
          <app-button v-if="preset.id !== 'main'" size="small" @click="removeWindowPreset(preset.id)"
            :icon="Delete" circle type="danger" :disabled="!!presetOccupier(preset)"
            :title="presetActionTitle(preset, $t('settingsPlugin.window.deleteAction'))" />
        </div>
      </div>
    </div>

    <app-button type="primary" @click="addWindowPreset" plain>
      <el-icon>
        <Plus />
      </el-icon>
      {{ $t('settingsPlugin.window.addWindow') }}
    </app-button>
  </section>

  <!-- 窗口编辑弹窗 -->
  <GlobalDialog v-model="showWindowEditDialog" :title="$t(editingIsNew ? 'settingsPlugin.window.addTitle' : 'settingsPlugin.window.editTitle')" width="60%"
    @close="handleClose">
    <div v-if="editingPreset" class="window-edit-form">
      <!-- 窗口名称 -->
      <div class="form-item">
        <label class="form-label">{{ $t('settingsPlugin.window.labelName') }}</label>
        <el-input v-model="editingPreset.name" :placeholder="$t('settingsPlugin.window.namePlaceholder')" />
      </div>

      <!-- 绑定工作区（可选；主窗口与其它窗口一致：不绑定则窗口打开时弹出「打开时选择」） -->
      <div class="form-item">
        <label class="form-label">{{ $t('settingsPlugin.window.labelWorkspace') }}</label>
        <el-select v-model="editingPreset.workspaceId" :placeholder="$t('settingsPlugin.window.workspacePlaceholder')" clearable style="width: 100%">
          <el-option v-for="ws in workspaceOptions" :key="ws.id" :label="ws.name" :value="ws.id" />
        </el-select>
        <span class="form-hint">{{ $t('settingsPlugin.window.workspaceHint') }}</span>
      </div>

      <!-- 启用插件 -->
      <div class="form-item">
        <label class="form-label">{{ $t('settingsPlugin.window.labelPlugins') }}</label>
        <div class="plugin-checkbox-list">
          <el-checkbox v-for="plugin in registeredPlugins" :key="plugin.id"
            :model-value="editingPreset.enabledPlugins.includes(plugin.id)" @change="togglePlugin(plugin.id)">
            {{ plugin.label }}
          </el-checkbox>
        </div>
        <span class="form-hint">{{ pluginPickHint }}</span>
      </div>

      <!-- 窗口尺寸 -->
      <div class="form-item">
        <label class="form-label">{{ $t('settingsPlugin.window.labelSize') }}</label>
        <el-select v-model="selectedSizeValue" @change="onSizeChange" style="width: 100%">
          <el-option v-for="opt in SIZE_OPTIONS" :key="opt.value" :label="sizeLabel(opt.value)" :value="opt.value" />
        </el-select>
        <div v-if="isCustomSize" class="custom-size-inputs">
          <el-input-number v-model="editingPreset.width" :min="400" :max="3840" :placeholder="$t('settingsPlugin.window.sizeWidth')" />
          <span class="size-sep">×</span>
          <el-input-number v-model="editingPreset.height" :min="300" :max="2160" :placeholder="$t('settingsPlugin.window.sizeHeight')" />
        </div>
      </div>

      <!-- 启动时打开 -->
      <div class="form-item form-item-row">
        <div class="form-label-group">
          <label class="form-label">{{ $t('settingsPlugin.window.labelStartup') }}</label>
          <span class="form-hint">
            {{ isEditingMain
              ? $t('settingsPlugin.window.startupHintMain')
              : $t('settingsPlugin.window.startupHintOther') }}
          </span>
        </div>
        <el-switch v-model="editingPreset.openOnStartup" :disabled="!editingIsNew && isLastStartupPreset(editingPreset)"
          inline-prompt :active-icon="Check" :inactive-icon="Close" />
      </div>

      <!-- 专注模式 -->
      <div class="form-item form-item-row">
        <div class="form-label-group">
          <label class="form-label">{{ $t('settingsPlugin.window.labelFocusMode') }}</label>
          <span class="form-hint">{{ $t('settingsPlugin.window.focusHint') }}</span>
        </div>
        <el-switch :model-value="editingPreset.focusMode" @change="toggleFocusMode" inline-prompt :active-icon="Check"
          :inactive-icon="Close" />
      </div>
    </div>
    <template #footer>
      <app-button @click="handleClose">{{ $t('settingsPlugin.common.cancel') }}</app-button>
      <app-button type="primary" @click="saveEditingPreset">{{ $t('settingsPlugin.common.save') }}</app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, Close, Delete, Edit, Plus, Switch, VideoPlay } from '@element-plus/icons-vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { pluginManager } from '@/core/pluginManager'
import { subscribe } from '@/core/runtime'
import type { Unsubscribe } from '@/core/runtime'
import {
  WINDOW_PRESETS_CHANGED_EVENT,
  closePresetWindows,
  currentPresetId,
  currentWindowLabel,
  publishWindowPresetsChanged,
} from '@/core/windowPresets'
import type { WindowPresetsChangedPayload } from '@/core/windowPresets'
import { windowService } from '@/services/windowService'
import { workspaceService } from '@/services/workspaceService'
import useStore from '@/store'
import type { WorkspaceSummary } from '@/Types/workspace'
import NotificationUtil from '@/utils/NotificationUtil'
import {
  allPresets,
  allWindowPluginIds,
  coversAllWindowPlugins,
  ensureWindowPresetsLoaded,
  expandWindowPluginIds,
  loadWindowPresets,
  mainWindowPreset,
  normalizeWindowPluginIds,
  persistMainWindowPreset,
  presetOccupierElsewhere,
  refreshOpenedPresetWindows,
  saveWindowPresets,
  windowPresets,
} from './useWindowPresetsStore'
import type { PresetOccupier } from './useWindowPresetsStore'
import type { WindowPresetConfig } from './types'

/** i18n：模板走 $t()，脚本里的提示文案走 t() */
const { t } = useI18n()

/** 可选的窗口尺寸（value 是稳定键，label 走 i18n 展示） */
type SizeKey = 'small' | 'medium' | 'large' | 'xlarge' | 'custom'
const SIZE_OPTIONS: { value: SizeKey; width: number; height: number }[] = [
  { value: 'small', width: 800, height: 600 },
  { value: 'medium', width: 1024, height: 768 },
  { value: 'large', width: 1280, height: 900 },
  { value: 'xlarge', width: 1920, height: 1080 },
  { value: 'custom', width: 0, height: 0 },
]

/** 尺寸键 → 展示文案（随语言切换） */
function sizeLabel(key: SizeKey): string {
  return t('settingsPlugin.window.size' + key[0].toUpperCase() + key.slice(1))
}

/** 设置页的共享预设状态与落盘动作都在 `useWindowPresetsStore`（两个分区共写一份配置） */
const showWindowEditDialog = ref(false)
const editingPreset = ref<WindowPresetConfig | null>(null)
const isCreatingPreset = ref(false)

/** 本窗口所属的窗口预设 ID（'main' = 主窗口；null = 辅助窗口 / 不属于任何预设） */
const thisWindowPresetId = ref<string | null>(null)
/** 本窗口 label：用来认出「广播是不是自己发的」，避免自己刷自己 */
const thisWindowLabel = currentWindowLabel()
/** 编辑弹窗开着时收到别的窗口的变更：先挂起，弹窗关闭后再重读（见 onExternalPresetsChanged） */
const pendingExternalReload = ref(false)
let unlistenPresetsChanged: Unsubscribe | null = null

/** 主窗口预设的默认值（真实值由 `useWindowPresetsStore` 从磁盘载入） */
const selectedSizeValue = ref<SizeKey>('medium')

/** 参与窗口预设勾选的插件（固定在活动栏底部的宿主入口如「设置」不参与预设清单） */
const registeredPlugins = computed(() => pluginManager.plugins.filter(p => !p.pinnedBottom))

/**
 * 清单里已勾选、但**当前没有勾选框**的插件（被禁用即从 `pluginManager` 注销）。
 *
 * 它们不是「没加入窗口」，只是眼下画不出勾选框；保存时**原样保留**，所以要在提示里说明，
 * 否则用户会以为它们被悄悄删掉了（与本次修的「空清单不勾选」同属一类「勾选框对不上」）。
 */
const presetOfflinePlugins = computed(() => {
  const known = new Set(registeredPlugins.value.map(p => p.id))
  return (editingPreset.value?.enabledPlugins ?? []).filter(id => !known.has(id))
})

/** 勾选情况说明：勾满 / 未勾满各自意味着什么，以及有没有上面那类看不见的条目 */
const pluginPickHint = computed(() => {
  const editing = editingPreset.value
  if (!editing) return ''
  const parts: string[] = []
  if (editing.focusMode) {
    parts.push(t('settingsPlugin.window.focusPickHint'))
  } else if (coversAllWindowPlugins(editing.enabledPlugins)) {
    parts.push(t('settingsPlugin.window.allPickedHint'))
  } else {
    parts.push(t('settingsPlugin.window.notPickedHint'))
  }
  const offline = presetOfflinePlugins.value
  if (offline.length) {
    parts.push(t('settingsPlugin.window.offlineHint', { list: offline.join('、') }))
  }
  return parts.join('；')
})

/** 可绑定的工作区列表（排除临时工作区） */
const workspaceOptions = ref<WorkspaceSummary[]>([])

/** 当前编辑的预设是否为新建 */
const editingIsNew = computed(() => isCreatingPreset.value)

/** 当前编辑的是否为主窗口预设 */
const isEditingMain = computed(() => editingPreset.value?.id === 'main')

/** 当前尺寸是否为自定义 */
const isCustomSize = computed(() => {
  if (!editingPreset.value) return false
  return !SIZE_OPTIONS.some(
    s => s.width === editingPreset.value!.width && s.height === editingPreset.value!.height && s.width > 0,
  )
})

/**
 * 「启动时打开」的预设数量 —— **主窗口也算一个**。
 *
 * 应用至少要有一个窗口在启动时打开，否则启动后一个窗口都不出现（进程却还在跑）。
 * 早先这里只数 `window_presets`、把主窗口排除在外，于是「主窗口开着」时也禁止关掉最后
 * 一个其它窗口 —— 属于过严；真正的不变量是「主窗口 + 其它窗口合计 ≥ 1」，
 * 与 Rust 启动分支（分别看 `main_window_preset.openOnStartup` 与 `window_presets[*]`）一致。
 */
const startupCount = computed(
  () =>
    (mainWindowPreset.value.openOnStartup ? 1 : 0) +
    windowPresets.value.filter(p => p.openOnStartup).length,
)

/**
 * 该预设是否为「最后一个启动时打开的窗口」——是则不允许关掉（主窗口同样适用）。
 *
 * 卡片上的快速开关与编辑弹窗里的 `el-switch` **必须共用这一条判定**：两处各写一遍
 * 必然分叉（弹窗里那份还要额外叠 `!editingIsNew`，因为新建预设尚未计入 startupCount）。
 */
function isLastStartupPreset(preset: WindowPresetConfig | null | undefined): boolean {
  if (!preset) return false
  return preset.openOnStartup && startupCount.value <= 1
}

/** 快速开关的提示文案（同时用作 title / aria-label / 禁用原因） */
function startupToggleTitle(preset: WindowPresetConfig): string {
  if (isLastStartupPreset(preset)) return t('settingsPlugin.window.startupLastOne')
  return preset.openOnStartup ? t('settingsPlugin.window.startupOn') : t('settingsPlugin.window.startupOff')
}

/**
 * 该预设此刻是否正被**别的窗口**运行（非 null ⇒ 本窗口不给编辑）。
 *
 * 编辑权归运行它的那个窗口：在别处编辑时，写回会拿本窗口（编辑者）的运行期副本当基准，
 * 把占用者刚做的改动静默顶掉 —— 「加入 / 移出」写的正是这条路径。占用状态来自窗口注册表
 * （见 `useWindowPresetsStore` 的「占用者」一节），本窗口自己占用不算。
 */
function presetOccupier(preset: WindowPresetConfig): PresetOccupier | null {
  return presetOccupierElsewhere(preset.id)
}

/** 被占用时的统一说明（卡片 tag 与三个按钮的 title 共用，避免各处说法不一） */
function presetLockHint(preset: WindowPresetConfig): string {
  const occupier = presetOccupier(preset)
  if (!occupier) return ''
  return t('settingsPlugin.window.occupiedHint', { title: occupier.title })
}

/** 按钮的禁用原因：被占用时给占用说明，否则给这条操作本来的说明 */
function presetActionTitle(preset: WindowPresetConfig, normal: string): string {
  return presetLockHint(preset) || normal
}

/** 「切到正在运行该预设的窗口」按钮的说明 */
function focusOccupierTitle(preset: WindowPresetConfig): string {
  const occupier = presetOccupier(preset)
  if (!occupier) return ''
  return t('settingsPlugin.window.focusOccupierTitle', { title: occupier.title })
}

/** 切到正在运行该预设的窗口（编辑权在那边，用户不必自己去窗口列表里找） */
async function focusOccupierWindow(preset: WindowPresetConfig) {
  const occupier = presetOccupier(preset)
  if (!occupier) return
  try {
    if (await windowService.focusWindow(occupier.label)) return
    NotificationUtil.error(t('settingsPlugin.window.focusFailed', { title: occupier.title }))
  } catch (e) {
    NotificationUtil.error(t('settingsPlugin.window.focusFailedSimple', { title: occupier.title }))
    console.warn('聚焦窗口失败:', e)
  }
  // 失败多半是那个窗口已经关了：顺手刷新一次占用状态，锁要跟着解除
  void refreshOpenedPresetWindows()
}

onMounted(async () => {
  thisWindowPresetId.value = await currentPresetId()
  try {
    await ensureWindowPresetsLoaded()
  } catch (e) {
    NotificationUtil.error(t('settingsPlugin.window.readFailed', { msg: e instanceof Error ? e.message : String(e) }))
    return
  }
  // 占用状态随时可能变（设置页在各窗口里常驻，上次打开之后窗口开开关关过）：
  // 每次进来重读一次，否则会出现「窗口早关了，预设却还锁着」
  await refreshOpenedPresetWindows()
  await loadWorkspaceOptions()
  // 别的窗口改了窗口预设：本窗口要跟着刷新，否则「当前」标记与「至少保留一个」的判定
  // 都是按旧数据算的（窗口菜单栏有自己的一份监听，见 MenuBar）
  try {
    unlistenPresetsChanged = await subscribe<WindowPresetsChangedPayload>(
      WINDOW_PRESETS_CHANGED_EVENT,
      payload => void onExternalPresetsChanged(payload),
    )
  } catch (e) {
    console.warn('订阅窗口预设变更失败，列表不会跟随其他窗口刷新:', e)
  }
})

onBeforeUnmount(() => {
  unlistenPresetsChanged?.()
  unlistenPresetsChanged = null
})

/** 加载工作区列表（供预设绑定选择） */
async function loadWorkspaceOptions() {
  try {
    const list = await workspaceService.list()
    const detail = await Promise.all(list.map(ws => workspaceService.get(ws.id).catch(() => null)))
    workspaceOptions.value = list.filter((_, i) => detail[i]?.settings?.isTemporary !== true)
  } catch (e) {
    console.warn('加载工作区列表失败:', e)
  }
}

/** 预设绑定的工作区名称（卡片展示用） */
function presetWorkspaceName(preset: WindowPresetConfig): string {
  if (!preset.workspaceId) return ''
  return workspaceOptions.value.find(ws => ws.id === preset.workspaceId)?.name ?? t('settingsPlugin.window.workspaceDeleted')
}

/**
 * 预设变更后广播：其他窗口的菜单栏与设置页据此刷新（见 core/windowPresets.ts）。
 *
 * 注意这里**不关窗口**：删预设要关哪些窗口由 `removeWindowPreset` 统一判断并发起
 * （`closePresetWindows`），放在这里会让每个窗口都去关同一个目标。
 */
async function notifyPresetsChanged(payload: Omit<WindowPresetsChangedPayload, 'from'> = {}) {
  try {
    await publishWindowPresetsChanged(payload)
  } catch (e) {
    console.warn('广播窗口预设变更失败:', e)
  }
}

/**
 * 别的窗口改了窗口预设 ⇒ 重读一次列表。
 *
 * 编辑弹窗开着时**不能直接覆盖** `windowPresets`：弹窗里那份是本地副本，覆盖后用户
 * 一点保存，就会把对方刚做的改动整片写回去（丢改动且无提示），所以先挂起、关窗后再刷。
 */
async function onExternalPresetsChanged(payload: WindowPresetsChangedPayload) {
  // 自己发的（同一窗口的另一个分区，如「插件管理」的加入 / 移出）：弹窗没开时不必重读
  // （共享状态已经是最新的），但弹窗开着就必须走下面的挂起逻辑 —— 弹窗里那份是打开时的
  // 副本，不重读的话一点保存就会把对方刚做的改动整片写回去。
  const fromSelf = payload.from !== '' && payload.from === thisWindowLabel
  if (fromSelf && !showWindowEditDialog.value) return
  if (showWindowEditDialog.value) {
    if (!pendingExternalReload.value) {
      NotificationUtil.info(t('settingsPlugin.window.externalChanged'))
    }
    pendingExternalReload.value = true
    return
  }
  await refreshPresetsFromDisk()
}

/** 重读预设列表（外部变更后同步用） */
async function refreshPresetsFromDisk() {
  pendingExternalReload.value = false
  try {
    await loadWindowPresets()
  } catch (e) {
    console.warn('刷新窗口预设失败:', e)
  }
}

/** 弹窗关闭后补一次挂起的刷新 */
function flushPendingReload() {
  if (pendingExternalReload.value) void refreshPresetsFromDisk()
}

/** 添加新窗口预设 */
function addWindowPreset() {
  isCreatingPreset.value = true
  editingPreset.value = {
    id: 'window-' + Date.now(),
    name: t('settingsPlugin.window.newWindowName'),
    enabledPlugins: allWindowPluginIds(),
    width: 1024,
    height: 768,
    openOnStartup: false,
    focusMode: false,
    workspaceId: '',
  }
  selectedSizeValue.value = 'medium'
  showWindowEditDialog.value = true
}

/**
 * 编辑窗口预设。
 *
 * 必须把清单**展开**再交给弹窗：预设里的空清单语义是「显示全部」（与活动栏、
 * `layout.filteredPlugins`、Rust 的 `update_window_presets` 一致），而勾选框是按
 * `includes(id)` 画的 —— 直接给空数组就会画成「一个都没勾」，与用户看到的活动栏
 * （全都在）相反，保存时还会被「至少选择一个插件」挡回去。
 */
function editWindowPreset(preset: WindowPresetConfig) {
  // 双保险：按钮已按占用状态禁用，但占用状态是异步刷新的，点下去那一刻可能刚好变了；
  // 而且这里顺带把用户送到能编辑它的地方（比单纯报错有用）
  const occupier = presetOccupier(preset)
  if (occupier) {
    NotificationUtil.info(t('settingsPlugin.window.focusSwitched', { title: occupier.title }))
    void focusOccupierWindow(preset)
    return
  }
  isCreatingPreset.value = false
  editingPreset.value = { ...preset, enabledPlugins: expandWindowPluginIds(preset.enabledPlugins) }
  const matched = SIZE_OPTIONS.find(s => s.width === preset.width && s.height === preset.height && s.width > 0)
  selectedSizeValue.value = matched?.value ?? 'custom'
  showWindowEditDialog.value = true
}

/**
 * 删除窗口预设。
 *
 * 该预设打开的**其它**窗口要一并关闭（多窗口应用里它们已经没有对应的预设了），
 * 但如果删的正是**本窗口自己**的预设，关掉自己会让用户正在操作的界面突然消失 ⇒ 只提示。
 * 关闭走 `close_window` ⇒ 目标窗口的 `onCloseRequested`，照常保存会话与未保存文档。
 */
async function removeWindowPreset(id: string) {
  if (id === 'main') return // 主窗口不可删除
  const preset = windowPresets.value.find(p => p.id === id)
  if (isLastStartupPreset(preset)) {
    NotificationUtil.error(t('settingsPlugin.window.keepOneStartup'))
    return
  }
  // 被别的窗口运行时不给删：那个窗口正用着这份配置，删除还会把它的窗口一并关掉
  const occupier = preset ? presetOccupier(preset) : null
  if (occupier) {
    NotificationUtil.info(t('settingsPlugin.window.deleteOccupied', { title: occupier.title }))
    return
  }
  windowPresets.value = windowPresets.value.filter(p => p.id !== id)
  await saveWindowPresets()
  // 先广播再关窗：关窗要等各窗口的保存流程（临时工作区还会弹确认），别拖住列表刷新
  await notifyPresetsChanged({ removed: [id] })
  const closed = await closePresetWindows(id, thisWindowLabel)
  if (thisWindowPresetId.value === id) {
    NotificationUtil.warn(t('settingsPlugin.window.deletedSelf', { name: preset?.name ?? id }))
  } else if (closed > 0) {
    NotificationUtil.success(t('settingsPlugin.window.closedWindows', { n: closed }))
  }
}

/**
 * 若改的正是**本窗口所属**的预览，把新清单直接应用到本窗口的活动栏。
 *
 * 预设的 `enabledPlugins` 与本窗口运行期的显示清单（`layoutStore.enabledPlugins`）是同一份
 * 数据的两个落点：「插件管理」里的加入 / 移出会同时改两边，这里必须对称 —— 否则在弹窗里
 * 改完清单后本窗口的活动栏还是旧的，而下一步的加入 / 移出又会以旧清单为基准写回预设，
 * 把这次改动整片覆盖掉。
 */
function syncCurrentWindowPluginList(preset: WindowPresetConfig) {
  if (preset.id !== thisWindowPresetId.value) return
  useStore().useLayoutStore.setWindowPluginList(preset.enabledPlugins)
}

/**
 * 卡片上的「启动时打开」快速开关：立即落盘并广播，免进编辑弹窗。
 *
 * 禁用态理论上点不到，这里仍挡一次是为了防御——快速开关绕过弹窗保存流程，
 * 一旦漏判就会把「至少保留一个启动时打开的窗口」这条不变量打破。
 */
async function toggleStartup(preset: WindowPresetConfig) {
  if (isLastStartupPreset(preset)) {
    NotificationUtil.error(t('settingsPlugin.window.keepOneStartup'))
    return
  }
  // 与编辑同一把锁：预设归运行它的窗口管，开关也只在那边改（按钮已禁用，这里兜底）
  const occupier = presetOccupier(preset)
  if (occupier) {
    NotificationUtil.info(t('settingsPlugin.window.startupOccupied', { title: occupier.title }))
    return
  }
  const next = !preset.openOnStartup
  if (preset.id === 'main') {
    await persistMainWindowPreset({ ...preset, openOnStartup: next })
  } else {
    const idx = windowPresets.value.findIndex(p => p.id === preset.id)
    if (idx === -1) return
    windowPresets.value[idx] = { ...windowPresets.value[idx], openOnStartup: next }
    await saveWindowPresets()
  }
  await notifyPresetsChanged()
}

/** 保存编辑的窗口预设 */
async function saveEditingPreset() {
  const editing = editingPreset.value
  if (!editing) return
  if (!editing.name.trim()) {
    NotificationUtil.error(t('settingsPlugin.window.nameRequired'))
    return
  }
  if (editing.focusMode && editing.enabledPlugins.length !== 1) {
    NotificationUtil.error(t('settingsPlugin.window.focusOnePlugin'))
    return
  }
  // 空清单的语义是「显示全部」，表达不了「一个都不要」⇒ 不能当成合法配置存下去。
  // 弹窗打开时清单已按「全部」展开，所以只有「把所有勾都取消」才会走到这里；
  // 一个插件都没注册（全被禁用）时不做这个拦截，否则会卡死在保存不了。
  if (editing.enabledPlugins.length === 0 && registeredPlugins.value.length > 0) {
    NotificationUtil.error(t('settingsPlugin.window.keepOnePlugin'))
    return
  }
  // 检查窗口名称是否重复（跨主窗口和其他预设）
  const currentName = editing.name.trim()
  const nameConflict = allPresets.value.some(p => p.id !== editing.id && p.name.trim() === currentName)
  if (nameConflict) {
    NotificationUtil.error(t('settingsPlugin.window.nameDuplicate'))
    return
  }

  /**
   * 写盘用的副本：勾满时归一化回空清单（= 显示全部），让以后新装的插件还能自动出现
   * （与「插件管理」的加入 / 移出同一口径）。专注模式跳过归一化 —— 它要求清单里恰好
   * 一个 id，换成空清单会被下游当成「全部」，单一语义就丢了。
   */
  const normalized: WindowPresetConfig = {
    ...editing,
    enabledPlugins: editing.focusMode
      ? [...editing.enabledPlugins]
      : normalizeWindowPluginIds(editing.enabledPlugins),
  }

  // 主窗口预设：单独存储（主窗口现在也可绑定工作区，绑定会一并落盘）
  if (normalized.id === 'main') {
    await persistMainWindowPreset(normalized)
    syncCurrentWindowPluginList(normalized)
    showWindowEditDialog.value = false
    editingPreset.value = null
    await notifyPresetsChanged()
    flushPendingReload()
    return
  }

  if (isCreatingPreset.value) {
    windowPresets.value.push({ ...normalized })
  } else {
    const idx = windowPresets.value.findIndex(p => p.id === normalized.id)
    if (idx !== -1) {
      windowPresets.value[idx] = { ...normalized }
    }
  }
  await saveWindowPresets()
  syncCurrentWindowPluginList(normalized)
  showWindowEditDialog.value = false
  editingPreset.value = null
  // 带上名字：如果只是改名，由该预设打开的窗口要同步自己的标题（见 core/windowPresets.ts）
  await notifyPresetsChanged({ renamed: { id: normalized.id, name: normalized.name.trim() } })
  flushPendingReload()
}

/** 尺寸选择变化 */
function onSizeChange(value: SizeKey) {
  const opt = SIZE_OPTIONS.find(s => s.value === value)
  if (opt && opt.width > 0 && editingPreset.value) {
    editingPreset.value.width = opt.width
    editingPreset.value.height = opt.height
  }
}

/**
 * 切换插件选中状态（专注模式下为单选：点击即替换）。
 *
 * 取消勾选前必须挡住「最后一个」：清单空了以后语义变成「显示全部」，等于所有插件又全
 * 回来了（既不直观，保存时也会被校验拦下）—— 一次点击什么都不发生、也没提示，属于哑操作。
 */
function togglePlugin(pluginId: string) {
  const editing = editingPreset.value
  if (!editing) return
  const idx = editing.enabledPlugins.indexOf(pluginId)
  if (idx === -1) {
    if (editing.focusMode) {
      // 专注模式：单选替换，只保留当前点击的插件
      editing.enabledPlugins = [pluginId]
    } else {
      editing.enabledPlugins.push(pluginId)
    }
    return
  }
  // 已选中的插件：专注模式下不允许取消（恰好保留一个）
  if (editing.focusMode) {
    NotificationUtil.info(t('settingsPlugin.window.focusKeepOneHint'))
    return
  }
  if (editing.enabledPlugins.length <= 1) {
    NotificationUtil.info(t('settingsPlugin.window.keepOnePluginHint'))
    return
  }
  editing.enabledPlugins.splice(idx, 1)
}

/** 切换专注模式 */
function toggleFocusMode(val: boolean) {
  const editing = editingPreset.value
  if (!editing) return
  editing.focusMode = val
  if (val && editing.enabledPlugins.length > 1) {
    // 专注模式开启时只保留第一个插件
    editing.enabledPlugins = [editing.enabledPlugins[0]]
  }
}

function handleClose() {
  showWindowEditDialog.value = false
  flushPendingReload()
}
</script>

<style lang="scss" scoped>
@use '@/styles/theme' as theme;

// ---- 窗口预设列表 ----
.window-preset-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}

.window-preset-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border-radius: var(--radius-md);
  transition: background-color 0.15s;

  @include theme.useTheme {
    border: 1px solid theme.getVar('borderColor');
    background-color: theme.getVar('appMainBgColor');
  }

  &:hover {
    @include theme.useTheme {
      background-color: theme.getVar('activityBarHover');
    }
  }
}

.preset-info {
  flex: 1;
  min-width: 0;
}

.preset-name-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 3px;
}

.preset-name {
  font-size: var(--font-size-md);
  font-weight: 500;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.preset-meta {
  font-size: var(--font-size-base);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.preset-sep {
  margin: 0 6px;
  opacity: 0.4;
}

.preset-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
  margin-left: 12px;

  // 禁用态（只剩最后一个启动项）：用整颗按钮降透明度表达「不可点」，
  // 并把实心色兜回原色 —— 全局 .el-button.is-disabled 只改 color/border 不改背景，
  // 结果会是「浅绿底 + 灰图标」，图标与底色几乎同亮度、看不见。
  :deep(.el-button.is-disabled) {
    opacity: 0.45;
    cursor: not-allowed;
  }

  :deep(.el-button--success.is-disabled) {
    color: var(--el-color-white);
    background-color: var(--el-color-success);
    border-color: var(--el-color-success);
  }
}

// ---- 编辑弹窗表单 ----
.window-edit-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.form-item {
  display: flex;
  flex-direction: column;
  gap: 4px;

  &.form-item-row {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }
}

.form-label {
  font-size: var(--font-size-md);
  font-weight: 500;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.form-label-group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.form-hint {
  font-size: var(--font-size-base);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.plugin-checkbox-list {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.custom-size-inputs {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 6px;

  .el-input-number {
    flex: 1;
  }
}

.size-sep {
  font-size: var(--font-size-md);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>
