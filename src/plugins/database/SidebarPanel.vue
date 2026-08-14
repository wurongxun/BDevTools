<template>
  <div class="db-sidebar">
    <div class="sidebar-header">
      <span class="title">数据库连接</span>
      <el-button :icon="Plus" size="small" circle @click="showConnectionDialog = true" />
      <el-button :icon="Refresh" size="small" circle @click="refreshConnections" />
    </div>
    <DatabaseTree ref="treeRef" />
    <ConnectionDialog
      v-model="showConnectionDialog"
      :edit-config="editingConfig"
      @submit="handleConnectionSubmit"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Plus, Refresh } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useConnectionStore } from './store'
import DatabaseTree from './components/DatabaseTree.vue'
import ConnectionDialog from './components/ConnectionDialog.vue'
import type { DbConnectionConfig } from './types'

const connectionStore = useConnectionStore()
const showConnectionDialog = ref(false)
const editingConfig = ref<DbConnectionConfig | null>(null)
const treeRef = ref<InstanceType<typeof DatabaseTree>>()

onMounted(() => {
  connectionStore.loadConnections()
})

function refreshConnections() {
  connectionStore.loadConnections()
  treeRef.value?.refreshTree()
}

async function handleConnectionSubmit(config: DbConnectionConfig) {
  try {
    if (config.id) {
      const res = await connectionStore.updateConnection(config)
      if (res.code === 200) {
        ElMessage.success('连接配置已更新')
      } else {
        ElMessage.error(res.msg)
      }
    } else {
      const res = await connectionStore.createConnection(config)
      if (res.code === 200) {
        ElMessage.success('连接配置已创建')
      } else {
        ElMessage.error(res.msg)
      }
    }
    showConnectionDialog.value = false
    treeRef.value?.refreshTree()
  } catch (e: any) {
    ElMessage.error('操作失败: ' + (e.message || e))
  }
}
</script>

<style scoped>
.db-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--el-bg-color);
}
.sidebar-header {
  display: flex;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  gap: 8px;
}
.sidebar-header .title {
  font-size: 13px;
  font-weight: 600;
  flex: 1;
}
</style>
