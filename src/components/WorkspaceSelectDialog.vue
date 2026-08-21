<!-- src/components/WorkspaceSelectDialog.vue -->
<!-- 工作区选择弹窗：新建编辑器窗口时选择要绑定的工作区，或新建临时工作区 -->
<template>
  <GlobalDialog
    v-model="visible"
    title="选择工作区"
    width="440px"
    height="420px"
  >
    <div class="ws-select">
      <div class="ws-select-tip">为新窗口选择一个工作区，或新建一个临时工作区</div>
      <div class="ws-select-list">
        <div v-if="loading" class="ws-select-empty">加载中...</div>
        <div v-else-if="items.length === 0" class="ws-select-empty">暂无可用工作区</div>
        <div
          v-for="item in items"
          :key="item.id"
          class="ws-select-item"
          :class="{ 'is-locked': item.locked }"
          :title="item.locked ? `已被窗口 [${item.holder}] 占用` : ''"
          @click="handleSelect(item)"
        >
          <div class="ws-item-name">
            {{ item.name }}
            <span v-if="item.isTemp" class="ws-item-badge ws-item-badge-temp">临时</span>
            <span v-if="item.locked" class="ws-item-badge">已占用</span>
          </div>
          <div class="ws-item-folders">{{ item.folders.join(', ') || '无关联文件夹' }}</div>
        </div>
      </div>
    </div>
    <template #footer>
      <el-button size="small" type="primary" @click="handleCreateTemp">+ 新建工作区（临时）</el-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import GlobalDialog from './GlobalDialog.vue'
import { workspaceService } from '@/services/workspaceService'
import type { WorkspaceSummary } from '@/Types/workspace'

interface Props {
  modelValue: boolean
}

const props = defineProps<Props>()

interface Emits {
  (e: 'update:modelValue', value: boolean): void
  /** 选中一个已有工作区 */
  (e: 'select', workspaceId: string): void
  /** 请求新建临时工作区 */
  (e: 'create-temp'): void
}

const emit = defineEmits<Emits>()

interface WorkspaceItem extends WorkspaceSummary {
  locked: boolean
  holder?: string
  /** 是否为临时工作区（settings.isTemporary） */
  isTemp: boolean
}

const visible = ref<boolean>(props.modelValue)
const loading = ref(false)
const items = ref<WorkspaceItem[]>([])

watch(() => props.modelValue, (val: boolean) => {
  visible.value = val
  if (val) loadList()
})

watch(visible, (val: boolean) => {
  emit('update:modelValue', val)
})

/** 加载工作区列表，并逐项查询锁占用状态与临时标记 */
async function loadList() {
  loading.value = true
  try {
    const list = await workspaceService.list()
    items.value = await Promise.all(
      list.map(async (ws) => {
        const [holder, config] = await Promise.all([
          workspaceService.lockStatus(ws.id).catch(() => null),
          workspaceService.get(ws.id).catch(() => null)
        ])
        return {
          ...ws,
          locked: !!holder,
          holder: holder ?? undefined,
          isTemp: config?.settings?.isTemporary === true
        }
      })
    )
  } catch (e) {
    console.warn('[WorkspaceSelectDialog] 加载工作区列表失败:', e)
    items.value = []
  } finally {
    loading.value = false
  }
}

function handleSelect(item: WorkspaceItem) {
  if (item.locked) return
  visible.value = false
  emit('select', item.id)
}

function handleCreateTemp() {
  visible.value = false
  emit('create-temp')
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.ws-select-tip {
  margin-bottom: 10px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.ws-select-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.ws-select-empty {
  text-align: center;
  padding: 32px 0;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.ws-select-item {
  padding: 8px 12px;
  border-radius: 4px;
  cursor: pointer;
  transition: background-color 0.15s;

  @include theme.useTheme {
    border: 1px solid theme.getVar('borderColor');

    &:hover:not(.is-locked) {
      background: theme.getVar('hoverColor');
    }
  }

  &.is-locked {
    cursor: not-allowed;
    opacity: 0.55;
  }
}

.ws-item-name {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 500;
  margin-bottom: 3px;

  @include theme.useTheme {
    font-size: theme.getVar('fontSizeSmall');
    color: theme.getVar('textColor');
  }
}

.ws-item-badge {
  font-size: 10px;
  padding: 0 6px;
  border-radius: 3px;

  // 反色徽标，两种主题下均保证对比度
  @include theme.useTheme {
    background: theme.getVar('textSecondary');
    color: theme.getVar('bgColor');
  }
}

.ws-item-badge-temp {
  background: var(--el-color-warning) !important;
  color: #fff !important;
}

.ws-item-folders {
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>
