<script setup lang="ts">
import { computed } from 'vue'
/**
 * AppStatusDot —— 紧凑状态「色点」徽标
 *
 * 桌面工具提升信息密度的通用手法：把「连接中 / 已连接 / 失败 / 离线」这类状态词，
 * 压成一个带语义色的 8px 色点（可叠加 11px 文字标签），省掉一整列宽度。
 *
 * 设计约束（对齐 CODEBUDDY 样式硬规则）：
 * - 色点颜色用「语义色」令牌（success / warning / danger / info / primary），
 *   不是硬编码十六进制，暗色/浅色主题自动适配；
 * - 文字标签用 --font-size-xs（11px，角标/徽标档，非正文）；
 * - 色点本身带 aria-label（通过 title 透传），键盘/读屏可感知；
 * - 命中区 ≥ 24px 只对「可交互」场景生效；本组件默认是纯展示（非按钮），
 *   若需点击请由调用方包一层可点击元素。
 */

type DotColor = 'success' | 'warning' | 'danger' | 'info' | 'primary' | 'default'

interface Props {
  /** 状态语义色：success=绿 / warning=橙 / danger=红 / info=灰蓝 / primary=主色 */
  color?: DotColor
  /** 可选文字标签（如「已连接」）；不传则只显示色点 */
  label?: string
  /** 悬停提示 / 无障碍名称（读屏会读到） */
  title?: string
  /** 色点直径（px），默认 8 */
  size?: number
  /** 是否在色点外包裹一个脉冲动画（用于「连接中 / 进行中」等动态状态） */
  pulsing?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  color: 'success',
  label: '',
  title: '',
  size: 8,
  pulsing: false,
})

/** 语义色 → Element Plus 语义变量（token，随主题切换） */
const colorVar = computed(() => {
  const map: Record<DotColor, string> = {
    success: 'var(--el-color-success)',
    warning: 'var(--el-color-warning)',
    danger: 'var(--el-color-danger)',
    info: 'var(--el-color-info)',
    primary: 'var(--el-color-primary)',
    default: 'var(--el-text-color-secondary)',
  }
  return map[props.color]
})
</script>

<template>
  <span
    class="status-dot"
    :class="{ 'is-pulsing': pulsing }"
    :title="title"
    :role="title ? 'status' : undefined"
    :aria-label="title || label || color"
  >
    <span class="status-dot__point" :style="{ width: size + 'px', height: size + 'px', background: colorVar }" />
    <span v-if="label" class="status-dot__label">{{ label }}</span>
  </span>
</template>

<style scoped lang="scss">
.status-dot {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  line-height: 1;
  flex-shrink: 0;
  vertical-align: middle;
}

.status-dot__point {
  display: inline-block;
  border-radius: var(--radius-full);
  flex-shrink: 0;
}

.status-dot__label {
  font-size: var(--font-size-xs);
  color: var(--textSecondary);
  white-space: nowrap;
}

/* 「进行中」脉冲：只在尊重 prefers-reduced-motion 的前提下播放 */
.status-dot.is-pulsing .status-dot__point {
  animation: status-dot-pulse 1.2s ease-in-out infinite;
}

@keyframes status-dot-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.35;
  }
}

@media (prefers-reduced-motion: reduce) {
  .status-dot.is-pulsing .status-dot__point {
    animation: none;
  }
}
</style>
