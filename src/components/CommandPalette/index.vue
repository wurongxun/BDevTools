<!--
  命令面板（Ctrl+Shift+P / F1）：统一唤起所有插件贡献的命令。

  它只认「命令表」，不认插件 —— 这是插件之间不互相 import 却能互相调用能力的唯一通道。
-->
<template>
  <GlobalDialog
    v-model="visible"
    title="命令面板"
    width="600px"
    :vertical-center="false"
    :close-on-overlay="true"
    @close="onClosed"
  >
    <div class="cmd-palette">
      <el-input
        ref="inputRef"
        v-model="query"
        class="cmd-input"
        placeholder="输入命令名称搜索…"
        aria-label="搜索命令"
        clearable
        @keydown="onKeydown"
      />
      <ul class="cmd-list" role="listbox" aria-label="命令列表">
        <li v-if="filtered.length === 0" class="cmd-empty">没有匹配的命令</li>
        <li v-for="(item, index) in filtered" :key="item.id">
          <button
            type="button"
            role="option"
            class="cmd-item"
            :class="{ 'is-active': index === activeIndex }"
            :aria-selected="index === activeIndex"
            @click="run(item)"
            @mouseenter="activeIndex = index"
          >
            <span class="cmd-title">{{ item.payload.title }}</span>
            <span v-if="item.payload.category" class="cmd-category">{{ item.payload.category }}</span>
          </button>
        </li>
      </ul>
    </div>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import type { InputInstance } from 'element-plus'
import GlobalDialog from '@/components/GlobalDialog.vue'
import { useCommandPalette } from '@/composables/useCommandPalette'
import { allCommands, registerCommands, HOST_CONTRIBUTOR } from '@/core/commands'
import { contributionRegistry } from '@/core/registry'
import type { CommandItem } from '@/core/registry'
import { pluginManager } from '@/core/pluginManager'
import { toggleDark } from '@/composables/useDark'
import useStore from '@/store/index'
import NotificationUtil from '@/utils/NotificationUtil'

const { visible } = useCommandPalette()
const { useLayoutStore } = useStore()

const query = ref('')
const activeIndex = ref(0)
const inputRef = ref<InputInstance | null>(null)

/** 全部命令（插件注册 + 宿主派生；列表随插件启停自动变化） */
const items = computed<CommandItem[]>(() => allCommands())

/** 模糊匹配：标题 / 分组 / 命令 ID 任一命中即可（大小写不敏感） */
const filtered = computed(() => {
  const q = query.value.trim().toLowerCase()
  const list = items.value
  if (!q) return list
  return list.filter(e =>
    e.payload.title.toLowerCase().includes(q)
    || (e.payload.category ?? '').toLowerCase().includes(q)
    || e.payload.command.toLowerCase().includes(q),
  )
})

/** 结果变化后选中项越界要拉回首位，否则 Enter 会执行到看不见的命令 */
watch(filtered, () => {
  activeIndex.value = 0
})

/** 打开时聚焦输入框并清空上次查询 */
watch(visible, async (open) => {
  if (!open) return
  query.value = ''
  activeIndex.value = 0
  await nextTick()
  inputRef.value?.focus()
})

/**
 * 键盘导航：↑↓ 选择、Enter 执行、Esc 关闭。
 * 焦点始终在输入框里，所以只监听它一个就够（面板内没有其它可聚焦控件）。
 */
function onKeydown(e: KeyboardEvent) {
  const count = filtered.value.length
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    activeIndex.value = count === 0 ? 0 : (activeIndex.value + 1) % count
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    activeIndex.value = count === 0 ? 0 : (activeIndex.value - 1 + count) % count
  } else if (e.key === 'Enter') {
    e.preventDefault()
    const target = filtered.value[activeIndex.value]
    if (target) void run(target)
  } else if (e.key === 'Escape') {
    e.preventDefault()
    visible.value = false
  }
}

/**
 * 执行选中的命令。
 *
 * 异常必须提示出来：静默吞掉会让「点了没反应」成为最难排查的一类 bug
 * （见项目约定：禁止 `.catch(() => {})`）。
 */
async function run(item: CommandItem): Promise<void> {
  visible.value = false
  try {
    await item.payload.run()
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    NotificationUtil.error(`命令「${item.payload.title}」执行失败：${msg}`)
  }
}

function onClosed() {
  query.value = ''
}

/**
 * 宿主自带的命令（不来自插件，因此由 UI 层注册 —— 它们要动 layout store 与主题，
 * 那些依赖不该反向塞进 `core/`）。
 */
function registerHostCommands(): void {
  registerCommands(HOST_CONTRIBUTOR, [
    {
      command: 'host.settings.open',
      title: '打开设置',
      category: '宿主',
      run: () => pluginManager.switchTool('settings'),
    },
    {
      command: 'host.sidebar.toggle',
      title: '显示 / 隐藏插件面板',
      category: '视图',
      run: () => useLayoutStore.setSidebarPanelCollapsed(!useLayoutStore.sidebarPanelCollapsed),
    },
    {
      command: 'host.dock.toggle',
      title: '显示 / 隐藏底部面板',
      category: '视图',
      run: () => useLayoutStore.toggleDockPanel('bottom'),
    },
    {
      command: 'host.theme.toggle',
      title: '切换深色 / 浅色主题',
      category: '宿主',
      run: () => toggleDark(),
    },
  ])
}

/**
 * 全局快捷键。
 *
 * 只在本窗口的 document 上监听：**第三方插件页是跨源沙箱 iframe**，它内部的按键不会冒泡
 * 到宿主（捕获阶段也拿不到），所以在插件页里按 Ctrl+Shift+P 唤不起来 —— 要「在插件里也能用」
 * 的全局交互只能走 OS 层 global-shortcut，那是另一个议题。
 */
function onGlobalKeydown(e: KeyboardEvent) {
  const isPaletteKey = (e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === 'P' || e.key === 'p')
  if (isPaletteKey || e.key === 'F1') {
    e.preventDefault()
    visible.value = !visible.value
  }
}

onMounted(() => {
  registerHostCommands()
  document.addEventListener('keydown', onGlobalKeydown)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onGlobalKeydown)
  // 宿主命令随面板一起撤销（正常不会卸载，留着是为了语义完整）
  contributionRegistry.withdraw(HOST_CONTRIBUTOR)
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.cmd-palette {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.cmd-input {
  flex-shrink: 0;
}

.cmd-list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 320px;
  overflow-y: auto;
}

.cmd-empty {
  padding: var(--space-6) 0;
  text-align: center;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.cmd-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  width: 100%;
  padding: var(--space-2) var(--space-3);
  border: none;
  border-radius: var(--radius-md);
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  appearance: none;

  @include theme.useTheme {
    background-color: transparent;
    color: theme.getVar('textColor');
  }
}

// 选中态用背景色而非边框：边框会挤动文字，列表滚动时视觉抖动
.cmd-item.is-active {
  @include theme.useTheme {
    background-color: theme.getVar('hoverColor');
  }
}

.cmd-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cmd-category {
  flex-shrink: 0;
  font-size: var(--font-size-sm);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}
</style>
