<template>
  <!-- 通用 -->
  <section class="settings-group">
    <h3 class="settings-group-title">{{ $t('settingsPlugin.general.sectionGeneral') }}</h3>
    <div class="settings-group-body">
      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">{{ $t('autostart.name') }}</span>
          <span class="setting-description">{{ $t('autostart.hint') }}</span>
        </div>
        <div class="setting-control">
          <el-switch v-model="autostart" @change="autostartChange()" inline-prompt :active-icon="Check"
            :inactive-icon="Close" />
        </div>
      </div>

      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">{{ $t('darkMode.name') }}</span>
          <span class="setting-description">{{ $t('settingsPlugin.general.darkModeDesc') }}</span>
        </div>
        <div class="setting-control">
          <el-switch v-model="dark" inline-prompt :active-icon="Check" :inactive-icon="Close" />
        </div>
      </div>

      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">{{ $t('highContrast.name') }}</span>
          <span class="setting-description">{{ $t('settingsPlugin.general.highContrastDesc') }}</span>
        </div>
        <div class="setting-control">
          <el-switch v-model="highContrast" inline-prompt :active-icon="Check" :inactive-icon="Close" />
        </div>
      </div>

      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">{{ $t('locale.language') }}</span>
          <span class="setting-description">{{ $t('settingsPlugin.general.localeDesc') }}</span>
        </div>
        <div class="setting-control">
          <el-select v-model="language" @change="languageChange" placeholder="选择语言" style="width: 180px">
            <el-option v-for="item in LANGUAGE_OPTIONS" :key="item.value" :label="$t('settingsPlugin.languages.' + item.value)" :value="item.value"
              :disabled="item.disabled" />
          </el-select>
        </div>
      </div>
    </div>
  </section>

  <!-- 网络 -->
  <section class="settings-group">
    <h3 class="settings-group-title">{{ $t('settingsPlugin.general.sectionNetwork') }}</h3>
    <div class="settings-group-body">
      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">{{ $t('settingsPlugin.general.networkCheck') }}</span>
          <span class="setting-description">{{ $t('settingsPlugin.general.networkCheckDesc') }}</span>
        </div>
        <div class="setting-control">
          <app-button type="primary" disabled plain :title="$t('settingsPlugin.general.comingSoon')">
            {{ $t('settingsPlugin.general.networkCheckBtn') }}
          </app-button>
        </div>
      </div>
    </div>
  </section>

  <!-- 更新 -->
  <section class="settings-group">
    <h3 class="settings-group-title">{{ $t('settingsPlugin.general.sectionUpdate') }}</h3>
    <div class="settings-group-body">
      <div class="settings-item settings-item--update">
        <div class="setting-info">
          <span class="setting-label">
            {{ $t('settingsPlugin.general.appUpdate') }}
            <span class="update-current">当前 {{ current_version || '—' }}</span>
          </span>

          <span v-if="update_phase === 'downloading'" class="setting-description">
            {{ $t('settingsPlugin.general.updateDownloading', { version: update_info?.version }) }}
          </span>
          <span v-else-if="update_phase === 'installing'" class="setting-description">
            {{ $t('settingsPlugin.general.updateInstalling') }}
          </span>
          <span v-else-if="update_phase === 'error'" class="setting-description update-error">
            {{ update_error }}
          </span>
          <span v-else-if="has_update && update_info" class="setting-description">
            {{ $t('settingsPlugin.general.updateFoundPrefix') }} <b>{{ update_info.version }}</b>
            <template v-if="update_info.date"> · {{ $t('settingsPlugin.general.updateReleasedOn', { date: update_info.date }) }}</template>
          </span>
          <span v-else class="setting-description">{{ $t('settingsPlugin.general.updateDefaultDesc') }}</span>

          <!-- 下载进度：安装包有几十 MB，没有进度反馈会被当成卡死 -->
          <div v-if="update_busy" class="update-progress">
            <div class="update-progress-track">
              <i class="update-progress-fill" :style="{ width: update_percent + '%' }"></i>
            </div>
            <span class="update-progress-text">
              {{ update_percent }}%<template v-if="update_total">
                · {{ format_bytes(update_received) }} / {{ format_bytes(update_total) }}</template>
            </span>
          </div>

          <!-- 发布说明 -->
          <details v-if="update_info?.body && update_phase !== 'installing'" class="update-notes">
            <summary>{{ $t('settingsPlugin.general.updateNotes') }}</summary>
            <pre class="update-notes-body">{{ update_info.body }}</pre>
          </details>
        </div>

        <div class="setting-control">
          <app-button
            v-if="has_update"
            type="primary"
            :loading="update_busy"
            :disabled="update_busy"
            @click="installAppUpdate"
          >
            {{ update_phase === 'installing' ? $t('settingsPlugin.general.updating') : update_busy ? $t('settingsPlugin.general.downloading') : $t('settingsPlugin.general.updateNow') }}
          </app-button>
          <app-button
            v-else
            type="primary"
            plain
            :loading="update_phase === 'checking'"
            :disabled="update_phase === 'checking'"
            @click="checkAppUpdate()"
          >
            {{ $t('settingsPlugin.general.checkUpdate') }}
          </app-button>
        </div>
      </div>
    </div>
  </section>

  <!-- 其他 -->
  <section class="settings-group">
    <h3 class="settings-group-title">{{ $t('settingsPlugin.general.sectionOther') }}</h3>
    <div class="settings-group-body">
      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">{{ $t('settingsPlugin.general.readLinesLabel') }}</span>
          <span class="setting-description">{{ $t('settingsPlugin.general.readLinesDesc') }}</span>
        </div>
        <div class="setting-control">
          <div class="read-lines-control">
            <el-input
              v-model="read_lines"
              type="number"
              :min="1"
              :max="READ_LINES_MAX"
              :aria-label="$t('settingsPlugin.general.readLinesLabel')"
              :placeholder="$t('settingsPlugin.general.readLinesPlaceholder')"
              @keyup.enter="onReadLines"
              style="width: 180px"
            />
            <app-button type="primary" @click="onReadLines" plain>
              {{ $t('settingsPlugin.general.confirm') }}
            </app-button>
          </div>
        </div>
      </div>

      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">{{ $t('settingsPlugin.general.register') }}</span>
          <span class="setting-description">{{ $t('settingsPlugin.general.registerDesc') }}</span>
        </div>
        <div class="setting-control">
          <app-button type="primary" @click="handleRegisterAssociation" plain>
            {{ $t('settingsPlugin.general.register') }}
          </app-button>
        </div>
      </div>

      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">{{ $t('settingsPlugin.general.clearCache') }}</span>
          <span class="setting-description">{{ $t('settingsPlugin.general.clearCacheDesc') }}</span>
        </div>
        <div class="setting-control">
          <app-button type="danger" disabled plain :title="$t('settingsPlugin.general.comingSoon')">
            {{ $t('settingsPlugin.general.clearCache') }}
          </app-button>
        </div>
      </div>

      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">{{ $t('settingsPlugin.general.clearBrowserCache') }}</span>
          <span class="setting-description">{{ $t('settingsPlugin.general.clearBrowserCacheDesc') }}</span>
        </div>
        <div class="setting-control">
          <app-button type="danger" disabled plain :title="$t('settingsPlugin.general.comingSoon')">
            {{ $t('settingsPlugin.general.clearBrowserCache') }}
          </app-button>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, Close } from '@element-plus/icons-vue'
