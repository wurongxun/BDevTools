<script setup lang="ts">
/**
 * 独立窗口型插件的**启动卡**（宿主生成，替代沙箱 iframe 作为 mainPanel）。
 *
 * 为什么不是 iframe：清单声明了 `standaloneWindow: true` 的插件，它的真实界面在独立窗口里
 * （那里的 webview 没有 `sandbox` 限制 → 有存档 / 全屏 / 指针锁定 / 跨源隔离）。
 * 把同一个页面塞进面板内 iframe 只会得到一份「看起来一样、但能力被削掉一半」的副本，
 * 所以这个面板不再渲染页面，只负责两件事：
 * 1. 点活动栏图标时（由 `AppSidebar` 触发）窗口已经自动开了，这里是**兜底重开入口**
 * 2. 把「为什么没有宿主桥」写在用户看得见的地方，而不是让人去翻文档
 */
import { computed } from 'vue'
import { findInstalledPlugin, launchStandalonePlugin } from '@/core/plugin/launch'
import { isPluginWindowOpen } from '@/core/plugin/window'
import { PAGE_CAPABILITY_LABELS } from '@/core/plugin/types'

const props = defineProps<{
  /** 插件 ID */
  pluginId: string
}>()

/** 已安装记录（响应式：卸载 / 改名后这张卡跟着更新） */
const plugin = computed(() => findInstalledPlugin(props.pluginId))

/** 独立窗口是否已打开（决定按钮文案：打开 / 前置） */
const isOpen = computed(() => isPluginWindowOpen(props.pluginId))

/** 能力的可读名（未知值原样显示，便于发现宿主与文档不同步） */
const caps = computed(() =>
  (plugin.value?.page_capabilities ?? []).map(c => ({
    key: c,
    label: PAGE_CAPABILITY_LABELS[c] ?? c,
  }))
)

function launch(): void {
  // launchStandalonePlugin 自己处理失败提示，这里不吞异常
  void launchStandalonePlugin(props.pluginId)
}
</script>

<template>
  <div class="pl-main">
    <div class="pl-card">
      <template v-if="plugin">
        <div class="pl-head">
          <h2 class="pl-title">{{ plugin.name }}</h2>
          <span class="pl-ver">v{{ plugin.version }}</span>
        </div>
        <p v-if="plugin.description" class="pl-desc">{{ plugin.description }}</p>

        <div v-if="caps.length" class="pl-caps">
          <span v-for="c in caps" :key="c.key" class="pl-cap" :title="`pageCapabilities: ${c.key}`">
            {{ c.label }}
          </span>
        </div>

        <div class="pl-actions">
          <button type="button" class="pl-btn pl-btn-primary" @click="launch">
            {{ isOpen ? '前置独立窗口' : '打开独立窗口' }}
          </button>
          <span class="pl-entry" :title="plugin.entry">{{ plugin.entry }}</span>
        </div>

        <p class="pl-note">
          本插件声明了 <code>standaloneWindow</code>，界面跑在<strong>独立窗口</strong>里：
          独占一个 webview（卡帧不拖累主界面），并且天然有存档（localStorage）、全屏与指针锁定。
          <br />
          代价是那个窗口<strong>拿不到任何宿主命令</strong>（它的窗口标识不在 ACL 授权列表里），
          所以插件页调不到宿主的 KV / 文件 / HTTP —— 需要这些能力的插件应由作者改回面板内运行。
        </p>
        <p class="pl-note pl-note-dim">
          点左侧活动栏图标同样会打开（或前置）这个窗口；这里只是关掉窗口后的重开入口。
        </p>
      </template>

      <!-- 卸载 / 禁用后旧的启动卡可能还在渲染树里（KeepAlive），给一句明确的话而不是空白 -->
      <template v-else>
        <h2 class="pl-title">插件已不存在</h2>
        <p class="pl-desc">
          <code>{{ pluginId }}</code> 已被卸载或禁用，可在「设置 → 插件」中查看当前列表。
        </p>
      </template>
    </div>
  </div>
</template>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.pl-main {
  height: 100%;
  overflow: auto;
  padding: var(--space-6);
  box-sizing: border-box;
}

.pl-card {
  max-width: 860px;
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

.pl-head {
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
}

.pl-title {
  margin: 0 0 var(--space-3);
  font-size: var(--font-size-xl);
  font-weight: 500;
}

.pl-ver {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.pl-desc {
  margin: 0 0 var(--space-3);
  font-size: var(--font-size-base);
  line-height: 1.7;

  code {
    font-family: var(--font-family-mono);
    padding: 0 var(--space-1);
    border-radius: var(--radius-xs);

    @include theme.useTheme {
      background-color: theme.getVar('sidebarBg');
    }
  }
}

.pl-caps {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-bottom: var(--space-3);
}

.pl-cap {
  padding: 0 var(--space-2);
  font-size: var(--font-size-xs);
  line-height: 18px;
  border-radius: var(--radius-sm);
  border: 1px solid;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
    border-color: theme.getVar('borderColor');
    background-color: theme.getVar('sidebarBg');
  }
}

.pl-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-bottom: var(--space-4);
}

.pl-btn {
  min-height: 26px;
  padding: var(--space-1) var(--space-4);
  font-family: inherit;
  font-size: var(--font-size-sm);
  cursor: pointer;
  border-radius: var(--radius-sm);
  border: 1px solid;
  appearance: none;

  @include theme.useTheme {
    color: theme.getVar('textColor');
    background-color: theme.getVar('sidebarBg');
    border-color: theme.getVar('borderStrong');

    &:hover {
      background-color: theme.getVar('activityBarHover');
    }
  }
}

.pl-btn-primary {
  @include theme.useTheme {
    border-color: theme.getVar('hoverColor');
  }
}

.pl-entry {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.pl-note {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-sm);
  line-height: 1.7;

  code {
    font-family: var(--font-family-mono);
    padding: 0 var(--space-1);
    border-radius: var(--radius-xs);

    @include theme.useTheme {
      background-color: theme.getVar('sidebarBg');
    }
  }

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.pl-note-dim {
  font-size: var(--font-size-xs);
}
</style>
