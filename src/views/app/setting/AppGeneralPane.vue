<template>
  <!-- 通用 -->
  <section class="settings-group">
    <h3 class="settings-group-title">通用</h3>
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
          <span class="setting-description">切换界面深浅色（与顶栏主题按钮同步）</span>
        </div>
        <div class="setting-control">
          <el-switch v-model="dark" inline-prompt :active-icon="Check" :inactive-icon="Close" />
        </div>
      </div>

      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">{{ $t('locale.language') }}</span>
          <span class="setting-description">选择界面显示语言</span>
        </div>
        <div class="setting-control">
          <el-select v-model="language" @change="languageChange" placeholder="选择语言" style="width: 180px">
            <el-option v-for="item in LANGUAGE_OPTIONS" :key="item.value" :label="item.label" :value="item.value"
              :disabled="item.disabled" />
          </el-select>
        </div>
      </div>
    </div>
  </section>

  <!-- 网络 -->
  <section class="settings-group">
    <h3 class="settings-group-title">网络</h3>
    <div class="settings-group-body">
      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">网络检测</span>
          <span class="setting-description">检测当前网络连接状态（功能开发中，暂不可用）</span>
        </div>
        <div class="setting-control">
          <app-button type="primary" disabled plain title="功能开发中，暂不可用">检测网络</app-button>
        </div>
      </div>
    </div>
  </section>

  <!-- 更新 -->
  <section class="settings-group">
    <h3 class="settings-group-title">更新</h3>
    <div class="settings-group-body">
      <div class="settings-item settings-item--update">
        <div class="setting-info">
          <span class="setting-label">
            应用更新
            <span class="update-current">当前 {{ current_version || '—' }}</span>
          </span>

          <span v-if="update_phase === 'downloading'" class="setting-description">
            正在下载 {{ update_info?.version }}…
          </span>
          <span v-else-if="update_phase === 'installing'" class="setting-description">
            安装包已下载，正在启动安装程序，完成后应用会自动重启
          </span>
          <span v-else-if="update_phase === 'error'" class="setting-description update-error">
            {{ update_error }}
          </span>
          <span v-else-if="has_update && update_info" class="setting-description">
            发现新版本 <b>{{ update_info.version }}</b>
            <template v-if="update_info.date"> · 发布于 {{ update_info.date }}</template>
          </span>
          <span v-else class="setting-description">检查并安装新版本，获取最新功能和修复</span>

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
            <summary>更新内容</summary>
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
            {{ update_phase === 'installing' ? '安装中' : update_busy ? '下载中' : '立即更新' }}
          </app-button>
          <app-button
            v-else
            type="primary"
            plain
            :loading="update_phase === 'checking'"
            :disabled="update_phase === 'checking'"
            @click="checkAppUpdate()"
          >
            检查更新
          </app-button>
        </div>
      </div>
    </div>
  </section>

  <!-- 其他 -->
  <section class="settings-group">
    <h3 class="settings-group-title">其他设置</h3>
    <div class="settings-group-body">
      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">文本默认读取行数</span>
          <span class="setting-description">编辑器打开文件时默认读取的行数，建议最大值不超过20万行，行数直接影响内存占用，请谨慎设置</span>
        </div>
        <div class="setting-control">
          <div class="read-lines-control">
            <el-input
              v-model="read_lines"
              type="number"
              :min="1"
              :max="READ_LINES_MAX"
              aria-label="文本默认读取行数"
              placeholder="默认读取行数"
              @keyup.enter="onReadLines"
              style="width: 180px"
            />
            <app-button type="primary" @click="onReadLines" plain>
              确认
            </app-button>
          </div>
        </div>
      </div>

      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">注册</span>
          <span class="setting-description">注册应用到 windows「打开方式」列表</span>
        </div>
        <div class="setting-control">
          <app-button type="primary" @click="handleRegisterAssociation" plain>
            注册
          </app-button>
        </div>
      </div>

      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">清除缓存</span>
          <span class="setting-description">清理应用缓存文件</span>
        </div>
        <div class="setting-control">
          <app-button type="danger" disabled plain title="功能开发中，暂不可用">清除缓存</app-button>
        </div>
      </div>

      <div class="settings-item">
        <div class="setting-info">
          <span class="setting-label">清除浏览器缓存</span>
          <span class="setting-description">清理内置浏览器缓存数据</span>
        </div>
        <div class="setting-control">
          <app-button type="danger" disabled plain title="功能开发中，暂不可用">清除浏览器缓存</app-button>
        </div>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Check, Close } from '@element-plus/icons-vue'
import { invoke } from '@/api/rpc'
import type { ResponseData } from '@/entity/entitys'
import useStore from '@/store/index'
import { autostartDisable, autostartEnable, checkUpdate, getVersion, publish } from '@/core/runtime'
import type { DownloadEvent, Store, Update } from '@/core/runtime'
import { isDark, toggleDark } from '@/composables/useDark'
import NotificationUtil from '@/utils/NotificationUtil'
import { useSettingsStore } from './useSettingsStore'

const { appConfig: userStore } = useStore()

/** 文本默认读取行数上限，与界面描述中的建议值保持一致 */
const READ_LINES_MAX = 200000

const LANGUAGE_OPTIONS = [
  { value: 'zh_CN', label: '简体中文', disabled: false },
  { value: 'zh_TW', label: '繁体中文（中国台湾）', disabled: false },
  { value: 'zh_HK', label: '繁体中文（中国香港）', disabled: false },
  { value: 'en', label: 'English', disabled: false },
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
    NotificationUtil.error(`读取应用配置失败：${e instanceof Error ? e.message : String(e)}`)
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
    NotificationUtil.warn('请输入大于 0 的整数行数')
    return
  }
  if (value > READ_LINES_MAX) {
    NotificationUtil.warn(`读取行数建议不超过 ${READ_LINES_MAX} 行，请重新设置`)
    return
  }
  read_lines.value = value
  if (store) {
    await store.set('read_lines', { value })
    await store.save()
    userStore.setReadLines(value)
    NotificationUtil.success(`已保存，默认读取 ${value} 行`)
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
      if (manual) NotificationUtil.success(`发现新版本 ${update_info.value.version}`)
    } else {
      update_info.value = null
      update_phase.value = 'idle'
      if (manual) NotificationUtil.info('当前已是最新版本')
    }
  } catch (error) {
    update_phase.value = 'error'
    update_error.value = updateErrorText(error)
    console.error('Failed to check for updates:', error)
    if (manual) NotificationUtil.error(`检查更新失败：${update_error.value}`)
  }
}

/** 下载并安装更新；下载进度走 UI（22MB 的包没有反馈会被当成卡死） */
async function installAppUpdate() {
  if (update_busy.value) return

  // 句柄可能因窗口重载等原因丢了，重新取一次；仍然拿不到就明确报错，不要静默 return
  if (!pending_update) await checkAppUpdate(false)
  if (!pending_update) {
    update_phase.value = 'error'
    update_error.value = '拿不到更新包信息，请重新检查更新'
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
    NotificationUtil.success(`已安装 ${version || '新版本'}，应用即将重启`)
  } catch (error) {
    update_phase.value = 'error'
    update_error.value = updateErrorText(error)
    console.error('更新失败:', error)
    NotificationUtil.error(`更新失败：${update_error.value}`)
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
@use '@/styles/theme.scss' as theme;

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
