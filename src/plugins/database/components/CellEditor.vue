<template>
  <!--
    为什么是「全局唯一一个」编辑器：
    早期版本是**每个单元格一个编辑器组件**。宽表动辄上万个格子，那就是上万个组件实例、
    上万个响应式 effect —— 光建实例就吃掉大半时间，而这些格子 99.9% 根本没人点。
    现在单元格只渲染纯文本，谁被点就由这一个浮层接管（记下行 key + 列索引）。

    顺带这也是上虚拟滚动的前提：虚拟滚动会回收滚出视口的行，
    「编辑器挂在单元格里」的话，正在编辑的那一行滚出去就会被卸载、编辑态凭空消失。

    为什么 Teleport 到 body：el-table 的 `.cell` 是 overflow:hidden，
    输入框塞在单元格里会被列宽直接裁掉（60px 的 int 列只剩几十像素，正在改的值根本看不全）。
  -->
  <Teleport to="body">
    <div
      ref="layerRef"
      class="ce-layer"
      :style="layerStyle"
      @keydown.enter.stop.prevent="commit"
      @keydown.esc.stop.prevent="cancel"
    >
      <div class="ce-main">
        <!-- 布尔：三态（可空列才有 NULL） -->
        <el-select v-if="kind === 'bool'" v-model="boolValue" size="small" class="ce-control" @change="commit">
          <el-option label="TRUE" :value="'TRUE'" />
          <el-option label="FALSE" :value="'FALSE'" />
          <el-option v-if="nullable" label="NULL" :value="'NULL'" />
        </el-select>

        <!-- 日期 / 时间：写回的一律是字符串（value-format 保证），清空即 NULL -->
        <el-date-picker
          v-else-if="kind === 'date'"
          v-model="dateValue"
          type="date"
          size="small"
          class="ce-control"
          value-format="YYYY-MM-DD"
          @change="commit"
        />
        <el-date-picker
          v-else-if="kind === 'datetime'"
          v-model="dateValue"
          type="datetime"
          size="small"
          class="ce-control"
          value-format="YYYY-MM-DD HH:mm:ss"
          @change="commit"
        />
        <el-time-picker
          v-else-if="kind === 'time'"
          v-model="dateValue"
          size="small"
          class="ce-control"
          value-format="HH:mm:ss"
          @change="commit"
        />

        <!-- 长文本：副弹层里编辑，不挤浮层 -->
        <div v-else-if="kind === 'longtext'" class="ce-control">
          <el-input v-model="textBuffer" size="small" readonly @click="longtextOpen = true" />
          <el-popover v-model:visible="longtextOpen" :width="420" trigger="manual" placement="bottom-start">
            <template #reference>
              <span class="ce-longtext-anchor" />
            </template>
            <div class="ce-pop">
              <el-input v-model="textBuffer" type="textarea" :rows="6" size="small" @keydown.esc.stop="cancel" />
              <div class="ce-pop-actions">
                <el-checkbox v-if="nullable" v-model="nullFlag">NULL</el-checkbox>
                <span class="spacer" />
                <el-button size="small" text @click="cancel">取消</el-button>
                <el-button size="small" type="primary" @click="closePopoverAndCommit">确定</el-button>
              </div>
            </div>
          </el-popover>
        </div>

        <!-- 其余：普通文本输入（数值 / DECIMAL 也走它 ——
             刻意不转成 Number：把 '12.30' 解析成数字会丢掉标度，也会把 '007' 变成 7） -->
        <el-input
          v-else
          ref="textRef"
          v-model="textBuffer"
          size="small"
          class="ce-control"
          :class="{ 'is-invalid': !!localError }"
          :placeholder="placeholder()"
          :disabled="nullFlag"
          @blur="onFocusOut"
        />

        <!-- NULL 开关只在「清空输入框」语义不唯一的那些类型上出现：
             布尔 / 时间控件本身就有「清空 = NULL」的含义，不需要重复入口。
             mousedown .preventDefault：点它不该把焦点从输入框抢走（否则会触发提交）。 -->
        <el-checkbox
          v-if="showNullToggle"
          v-model="nullFlag"
          size="small"
          class="ce-null"
          title="写入 NULL"
          @mousedown.prevent
          @change="onNullToggle"
        >
          NULL
        </el-checkbox>
      </div>

      <div v-if="localError" class="ce-error">⚠ {{ localError }}</div>
      <div v-else class="ce-hint">
        {{ hintText() }}<span v-if="columnTitle" class="ce-col"> · {{ columnTitle }}</span>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import type { ColumnInfo } from '../types'
import { SQL_NULL, isSqlNull, normalizeCellInput, type ValueKind } from '../utils/editing'

