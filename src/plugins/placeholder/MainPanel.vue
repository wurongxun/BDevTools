<template>
  <div class="ph-main">
    <div class="ph-card">
      <h2 class="ph-title">占位插件：验证「无侧栏插件」布局</h2>
      <p class="ph-desc">
        本插件只注册了 <code>mainPanel</code>，没有 <code>sidebarPanel</code>。
        宿主应把侧栏收敛到活动栏宽度（48px），让本面板占满剩余宽度——而不是留下一条空白侧栏。
      </p>

      <h3 class="ph-sub">自检清单</h3>
      <ul class="ph-list">
        <li>左侧只剩图标列，侧栏实宽约 48px（见下方实时读数）</li>
        <li>侧栏与主区之间的拖拽分隔条消失</li>
        <li>顶栏「显示 / 隐藏插件面板」按钮置灰，提示「当前插件没有侧栏面板」</li>
        <li>点击左侧本插件图标不会自动展开面板，其它插件的面板显隐偏好不受影响</li>
        <li>切回「日志 / 数据库」等有侧栏的插件，侧栏宽度恢复到该插件记住的值</li>
      </ul>

      <h3 class="ph-sub">实时状态</h3>
      <dl class="ph-live">
        <div class="ph-row">
          <dt>侧栏实宽</dt>
          <dd class="ph-mono">{{ sidebarWidthText }}</dd>
        </div>
        <div class="ph-row">
          <dt>最近布局事件</dt>
          <dd class="ph-mono">{{ lastReason }}</dd>
        </div>
        <div class="ph-row">
          <dt>当前激活插件</dt>
          <dd class="ph-mono">{{ activeId || '(空)' }}</dd>
        </div>
        <div class="ph-row">
          <dt>该插件有侧栏</dt>
          <dd class="ph-mono">{{ activeHasSidebar ? '是' : '否' }}</dd>
        </div>
      </dl>

      <h3 class="ph-sub">已注册插件</h3>
      <div class="ph-chips">
        <button
          v-for="p in plugins"
          :key="p.id"
          type="button"
          class="ph-chip"
          :class="{ 'is-active': p.id === activeId }"
          :aria-pressed="p.id === activeId"
          :title="`切换到「${p.label}」`"
          @click="switchTo(p.id)"
        >
          {{ p.label }}
          <span class="ph-chip-flag" :class="p.hasSidebar ? 'is-yes' : 'is-no'">
            {{ p.hasSidebar ? '有侧栏' : '无侧栏' }}
          </span>
        </button>
      </div>
      <p class="ph-hint">点击插件名即可切换，用来观察切回有侧栏插件时宽度是否恢复。</p>
    </div>
  </div>
</template>

<script setup lang="ts" name="PlaceholderMainPanel">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { pluginManager } from '@/core/pluginManager'
import { contributionRegistry } from '@/core/registry'

/**
 * 侧栏实宽从 Layout 派发的同窗口事件 layout-update 里读（detail.sidebarWidth）
 * 不查 DOM，避免多实例/多窗口下取值串味
 */
const sidebarWidth = ref<number | null>(null)
const lastReason = ref('—')

function onLayoutUpdate(e: Event) {
  const detail = (e as CustomEvent<{ sidebarWidth?: number; reason?: string }>).detail
  if (!detail) return
  if (typeof detail.sidebarWidth === 'number') sidebarWidth.value = detail.sidebarWidth
  lastReason.value = detail.reason ?? '—'
}

onMounted(() => {
  window.addEventListener('layout-update', onLayoutUpdate)
})

onUnmounted(() => {
  window.removeEventListener('layout-update', onLayoutUpdate)
})

const sidebarWidthText = computed(() =>
  sidebarWidth.value === null ? '等待布局事件…' : `${sidebarWidth.value} px`
)
const activeId = computed(() => pluginManager.activeToolId.value)
const activeHasSidebar = computed(() => pluginManager.activeHasSidebar)
/**
 * 查注册表而不是读插件对象：这里要展示的是「宿主看到了什么」，
 * 与布局真正渲染用的必须是同一份数据 —— 否则诊断面板会给出与实际不一致的结论。
 */
const plugins = computed(() => {
  const sides = new Set(contributionRegistry.views('sidebar').map(e => e.pluginId))
  return contributionRegistry.activityItems().map(e => ({
    id: e.pluginId,
    label: e.payload.label,
    hasSidebar: sides.has(e.pluginId),
  }))
})

function switchTo(id: string) {
  pluginManager.switchTool(id)
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.ph-main {
  height: 100%;
  overflow: auto;
  padding: var(--space-6);
  box-sizing: border-box;
}

.ph-card {
  max-width: 760px;
  margin: 0 auto;
  padding: var(--space-6);
  border-radius: var(--radius-lg);
  border: 1px solid;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border-color: theme.getVar('borderColor');
    color: theme.getVar('textColor');
  }
}

.ph-title {
  margin: 0 0 var(--space-3);
  font-size: var(--font-size-xl);
  font-weight: 500;
}

.ph-sub {
  margin: var(--space-6) 0 var(--space-2);
  font-size: var(--font-size-md);
  font-weight: 500;
}

.ph-desc {
  margin: 0;
  font-size: var(--font-size-base);
  line-height: 1.6;

  code {
    font-family: var(--font-family-mono);
    padding: 0 var(--space-1);
    border-radius: var(--radius-xs);

    @include theme.useTheme {
      background-color: theme.getVar('sidebarBg');
    }
  }
}

.ph-list {
  margin: 0;
  padding-left: var(--space-6);
  font-size: var(--font-size-base);
  line-height: 1.8;
}

.ph-live {
  margin: 0;
  font-size: var(--font-size-base);
}

.ph-row {
  display: flex;
  gap: var(--space-4);
  padding: var(--space-1) 0;

  dt {
    width: 120px;
    flex-shrink: 0;

    @include theme.useTheme {
      color: theme.getVar('textSecondary');
    }
  }

  dd {
    margin: 0;
  }
}

.ph-mono {
  font-family: var(--font-family-mono);
}

.ph-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.ph-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  min-height: 24px;
  padding: var(--space-1) var(--space-3);
  font-family: inherit;
  font-size: var(--font-size-sm);
  cursor: pointer;
  border-radius: var(--radius-md);
  border: 1px solid;
  appearance: none;
  transition: background-color 0.15s;

  @include theme.useTheme {
    color: theme.getVar('textColor');
    background-color: theme.getVar('sidebarBg');
    border-color: theme.getVar('borderColor');

    &:hover {
      background-color: theme.getVar('activityBarHover');
    }
  }

  &.is-active {
    @include theme.useTheme {
      border-color: theme.getVar('hoverColor');
    }
  }
}

.ph-chip-flag {
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &.is-yes {
    @include theme.useTheme {
      color: theme.getVar('hoverColor');
    }
  }
}

.ph-hint {
  margin: var(--space-2) 0 0;
  font-size: var(--font-size-sm);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>