import { invoke } from '@/api/rpc'
import type { ResponseData } from '@/entity/entitys'
import useStore from '@/store'
import { autostartDisable, autostartEnable, checkUpdate, getVersion, publish } from '@/core/runtime'
import type { DownloadEvent, Store, Update } from '@/core/runtime'
import { isDark, toggleDark } from '@/composables/useDark'
import { isHighContrast, toggleHighContrast } from '@/composables/useHighContrast'
import NotificationUtil from '@/utils/NotificationUtil'
import { useSettingsStore } from './useSettingsStore'

const { appConfig: userStore } = useStore()

/** i18n：脚本里的提示文案走 t()，模板里走 $t() */
const { t } = useI18n()

/** 文本默认读取行数上限，与界面描述中的建议值保持一致 */
const READ_LINES_MAX = 200000

const LANGUAGE_OPTIONS = [
  { value: 'zh_CN', disabled: false },
  { value: 'zh_TW', disabled: false },
  { value: 'en', disabled: false },
]

const autostart = ref<boolean>(false)
const language = ref<string>('')
const read_lines = ref<number>(50000)

/**
 * 暗色模式开关：以全局 useDark 状态为唯一真相（可写 computed），
 * 避免设置页开关与顶栏主题按钮各持一份状态、切换后互不同步。
 */