/**
 * 单元格编辑器（**全局单例浮层**）。
 *
 * 由父组件用 `v-if` + `key` 挂起/销毁：每次点开一个单元格就换一次 key，
 * 状态天然干净，不需要在内部维护「打开 / 关闭」两套分支。
 *
 * - Enter 提交 / Esc 取消 / 焦点离开浮层提交
 * - **提交的不是值而是「意图」**：NULL 勾选 ⇒ 提交 `SQL_NULL` 标记，
 *   这样上层才能区分「改成 NULL」与「改成空串」（两者在服务端的语义完全不同）
 * - 数字 / 布尔 / 时间 / JSON 列**清空 ⇒ 写 NULL**（它们的空串不是合法值，
 *   MySQL 会报 `Incorrect integer value: ''`）；NOT NULL 时直接拦住并说明原因
 */
const props = defineProps<{
  /** 当前值（已合并未保存的改动） */
  value: unknown
  /** 被点的单元格节点：用它定位，滚动时跟着它走 */
  anchor: HTMLElement | null
  column?: ColumnInfo | undefined
  kind: ValueKind
  /** 列名，显示在浮层提示里（改的是哪一列，别靠猜） */
  columnTitle?: string
}>()

const emit = defineEmits<{
  /** 提交（值可能是 SQL_NULL 标记） */
  commit: [value: unknown]
  cancel: []
}>()

const textBuffer = ref('')
const nullFlag = ref(false)
const localError = ref<string | null>(null)
const textRef = ref<any>(null)
const layerRef = ref<HTMLElement | null>(null)
const longtextOpen = ref(false)
const boolValue = ref('TRUE')
const dateValue = ref<string | null>(null)

const nullable = computed(() => props.column?.is_nullable ?? false)
/** 布尔 / 时间控件的「清空」就是 NULL，再给一个复选框是重复入口 */
const showNullToggle = computed(() => nullable.value && (props.kind === 'text' || props.kind === 'number'))

function placeholder(): string {
  if (props.kind === 'number') {
    return nullable.value ? '数值；留空 = 写 NULL' : '数值（该列不允许 NULL）'
  }
  if (props.kind === 'longtext' || props.kind === 'text') return '文本；留空 = 空字符串'
  return ''
}

function hintText(): string {
  if (props.kind === 'number') {
    return nullable.value ? 'Enter 提交 · Esc 取消 · 清空内容即写入 NULL' : 'Enter 提交 · Esc 取消'
  }
  return 'Enter 提交 · Esc 取消'
}

// ===================================================================
// 定位：跟随被点的单元格
// ===================================================================
const MIN_WIDTH = 260
const LONGTEXT_WIDTH = 420
const pos = ref({ top: 0, left: 0, width: MIN_WIDTH })

const layerStyle = computed(() => ({
  top: `${pos.value.top}px`,
  left: `${pos.value.left}px`,
  width: `${pos.value.width}px`,
}))

/** 锚点还在页面上、且还看得见吗（被容器裁掉 / 行被回收都算看不见） */
function anchorVisible(): boolean {
  const el = props.anchor
  if (!el || !el.isConnected) return false
  const r = el.getBoundingClientRect()
  if (r.width === 0 && r.height === 0) return false
  const x = r.left + r.width / 2
  const y = r.top + r.height / 2
  if (x < 0 || y < 0 || x > window.innerWidth || y > window.innerHeight) return false
  const hit = document.elementFromPoint(x, y)
  if (!hit) return false
  // 中心点被自己这个浮层盖住是正常的（浮层就贴在单元格上），不算「看不见」
  if (hit === layerRef.value || layerRef.value?.contains(hit)) return true
  return el === hit || el.contains(hit)
}

function updatePos() {
  const el = props.anchor
  if (!el) return
  // 单元格滚出可视区了 ⇒ 别让一个看不见的输入框留在屏幕上，直接放弃这次编辑
  // （宁可丢一次输入，也不要让人在看不见的地方继续敲）
  if (!anchorVisible()) {
    emit('cancel')
    return
  }
  const r = el.getBoundingClientRect()
  const minW = props.kind === 'longtext' ? LONGTEXT_WIDTH : MIN_WIDTH
  const width = Math.max(Math.ceil(r.width) + 2, minW)
  const maxLeft = window.innerWidth - width - 8
  const maxTop = window.innerHeight - 96
  pos.value = {
    top: Math.max(8, Math.min(r.top - 3, maxTop)),
    left: Math.max(8, Math.min(r.left - 1, maxLeft)),
    width,
  }
}

function startFollow() {
  window.addEventListener('scroll', updatePos, true)
  window.addEventListener('resize', updatePos)
}
function stopFollow() {
  window.removeEventListener('scroll', updatePos, true)
  window.removeEventListener('resize', updatePos)
}

/** 焦点是否还在浮层内部（点 NULL 复选框 / 弹层不算失焦，否则一勾选就被提交走了） */
function focusStillInLayer(): boolean {
  const active = document.activeElement
  return !!active && !!layerRef.value?.contains(active)
}

