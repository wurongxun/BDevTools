<template>
  <div class="topbar">
    <div class="brand">Redis 客户端<span class="ver">P2 体验</span></div>
    <ConnBar />
    <div class="conn-state" :class="{ online: store.connected }">
      {{ store.connected ? '已连接' : '未连接' }}
    </div>
  </div>

  <div class="layout">
    <aside class="sidebar"><DbSidebar /></aside>
    <main class="main">
      <div class="tabs">
        <button class="tab" :class="{ active: tab === 'browser' }" type="button" @click="tab = 'browser'">
          数据浏览
        </button>
        <button class="tab" :class="{ active: tab === 'terminal' }" type="button" @click="tab = 'terminal'">
          命令行
        </button>
        <button class="tab" :class="{ active: tab === 'monitor' }" type="button" @click="tab = 'monitor'">
          监控
        </button>
        <button class="tab" :class="{ active: tab === 'ops' }" type="button" @click="tab = 'ops'">
          运维
        </button>
      </div>

      <section class="pane" :class="{ hidden: tab !== 'browser' }">
        <KeyInfoBar />
        <div class="value-area">
          <ValueEditor v-if="store.selectedKey" />
          <div v-else class="placeholder">在左侧选择或搜索一个 key 查看其内容</div>
        </div>
      </section>

      <section class="pane" :class="{ hidden: tab !== 'terminal' }" id="pane-terminal">
        <Terminal />
      </section>

      <section class="pane" :class="{ hidden: tab !== 'monitor' }">
        <MonitorPanel />
      </section>

      <section class="pane" :class="{ hidden: tab !== 'ops' }">
        <OpsView />
      </section>
    </main>
  </div>

  <!-- 对话框宿主 -->
  <ConnDialog v-if="store.dialog?.type === 'conn'" @close="closeDialog" />
  <CompareDialog v-else-if="store.dialog?.type === 'compare'" @close="closeDialog" />
  <ImportExportDialog v-else-if="store.dialog?.type === 'importExport'" @close="closeDialog" />

  <div v-if="store.toast" class="toast" :class="store.toast.type">{{ store.toast.msg }}</div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { store, ensureInit, connectActive, closeDialog } from '../store'
import ConnBar from './ConnBar.vue'
import DbSidebar from './DbSidebar.vue'
import KeyInfoBar from './KeyInfoBar.vue'
import ValueEditor from './ValueEditor.vue'
import Terminal from './Terminal.vue'
import MonitorPanel from './MonitorPanel.vue'
import OpsView from './OpsView.vue'
import ConnDialog from './ConnDialog.vue'
import CompareDialog from './CompareDialog.vue'
import ImportExportDialog from './ImportExportDialog.vue'

const tab = ref('browser')

onMounted(async () => {
  ensureInit()
  if (store.activeConnId) {
    await connectActive()
  }
})
</script>
