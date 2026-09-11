<template>
  <el-drawer v-model="visible" title="日志高亮规则" size="480px" :close-on-click-modal="true">
    <div class="rule-manager">
      <p class="rule-tip">
        规则按从上到下顺序匹配，重叠时靠前者优先。背景建议使用半透明色，明暗主题下都不会遮挡文字。
      </p>

      <!-- 规则列表 -->
      <div v-if="store.rules.length" class="rule-list">
        <div
          v-for="rule in store.rules"
          :key="rule.id"
          class="rule-item"
          :class="{ 'is-disabled': !rule.enabled }"
        >
          <el-switch
            v-model="rule.enabled"
            size="small"
            @change="(v: boolean | string | number) => store.updateRule(rule.id, { enabled: !!v })"
          />
          <span class="rule-swatch" :style="{ background: rule.color }"></span>
          <div class="rule-main">
            <div class="rule-name">
              {{ rule.name }}
              <span class="rule-flag">{{ rule.useRegex ? '正则' : '文本' }}</span>
              <span v-if="rule.ignoreCase" class="rule-flag">忽略大小写</span>
            </div>
            <div class="rule-pattern">{{ rule.pattern }}</div>
          </div>
          <div class="rule-ops">
            <app-button size="small" text title="上移" @click="store.moveRule(rule.id, -1)">
              <el-icon><ArrowUp /></el-icon>
            </app-button>
            <app-button size="small" text title="下移" @click="store.moveRule(rule.id, 1)">
              <el-icon><ArrowDown /></el-icon>
            </app-button>
            <app-button size="small" text title="编辑" @click="startEdit(rule)">
              <el-icon><Edit /></el-icon>
            </app-button>
            <app-button size="small" text type="danger" title="删除" @click="store.removeRule(rule.id)">
              <el-icon><Delete /></el-icon>
            </app-button>
          </div>
        </div>
      </div>
      <p v-else class="rule-empty">暂无规则，新增一条以高亮特定关键词或异常。</p>

      <!-- 新增 / 编辑表单 -->
      <div class="rule-form">
        <div class="form-title">{{ editingId ? '编辑规则' : '新增规则' }}</div>
        <el-form label-width="72px" size="small">
          <el-form-item label="名称">
            <el-input v-model="form.name" placeholder="如：空指针异常" />
          </el-form-item>
          <el-form-item label="匹配内容">
            <el-input v-model="form.pattern" :placeholder="form.useRegex ? '如 \\bTimeout\\b' : '如 NullPointer'" />
          </el-form-item>
          <el-form-item label="选项">
            <el-checkbox v-model="form.useRegex">正则</el-checkbox>
            <el-checkbox v-model="form.ignoreCase">忽略大小写</el-checkbox>
          </el-form-item>
          <el-form-item label="背景色">
            <el-color-picker v-model="form.color" show-alpha />
            <span class="form-hint">半透明背景在明暗主题下都可读</span>
          </el-form-item>
          <el-form-item label="预览">
            <span class="rule-preview" :style="{ background: form.color }">示例文本 Sample</span>
          </el-form-item>
        </el-form>
        <div class="form-ops">
          <app-button v-if="editingId" size="small" @click="cancelEdit">取消</app-button>
          <app-button size="small" type="primary" :disabled="!form.pattern" @click="submit">
            {{ editingId ? '保存修改' : '新增规则' }}
          </app-button>
        </div>
      </div>
    </div>
  </el-drawer>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ArrowDown, ArrowUp, Delete, Edit } from '@element-plus/icons-vue'
import { useLogStore } from '../store/logs'
import type { HighlightRule } from '../types'

const props = defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{ (e: 'update:modelValue', v: boolean): void }>()

const store = useLogStore()
const visible = computed({
  get: () => props.modelValue,
  set: v => emit('update:modelValue', v),
})

const editingId = ref('')
const form = reactive({
  name: '',
  pattern: '',
  useRegex: true,
  ignoreCase: false,
  color: 'rgba(147, 112, 219, 0.38)',
})

function resetForm() {
  editingId.value = ''
  form.name = ''
  form.pattern = ''
  form.useRegex = true
  form.ignoreCase = false
  form.color = 'rgba(147, 112, 219, 0.38)'
}

function startEdit(rule: HighlightRule) {
  editingId.value = rule.id
  form.name = rule.name
  form.pattern = rule.pattern
  form.useRegex = rule.useRegex
  form.ignoreCase = rule.ignoreCase
  form.color = rule.color
}

function cancelEdit() {
  resetForm()
}

function submit() {
  const payload = {
    name: form.name.trim() || form.pattern,
    pattern: form.pattern,
    useRegex: form.useRegex,
    ignoreCase: form.ignoreCase,
    color: form.color,
    enabled: true,
  }
  if (editingId.value) {
    store.updateRule(editingId.value, payload)
  } else {
    store.addRule(payload)
  }
  resetForm()
}

watch(visible, v => {
  if (v) resetForm()
})
</script>

<style scoped lang="scss">
.rule-manager {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.rule-tip {
  margin: 0;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
  line-height: 1.6;
}

.rule-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rule-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: var(--radius-md);

  &.is-disabled {
    opacity: 0.55;
  }
}

.rule-swatch {
  flex: none;
  width: 18px;
  height: 18px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--el-border-color);
}

.rule-main {
  flex: 1;
  min-width: 0;
}

.rule-name {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--font-size-base);
  color: var(--el-text-color-primary);
}

.rule-flag {
  padding: 0 4px;
  border-radius: var(--radius-xs);
  font-size: var(--font-size-xs);
  color: var(--el-text-color-secondary);
  background: var(--el-fill-color-light);
}

.rule-pattern {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rule-ops {
  flex: none;
  display: flex;
  gap: 2px;
}

.rule-empty {
  margin: 0;
  padding: 12px 0;
  text-align: center;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-placeholder);
}

.rule-form {
  padding-top: 8px;
  border-top: 1px solid var(--el-border-color-lighter);
}

.form-title {
  margin-bottom: 8px;
  font-size: var(--font-size-base);
  font-weight: 500;
  color: var(--el-text-color-primary);
}

.form-hint {
  margin-left: 8px;
  font-size: var(--font-size-xs);
  color: var(--el-text-color-placeholder);
}

.rule-preview {
  display: inline-block;
  padding: 1px 6px;
  border-radius: var(--radius-xs);
  font-family: var(--font-family-mono);
}

.form-ops {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
