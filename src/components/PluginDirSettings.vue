<template>
  <div class="plugin-dir-settings">
    <div class="setting-item">
      <div class="setting-label">
        <span class="label-text">工作目录</span>
        <span class="label-desc">插件数据存储位置</span>
      </div>
      <div class="setting-control">
        <el-input
          v-model="customPath"
          size="small"
          placeholder="默认（留空使用默认路径）"
          :readonly="!editing"
        >
          <template #prepend>
            <el-tooltip :content="currentDir" placement="top" :disabled="!currentDir">
              <span class="current-path">{{ currentDir || '加载中...' }}</span>
            </el-tooltip>
          </template>
        </el-input>
        <el-button v-if="!editing" size="small" @click="startEdit">自定义</el-button>
        <template v-else>
          <el-button size="small" type="primary" @click="saveEdit">保存</el-button>
          <el-button size="small" @click="cancelEdit">取消</el-button>
        </template>
        <el-button size="small" @click="openInExplorer" :disabled="!currentDir" title="在文件管理器中打开">
          <el-icon><FolderOpened /></el-icon>
        </el-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { FolderOpened } from '@element-plus/icons-vue'
import { usePluginDir } from '@/composables/usePluginDir'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
  pluginId: string
}>()

const pluginDir = usePluginDir(props.pluginId)
const currentDir = ref('')
const customPath = ref('')
const editing = ref(false)

onMounted(async () => {
  await refreshDir()
})

async function refreshDir() {
  currentDir.value = await pluginDir.getDir()
  const custom = await pluginDir.getCustomDir()
  customPath.value = custom || ''
}

function startEdit() {
  editing.value = true
}

function cancelEdit() {
  editing.value = false
  // 恢复原值
  pluginDir.getCustomDir().then(custom => {
    customPath.value = custom || ''
  })
}

async function saveEdit() {
  editing.value = false
  const success = await pluginDir.setCustomDir(customPath.value)
  if (success) {
    await refreshDir()
  }
}

async function openInExplorer() {
  if (currentDir.value) {
    try {
      await invoke('file_manager_open_path', { path: currentDir.value })
    } catch (e) {
      console.error('打开目录失败:', e)
    }
  }
}
</script>

<style scoped>
.plugin-dir-settings {
  padding: 8px 0;
}
.setting-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.setting-label {
  display: flex;
  flex-direction: column;
}
.label-text {
  font-size: 13px;
  font-weight: 500;
  color: var(--el-text-color-primary);
}
.label-desc {
  font-size: 12px;
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
  font-size: 11px;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: inline-block;
}
</style>