const dark = computed<boolean>({
  get: () => isDark.value,
  set: (value: boolean) => {
    void applyDarkChange(value)
  },
})

/**
 * 高对比度开关：与上面的暗黑开关同构（以全局单例为唯一真相）。
 * 它是**正交**于明暗的第二个开关，两者可任意组合（浅/深 × 普通/高对比）。
 */
const highContrast = computed<boolean>({
  get: () => isHighContrast.value,
  set: (value: boolean) => {
    void applyHighContrastChange(value)
  },
})

/**
 * 应用更新状态机。
 *
 * 以前只有 `has_update` 一个布尔量，「检查」和「安装」被压在同一个按钮里：没有更新时按钮写
 * 「检查更新」，点下去却直接提示「已是最新」——因为它的实现第一行就是 `if (!has_update) return`，
 * 手动重新检查实际是失效的。拆成显式状态后两个动作各自独立。
 */
type UpdatePhase = 'idle' | 'checking' | 'ready' | 'downloading' | 'installing' | 'done' | 'error'

const has_update = ref<boolean>(false)
const update_info = ref<{ version?: string; date?: string; body?: string } | null>(null)
const current_version = ref<string>('')
const update_phase = ref<UpdatePhase>('idle')
const update_error = ref<string>('')
const update_received = ref<number>(0)
const update_total = ref<number>(0)
const update_percent = ref<number>(0)
/** Update 句柄：故意不放进 ref —— 插件侧的类实例包成响应式代理没有意义，还容易踩到方法 this 的坑 */
let pending_update: Update | null = null

const update_busy = computed(
  () => update_phase.value === 'downloading' || update_phase.value === 'installing',
)

/** settings.json 句柄（与「窗口设置」分区共用同一份，见 useSettingsStore） */
let store: Store | undefined = undefined

onMounted(async () => {
  // 检查更新：启动时静默进行——网络不通不该在用户进设置页时报错弹窗
  void refreshAppVersion()
  await checkAppUpdate(false)

  try {
    store = await useSettingsStore()
  } catch (e) {
    NotificationUtil.error(t('settingsPlugin.general.readConfigFailed', { msg: e instanceof Error ? e.message : String(e) }))
    return
  }

  const val_auto_start = await store.get<{ value: boolean }>('auto_start')
  if (val_auto_start !== undefined) {
    autostart.value = val_auto_start.value
    userStore.setAutoStart(val_auto_start.value)
  }
  const language_val = await store.get<{ value: string }>('language')
  if (language_val !== undefined) {
    language.value = language_val.value
    userStore.setLanguage(language_val.value)
  }
  const read_lines_val = await store.get<{ value: number }>('read_lines')
  if (read_lines_val !== undefined) {
    read_lines.value = Number(read_lines_val.value)
    userStore.setReadLines(Number(read_lines_val.value))
  }
  // update_reminder / theme 的读取已在 store/modules/appConfigInfo.ts 统一完成，此处不重复
  const dark_val = await store.get<{ value: boolean }>('dark')
  if (dark_val !== undefined) {
    userStore.setDark(dark_val.value)
    // 与顶栏保持一致：以持久化的值为准同步主题状态
    if (dark_val.value !== isDark.value) toggleDark()
  }
  // 启动时已由 initHighContrast() 读过，这里再对齐一次（与 dark 同节奏），
  // 防止 KV 值被别的窗口改过而本窗口的开关显示成旧值
  const hc_val = await store.get<{ value: boolean }>('high_contrast')
  if (hc_val !== undefined && hc_val.value !== isHighContrast.value) {
    toggleHighContrast()
  }
})