function onFocusOut() {
  nextTick(() => {
    // 长文本弹层正开着 ⇒ 焦点跑到弹层（它在 body 下）是预期行为，不是提交
    if (longtextOpen.value) return
    if (focusStillInLayer()) return
    commit()
  })
}

/** 把当前值摊平成各控件能接的形态 */
function loadFromValue() {
  const v = props.value
  if (isSqlNull(v) || v === SQL_NULL) {
    textBuffer.value = ''
    nullFlag.value = true
  } else if (v === null || v === undefined) {
    textBuffer.value = ''
    nullFlag.value = false
  } else {
    textBuffer.value = typeof v === 'object' ? JSON.stringify(v) : String(v)
    nullFlag.value = false
  }
  if (props.kind === 'bool') {
    if (textBuffer.value === '' || nullFlag.value) boolValue.value = nullable.value ? 'NULL' : 'FALSE'
    else boolValue.value = truthy(textBuffer.value) ? 'TRUE' : 'FALSE'
  }
  if (props.kind === 'date' || props.kind === 'datetime' || props.kind === 'time') {
    dateValue.value = textBuffer.value || null
  }
  localError.value = null
}

function truthy(s: string): boolean {
  const t = s.trim().toLowerCase()
  return t === '1' || t === 'true' || t === 't' || t === 'yes'
}

/**
 * 待提交的值 = 控件形态 → 单元格意图 → **按列类型归一化**。
 * 归一化会做三件事：把数字 / 时间 / JSON 列的空输入变成 NULL；把 NOT NULL 列上的空输入
 * 拦下来；顺带做数字合法性与长度校验。宁可在这里说清楚，也不要让服务端回一句看不懂的错。
 */
function valueToCommit(): { value: unknown; error: string | null } {
  let raw: unknown
  if (props.kind === 'bool') {
    raw = boolValue.value === 'NULL' ? SQL_NULL : boolValue.value === 'TRUE'
  } else if (props.kind === 'date' || props.kind === 'datetime' || props.kind === 'time') {
    raw = dateValue.value === null || dateValue.value === '' ? SQL_NULL : dateValue.value
  } else if (nullFlag.value) {
    raw = SQL_NULL
  } else {
    raw = textBuffer.value
  }
  const { value, error } = normalizeCellInput(raw, props.column)
  return { value, error }
}

function commit() {
  const { value, error } = valueToCommit()
  if (error) {
    // 校验不过就**停在编辑态**并把原因写在旁边 —— 直接退出会让人以为改动丢了
    localError.value = error
    return
  }
  stopFollow()
  longtextOpen.value = false
  emit('commit', value)
}

function cancel() {
  stopFollow()
  longtextOpen.value = false
  localError.value = null
  emit('cancel')
}

function onNullToggle() {
  if (nullFlag.value) textBuffer.value = ''
}

function closePopoverAndCommit() {
  longtextOpen.value = false
  commit()
}

onMounted(() => {
  loadFromValue()
  void nextTick(() => {
    updatePos()
    startFollow()
    if (props.kind === 'bool' || props.kind === 'date' || props.kind === 'datetime' || props.kind === 'time') return
    textRef.value?.focus?.()
    textRef.value?.select?.()
  })
})

onBeforeUnmount(stopFollow)
</script>

<!-- 浮层节点 Teleport 到 body，拿不到 scoped 的 data-v ⇒ 样式写在非 scoped 块，统一 ce- 前缀 -->
<style>
.ce-layer {
  position: fixed;
  z-index: 3000;
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 3px;
  box-sizing: border-box;
  background: var(--el-bg-color);
  border: 1px solid var(--el-color-primary);
  border-radius: var(--el-border-radius-base);
  box-shadow: var(--el-box-shadow-light);
}
.ce-main {
  display: flex;
  align-items: center;
  gap: 6px;
}
.ce-layer .ce-control {
  flex: 1;
  min-width: 0;
}
.ce-layer .ce-null {
  flex-shrink: 0;
  margin-right: 2px;
}
.ce-layer .ce-null .el-checkbox__label {
  font-size: var(--font-size-sm);
  padding-left: 4px;
}
.ce-layer .ce-error {
  font-size: var(--font-size-sm);
  line-height: 1.4;
  color: var(--stateErrorFg);
}
.ce-layer .ce-hint {
  font-size: var(--font-size-sm);
  line-height: 1.4;
  color: var(--el-text-color-secondary);
}
.ce-layer .ce-col {
  font-family: var(--font-family-mono);
}
.ce-layer .ce-control.is-invalid .el-input__wrapper {
  box-shadow: 0 0 0 1px var(--el-color-danger) inset;
}
.ce-longtext-anchor {
  display: inline-block;
  width: 0;
  height: 0;
}
.ce-pop {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.ce-pop-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}
.ce-pop .spacer {
  flex: 1;
}
</style>
