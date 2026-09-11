<script setup lang="ts">
/**
 * AppButton —— el-button 的本地替代组件
 *
 * 设计目标：调用点只需把 <el-button> 标签替换为 <app-button>，属性/事件/插槽
 * 全部同名透传，行为与观感对齐 Element Plus Button：
 * - 内部渲染原生 <button type="button">，不再依赖 el-button 组件本身；
 * - class 体系（el-button el-button--type/--size is-*）与 DOM 结构
 *   （icon/loading 的 <el-icon> + 内容 <span> 包裹）1:1 复刻 EP，
 *   使既有全局样式（EP theme css + element.scss 桌面密度调校）继续生效，
 *   视觉零回归；
 * - disabled/loading 时原生 disabled 生效，阻止点击。
 *
 * 后续若彻底移除 EP 按钮样式，只需在本文件内把 class 换成自研 .app-btn
 * 体系，调用点无需再动。
 */
import { computed, useSlots } from 'vue'
import * as Icons from '@element-plus/icons-vue'
import { Loading } from '@element-plus/icons-vue'

interface AppButtonProps {
  type?: '' | 'default' | 'primary' | 'success' | 'warning' | 'info' | 'danger'
  size?: '' | 'default' | 'small' | 'large'
  plain?: boolean
  round?: boolean
  circle?: boolean
  text?: boolean
  link?: boolean
  dashed?: boolean
  loading?: boolean
  disabled?: boolean
  bg?: boolean
  /** 图标：可传组件对象（如 `:icon="Edit"`），或图标名字符串（如 `icon="Close"`） */
  icon?: string | object
  /** 无边框纯图标工具栏按钮（桌面风格）：透明背景、无边框、紧凑 22×22，hover 浮起 */
  bare?: boolean
  autoInsertSpace?: boolean
}

const props = withDefaults(defineProps<AppButtonProps>(), {})
const slots = useSlots()

/**
 * 解析 icon prop 为可渲染的组件：
 * - 传组件对象 → 原样使用
 * - 传字符串 → 从 @element-plus/icons-vue 按名字查（找不到则回退为 null，不渲染图标）
 */
const iconComponent = computed(() => {
  if (!props.icon) return null
  if (typeof props.icon === 'string') {
    const name = props.icon as keyof typeof Icons
    return (Icons[name] as unknown as object) || null
  }
  return props.icon
})

/** 是否「只显示图标」：有 icon 且无默认插槽（无文字内容） */
const iconOnly = computed(() => !!props.icon && !slots.default)

const rootClass = computed(() => {
  const cls = ['el-button']
  if (props.type && props.type !== 'default') cls.push(`el-button--${props.type}`)
  if (props.size && props.size !== 'default') cls.push(`el-button--${props.size}`)
  if (props.disabled) cls.push('is-disabled')
  if (props.loading) cls.push('is-loading')
  if (props.plain) cls.push('is-plain')
  if (props.round) cls.push('is-round')
  if (props.circle) cls.push('is-circle')
  if (props.text) cls.push('is-text')
  if (props.dashed) cls.push('is-dashed')
  if (props.link) cls.push('is-link')
  if (props.bg) cls.push('is-has-bg')
  // 无边框纯图标工具栏按钮（桌面风格，参照各插件 .header-btn）
  if (props.bare) cls.push('is-bare')
  // 纯图标模式：只显示图标、无文字时收窄内边距，形成紧凑图标按钮
  if (iconOnly.value && !props.circle && !props.bare) cls.push('is-icon-only')
  return cls
})

const isDisabled = computed(() => !!(props.disabled || props.loading))
</script>

<template>
  <button
    type="button"
    :class="rootClass"
    :disabled="isDisabled"
    :aria-disabled="isDisabled || undefined"
  >
    <el-icon v-if="loading" class="is-loading"><Loading /></el-icon>
    <el-icon v-else-if="iconComponent"><component :is="iconComponent" /></el-icon>
    <span v-if="slots.default"><slot /></span>
  </button>
</template>