async function languageChange() {
  // setLanguage 内部同步切换 i18n；再广播给其他窗口
  userStore.setLanguage(language.value)
  if (store) {
    await store.set('language', { value: language.value })
    await store.save()
  }
  await publish('language-changed', { language: language.value })
}

async function autostartChange() {
  if (store) {
    await store.set('auto_start', { value: autostart.value })
    await store.save()
    if (autostart.value) {
      await autostartEnable()
    } else {
      await autostartDisable()
    }
  }
}

/** 应用暗色模式变更：持久化 + 切换主题 + 同步 store */
async function applyDarkChange(value: boolean) {
  if (store) {
    await store.set('dark', { value })
    await store.save()
  }
  if (value !== isDark.value) toggleDark()
  userStore.setDark(value)
}

/** 应用高对比度变更（键 `high_contrast`，与上面的 dark 同一套路） */
async function applyHighContrastChange(value: boolean) {
  if (store) {
    await store.set('high_contrast', { value })
    await store.save()
  }
  if (value !== isHighContrast.value) toggleHighContrast()
}

// 步骤1：注册应用到"打开方式"列表
async function handleRegisterAssociation() {
  const res = await invoke<ResponseData<unknown>>('register_file_association')
  if (res.code === 200) {
    // 步骤2：打开 Windows 默认应用设置页面
    await invoke('open_default_apps_settings')
  }
}

async function onReadLines() {
  // 校验：必须是正整数且不超过建议上限，避免脏值直接落盘
  const value = Number(read_lines.value)
  if (!Number.isInteger(value) || value <= 0) {
    NotificationUtil.warn(t('settingsPlugin.general.readLinesInvalid'))
    return
  }
  if (value > READ_LINES_MAX) {
    NotificationUtil.warn(t('settingsPlugin.general.readLinesTooLarge', { n: READ_LINES_MAX }))
    return
  }
  read_lines.value = value
  if (store) {
    await store.set('read_lines', { value })
    await store.save()
    userStore.setReadLines(value)
    NotificationUtil.success(t('settingsPlugin.general.readLinesSaved', { value }))
  }
}

// ==================== 应用更新 ====================

/** 读取当前应用版本，用于和更新清单里的版本对照展示 */
async function refreshAppVersion() {
  try {
    current_version.value = await getVersion()
  } catch {
    /* 非 Tauri 环境（纯前端预览）忽略 */
  }
}

/**
 * 检查更新。
 *
 * `manual` 区分「用户点的」和「启动时自动跑的」：只有前者会弹提示，
 * 后者失败时静默（否则离线启动会平白弹一个错误）。
 */
async function checkAppUpdate(manual = true) {
  if (update_busy.value || update_phase.value === 'checking') return
  update_phase.value = 'checking'
  update_error.value = ''
  pending_update = null
  has_update.value = false

  try {
    const result = await checkUpdate()
    if (result) {
      pending_update = result
      has_update.value = true
      update_info.value = {
        version: result.version || '未知版本',
        date: formatPubDate(result.date),
        body: (result.body || '').trim(),
      }
      update_phase.value = 'ready'
      if (manual) NotificationUtil.success(t('settingsPlugin.general.updateFoundToast', { version: update_info.value.version }))
    } else {
      update_info.value = null
      update_phase.value = 'idle'
      if (manual) NotificationUtil.info(t('settingsPlugin.general.upToDate'))
    }
  } catch (error) {
    update_phase.value = 'error'
    update_error.value = updateErrorText(error)
    console.error('Failed to check for updates:', error)
    if (manual) NotificationUtil.error(t('settingsPlugin.general.checkUpdateFailed', { msg: update_error.value }))
  }
}

