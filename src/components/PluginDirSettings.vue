<template>
  <div class="plugin-dir-settings">
    <div class="setting-item">
      <div class="setting-label">
        <span class="label-text">本地脚本目录</span>
        <span class="label-desc">应用默认存储位置，按工作区隔离</span>
      </div>
      <div class="setting-control">
        <span class="current-path" :title="defaultDir">{{ defaultDir || '加载中...' }}</span>
        <el-button size="small" @click="openInExplorer(defaultDir)" :disabled="!defaultDir" title="在文件管理器中打开">
          <el-icon><FolderOpened /></el-icon>
        </el-button>
      </div>
    </div>

    <div v-for="dir in externalDirs" :key="dir" class="setting-item">
      <div class="setting-label">
        <span class="label-text">{{ dirName(dir) }}</span>
        <span class="label-desc">外部脚本目录（跨工作区共享）</span>
      </div>
      <div class="setting-control">
        <span class="current-path" :title="dir">{{ dir }}</span>
        <el-button size="small" @click="openInExplorer(dir)" title="在文件管理器中打开">
          <el-icon><FolderOpened /></el-icon>
        </el-button>
        <el-button size="small" type="danger" plain @click="removeDir(dir)">移除</el-button>
      </div>
    </div>

    <el-button
      size="small"
      type="primary"
      plain
      class="add-btn"
      :loading="adding"
      @click="addDir"
    >
      <el-icon><FolderAdd /></el-icon>
      添加外部脚本目录
    </el-button>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { FolderOpened, FolderAdd } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { usePluginDir } from '@/composables/usePluginDir'
import { invoke } from '@/api/rpc'
import { open, emit, listen } from '@/core/runtime'

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
  unlistenDirChanged = await listen(PLUGIN_DIR_CHANGED_EVENT, () => {
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
      emit(PLUGIN_DIR_CHANGED_EVENT).catch(() => {})
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
    emit(PLUGIN_DIR_CHANGED_EVENT).catch(() => {})
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
  gap: 10px;
  padding: 8px 0;
}
.setting-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.setting-label {
  display: flex;
  flex-direction: column;
}
.label-text {
  font-size: var(--font-size-md);
  font-weight: 500;
  color: var(--el-text-color-primary);
}
.label-desc {
  font-size: var(--font-size-base);
  color: var(--el-text-color-secondary);
  margin-top: 2px;
}
.setting-control {
  display: flex;
  align-items: center;
  gap: 6px;
}
.setting-control .el-input {
  flex: 1;
}
.current-path {
  flex: 1;
  min-width: 0;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-regular);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  user-select: text;
}
.add-btn {
  align-self: flex-start;
}
</style>
