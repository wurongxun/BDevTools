<template>
  <div class="plugin-dir-settings">
    <div class="setting-item">
      <div class="setting-label" title="本地脚本目录（应用默认存储位置，按工作区隔离）">本地脚本目录</div>
      <div class="setting-control">
        <span class="current-path" :title="defaultDir">{{ defaultDir || '加载中...' }}</span>
        <button
          class="icon-btn"
          :disabled="!defaultDir"
          title="在文件管理器中打开"
          @click="openInExplorer(defaultDir)"
        >
          <el-icon><FolderOpened /></el-icon>
        </button>
      </div>
    </div>

    <div v-for="dir in externalDirs" :key="dir" class="setting-item">
      <div class="setting-label" :title="`${dir}（外部脚本目录，跨工作区共享）`">{{ dirName(dir) }}</div>
      <div class="setting-control">
        <span class="current-path" :title="dir">{{ dir }}</span>
        <button class="icon-btn" title="在文件管理器中打开" @click="openInExplorer(dir)">
          <el-icon><FolderOpened /></el-icon>
        </button>
        <button class="icon-btn danger" title="移除挂载（目录内容不会被删除）" @click="removeDir(dir)">
          <el-icon><Close /></el-icon>
        </button>
      </div>
    </div>

    <app-button
      size="small"
      type="primary"
      plain
      class="add-btn"
      :loading="adding"
      @click="addDir"
    >
      <el-icon><FolderAdd /></el-icon>
      <span>添加外部脚本目录</span>
    </app-button>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { FolderOpened, FolderAdd, Close } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { usePluginDir } from '@/composables/usePluginDir'
import { invoke } from '@/api/rpc'
import { open, publish, subscribe } from '@/core/runtime'

/** 外部脚本目录变更事件（跨窗口同步面板展示与脚本树刷新） */
const PLUGIN_DIR_CHANGED_EVENT = 'plugin-dir-changed'

const props = defineProps<{
  pluginId: string
}>()

const pluginDir = usePluginDir(props.pluginId)
const defaultDir = ref('')
const externalDirs = ref<string[]>([])
const adding = ref(false)
let unlistenDirChanged: (() => void) | null = null

/** 取路径最后一段作为显示名 */
function dirName(path: string): string {
  const n = path.replace(/\\/g, '/').replace(/\/+$/, '')
  return n.split('/').filter(Boolean).pop() || n
}

onMounted(async () => {
  await refreshDirs()
  // 其他窗口修改了外部目录时同步刷新展示（存储已共享，此处仅刷新 UI）
  unlistenDirChanged = await subscribe(PLUGIN_DIR_CHANGED_EVENT, () => {
    refreshDirs()
  })
})

onUnmounted(() => {
  unlistenDirChanged?.()
  unlistenDirChanged = null
})

// 窗口切换工作区时刷新默认目录（外部目录用户级不变，一并刷新）
watch(pluginDir.workspaceId, () => {
  refreshDirs()
})

async function refreshDirs() {
  defaultDir.value = await pluginDir.getDir()
  externalDirs.value = await pluginDir.listExternalDirs()
}

async function addDir() {
  const selected = await open({ directory: true, multiple: false })
  if (typeof selected !== 'string' || !selected) return
  adding.value = true
  try {
    const res = await pluginDir.addExternalDir(selected)
    if (res.ok) {
      ElMessage.success('已添加')
      await refreshDirs()
      publish(PLUGIN_DIR_CHANGED_EVENT).catch(() => {})
    } else {
      ElMessage.error(res.msg || '添加失败')
    }
  } finally {
    adding.value = false
  }
}

async function removeDir(path: string) {
  const ok = await pluginDir.removeExternalDir(path)
  if (ok) {
    ElMessage.success('已移除挂载（目录内容未删除）')
    await refreshDirs()
    publish(PLUGIN_DIR_CHANGED_EVENT).catch(() => {})
  } else {
    ElMessage.error('移除失败')
  }
}

async function openInExplorer(path: string) {
  if (!path) return
  try {
    await invoke('file_manager_open_path', { path })
  } catch (e) {
    console.error('打开目录失败:', e)
  }
}
</script>

<style scoped>
.plugin-dir-settings {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 4px 0;
}
/* 每行：左标签 + 右控制（路径 + 图标按钮），单行紧凑布局 */
.setting-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
}
.setting-label {
  flex-shrink: 0;
  width: 88px;
  font-size: var(--font-size-md);
  font-weight: 500;
  color: var(--el-text-color-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.setting-control {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 2px;
}
/* 路径：等宽字体 + 省略号（与项目「标识符文本用 --font-family-mono」约定一致） */
.current-path {
  flex: 1;
  min-width: 0;
  font-size: var(--font-size-sm);
  font-family: var(--font-family-mono);
  color: var(--el-text-color-regular);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  user-select: text;
}
/* 与 SidebarPanel/ScriptExplorer 的 .header-btn 同款 22×22 原生按钮 */
.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: none;
  background: transparent;
  border-radius: var(--radius-md);
  cursor: pointer;
  color: var(--el-text-color-secondary);
  font-size: var(--font-size-lg);
  font-family: inherit;
  transition: all 0.15s;
  flex-shrink: 0;
}
.icon-btn:hover:not(:disabled) {
  color: var(--el-text-color-primary);
  background-color: var(--el-fill-color-light);
}
.icon-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
.icon-btn.danger:hover:not(:disabled) {
  color: var(--stateErrorFg);
  background-color: var(--el-color-danger-light-9);
}
.add-btn {
  align-self: flex-start;
  margin-top: 4px;
}
</style>
