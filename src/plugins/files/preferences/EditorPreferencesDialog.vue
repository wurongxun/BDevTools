<template>
  <GlobalDialog v-model="visible" title="编辑器偏好" width="40%">
    <div class="pref-body">
      <p class="pref-hint">
        偏好保存在应用配置中，对所有文件编辑器立即生效。「纯查看模式」可一键回到 M1 基线。
      </p>

      <div class="pref-row">
        <span class="pref-label">自动换行</span>
        <div class="pref-control">
          <el-switch v-model="prefs.wordWrap" active-value="on" inactive-value="off" />
        </div>
      </div>
      <div class="pref-row">
        <span class="pref-label">文档地图（minimap）</span>
        <div class="pref-control">
          <el-switch v-model="prefs.minimap" />
        </div>
      </div>
      <div class="pref-row">
        <span class="pref-label">显示空白字符</span>
        <div class="pref-control">
          <el-select v-model="prefs.renderWhitespace" size="small">
            <el-option label="不显示" value="none" />
            <el-option label="仅选区" value="selection" />
            <el-option label="全部" value="all" />
          </el-select>
        </div>
      </div>
      <div class="pref-row">
        <span class="pref-label">缩进参考线</span>
        <div class="pref-control">
          <el-switch v-model="prefs.guidesIndentation" />
        </div>
      </div>
      <div class="pref-row">
        <span class="pref-label">括号对着色</span>
        <div class="pref-control">
          <el-switch v-model="prefs.bracketPairColorization" />
        </div>
      </div>
      <div class="pref-row">
        <span class="pref-label">Ctrl+滚轮缩放</span>
        <div class="pref-control">
          <el-switch v-model="prefs.mouseWheelZoom" />
        </div>
      </div>
      <div class="pref-row">
        <span class="pref-label">字号</span>
        <div class="pref-control">
          <el-input-number v-model="prefs.fontSize" :min="10" :max="28" size="small" />
        </div>
      </div>
      <div class="pref-row">
        <span class="pref-label">字体族</span>
        <div class="pref-control">
          <el-input v-model="prefs.fontFamily" size="small" placeholder="留空跟随默认等宽字体" clearable />
        </div>
      </div>
      <div class="pref-row">
        <span class="pref-label">行号</span>
        <div class="pref-control">
          <el-select v-model="prefs.lineNumbers" size="small">
            <el-option label="显示" value="on" />
            <el-option label="相对行号" value="relative" />
            <el-option label="隐藏" value="off" />
          </el-select>
        </div>
      </div>
      <div class="pref-row">
        <span class="pref-label">纯查看模式（回退）</span>
        <div class="pref-control">
          <el-switch v-model="prefs.pureViewMode" />
        </div>
      </div>

      <div class="pref-section-title">快捷键（留空表示不绑定）</div>
      <div v-for="item in KEY_LABELS" :key="item.id" class="pref-row">
        <span class="pref-label">{{ item.label }}</span>
        <div class="pref-control">
          <el-input
            :model-value="prefs.keybindings[item.id] ?? ''"
            size="small"
            placeholder="例如 Ctrl+Alt+B"
            @update:model-value="setKeybinding(item.id, $event)"
          />
        </div>
      </div>
    </div>
    <template #footer>
      <app-button size="small" @click="resetPreferences">恢复默认</app-button>
      <app-button size="small" type="primary" @click="visible = false">完成</app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import {
  editorPreferencesVisible,
  prefs,
  resetPreferences
} from './useEditorPreferences'

const visible = computed({
  get: () => editorPreferencesVisible.value,
  set: (v: boolean) => { editorPreferencesVisible.value = v }
})

const KEY_LABELS: { id: string; label: string }[] = [
  { id: 'files.view.toggleSplit', label: '切换分屏' },
  { id: 'files.bookmark.toggle', label: '切换行书签' },
  { id: 'files.bookmark.next', label: '下一个书签' },
  { id: 'files.bookmark.prev', label: '上一个书签' },
  { id: 'files.editor.zoomIn', label: '放大字号' },
  { id: 'files.editor.zoomOut', label: '缩小字号' }
]

function setKeybinding(id: string, value: string) {
  prefs.keybindings = { ...prefs.keybindings, [id]: String(value ?? '').trim() }
}
</script>

<style scoped lang="scss">
.pref-body {
  max-height: 60vh;
  overflow-y: hidden;
  padding-right: var(--space-2);
}

.pref-hint {
  margin: 0 0 var(--space-3) 0;
  font-size: var(--fontSizeSmall);
  color: var(--el-text-color-secondary);
  line-height: 1.6;
}

/* 每行自身是一个两列栅格（而不是 flex + space-between）：
   左列固定宽 → 所有控件的**左边界**对齐；右列吃掉剩余宽度 → **右边界**也对齐。
   此前控件各按自身默认宽度（el-select 写死 210 / el-input 撑满 / el-input-number 150），
   右侧参差、两个下拉框与相邻输入框宽度不一致，看起来像没对齐。 */
.pref-row {
  display: grid;
  grid-template-columns: 176px minmax(0, 1fr);
  align-items: center;
  column-gap: var(--space-3);
  padding: 6px 0;
  border-bottom: 1px solid var(--borderColor);
}

.pref-label {
  font-size: var(--fontSizeSmall);
  color: var(--el-text-color-primary);
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pref-control {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

/* 控件填满控件列：选择框与文本框同宽同高（都是 size="small" 的 24px） */
.pref-control :deep(.el-select),
.pref-control :deep(.el-input) {
  width: 100%;
}

/* 数字框按内容收窄（左右两个步进按钮 + 3 位数），但不参与拉伸，左边界随列对齐 */
.pref-control :deep(.el-input-number) {
  width: 112px;
}

.pref-section-title {
  margin: var(--space-4) 0 var(--space-1) 0;
  font-size: var(--fontSizeSmall);
  font-weight: 600;
  color: var(--el-text-color-primary);
}
</style>