/** 下载并安装更新；下载进度走 UI（22MB 的包没有反馈会被当成卡死） */
async function installAppUpdate() {
  if (update_busy.value) return

  // 句柄可能因窗口重载等原因丢了，重新取一次；仍然拿不到就明确报错，不要静默 return
  if (!pending_update) await checkAppUpdate(false)
  if (!pending_update) {
    update_phase.value = 'error'
    update_error.value = t('settingsPlugin.general.noUpdateInfo')
    return
  }

  const version = update_info.value?.version || ''
  update_phase.value = 'downloading'
  update_received.value = 0
  update_total.value = 0
  update_percent.value = 0
  update_error.value = ''

  try {
    await pending_update.downloadAndInstall((event: DownloadEvent) => {
      if (event.event === 'Started') {
        update_total.value = event.data.contentLength || 0
      } else if (event.event === 'Progress') {
        update_received.value += event.data.chunkLength || 0
        // 服务端没给 Content-Length 时退化成「不确定进度」，不假报 0%/100%
        update_percent.value = update_total.value
          ? Math.min(100, Math.round((update_received.value / update_total.value) * 100))
          : 0
      } else if (event.event === 'Finished') {
        update_percent.value = 100
        update_phase.value = 'installing'
      }
    })

    // Windows 上基本走不到这里：安装器进程起来后宿主就 std::process::exit(0) 了，
    // 由安装器自己重启应用（NSIS 收到 /R）。保留分支是为了非 Windows 平台与异常可控。
    update_phase.value = 'done'
    NotificationUtil.success(t('settingsPlugin.general.installedRestart', { version: version || '' }))
  } catch (error) {
    update_phase.value = 'error'
    update_error.value = updateErrorText(error)
    console.error('更新失败:', error)
    NotificationUtil.error(t('settingsPlugin.general.updateFailed', { msg: update_error.value }))
  }
}

/** 把插件抛出的原始错误转成能看懂的说明（签名失败 / 网络失败是最常见的两类） */
function updateErrorText(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error)
  if (/signature|verif/i.test(raw)) return `签名校验失败：${raw}`
  if (/network|fetch|dns|connect|timed? ?out/i.test(raw)) return `下载失败（网络问题）：${raw}`
  return raw
}

/** 清单里的 pub_date 是 ISO 串，展示到日即可 */
function formatPubDate(v?: string): string {
  if (!v) return ''
  const d = new Date(v)
  if (Number.isNaN(d.getTime())) return v
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

/** 字节数 → 人类可读（进度条右侧用） */
function format_bytes(n: number): string {
  if (!n || n < 0) return '—'
  const mb = n / 1024 / 1024
  return mb >= 1 ? `${mb.toFixed(1)} MB` : `${Math.round(n / 1024)} KB`
}
</script>

<style lang="scss" scoped>
@use '@/styles/theme' as theme;

/* 更新那一行左侧状态文字 / 版本号 / 错误态 */
.update-current {
  margin-left: 6px;
  font-size: var(--font-size-xs);
  font-weight: normal;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

/* 错误态必须显眼，且跟随主题。
   注意：本元素同时带 .setting-description，两条规则的特异性都是 html.<theme> .cls，
   所以 danger 色必须写在 useTheme 块**内**、并靠源码顺序取胜；
   写在块外会被 .setting-description 的 html 规则压掉（红色静默失效）。
   dangerText 而非 --el-color-danger：后者当正文级文字仅 2.9~3.7:1，不达 AA。 */
.update-error {
  @include theme.useTheme {
    color: theme.getVar('dangerText');
  }
}

.update-progress {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 6px;
  max-width: 420px;
}

.update-progress-track {
  flex: 1;
  height: 4px;
  border-radius: var(--radius-xs);
  overflow: hidden;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
  }
}

.update-progress-fill {
  display: block;
  height: 100%;
  background-color: var(--el-color-primary, #409eff);
  transition: width 0.2s ease;
}

.update-progress-text {
  flex-shrink: 0;
  font-size: var(--font-size-xs);
  font-variant-numeric: tabular-nums;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.update-notes {
  margin-top: 6px;
  max-width: 620px;
  font-size: var(--font-size-xs);

  summary {
    cursor: pointer;
    user-select: none;
    width: fit-content;

    @include theme.useTheme {
      color: theme.getVar('textSecondary');
    }
  }
}

.update-notes-body {
  margin: 6px 0 0;
  padding: 6px 8px;
  max-height: 132px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-family-base);
  line-height: 1.6;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
    background-color: theme.getVar('sidebarBg');
    border: 1px solid theme.getVar('borderColor');
    border-radius: var(--radius-md);
  }
}

.read-lines-control {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
