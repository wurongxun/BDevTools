<template>
  <div class="w3d-main">
    <div class="w3d-card">
      <h2 class="w3d-title">3D 与游戏</h2>
      <p class="w3d-desc">
        这里列出<strong>声明了页面能力</strong>（<code>pageCapabilities</code>）或<strong>独立窗口</strong>
        （<code>standaloneWindow</code>）的第三方插件。声明了独立窗口的插件，点它自己的活动栏图标就会直接开窗口；
        本页只是集中入口与兜底。
      </p>
      <p class="w3d-desc">
        为什么需要独立窗口：面板内的沙箱 iframe 进不了跨源隔离（<code>isolate</code> 永远为 false），
        也没有存档（<code>localStorage</code>）、全屏与指针锁定；独立窗口独占一个 webview，
        卡帧不拖累主界面。
      </p>
      <p class="w3d-desc w3d-desc-warn">
        独立窗口的插件页拿不到任何宿主命令（该窗口不在 ACL 授权列表里），所以它<strong>调不到宿主后端</strong>。
        需要 KV / 文件 / HTTP 的插件请仍用插件自己的面板（面板内的沙箱 iframe 才有宿主桥）——
        这也是「是否走独立窗口」必须由插件作者显式声明、不能自动推断的原因。
      </p>

      <div v-if="games.length" class="w3d-list">
        <div v-for="p in games" :key="p.id" class="w3d-item">
          <div class="w3d-item-head">
            <span class="w3d-name">{{ p.name }}</span>
            <span class="w3d-ver">v{{ p.version }}</span>
            <span
              class="w3d-mode"
              :class="p.standalone_window ? 'w3d-mode-window' : 'w3d-mode-panel'"
              :title="p.standalone_window
                ? 'standaloneWindow: true —— 活动栏图标直接开独立窗口'
                : '未声明 standaloneWindow —— 入口仍是面板内沙箱 iframe'"
            >
              {{ p.standalone_window ? '独立窗口（默认入口）' : '面板内嵌' }}
            </span>
          </div>
          <p v-if="p.description" class="w3d-item-desc">{{ p.description }}</p>
          <div class="w3d-caps">
            <span v-for="c in p.page_capabilities" :key="c" class="w3d-cap" :title="`pageCapabilities: ${c}`">
              {{ capabilityLabel(c) }}
            </span>
          </div>
          <div class="w3d-actions">
            <button type="button" class="w3d-btn w3d-btn-primary" @click="launch(p)">
              {{ isOpen(p.id) ? '前置独立窗口' : '在新窗口打开' }}
            </button>
            <button type="button" class="w3d-btn" @click="openPanel(p.id)">切到插件面板</button>
            <span class="w3d-entry" :title="p.entry">{{ p.entry }}</span>
          </div>
        </div>
      </div>

      <div v-else class="w3d-empty">
        <h3 class="w3d-sub">还没有可运行的 3D / 游戏插件</h3>
        <p class="w3d-desc">
          第三方插件默认跑在<strong>最严 CSP</strong> 下：不能编译 WASM、不能 fetch 包内资源、不能用 Worker。
          想跑游戏（哪怕是 three.js 这种纯 JS 的），请在包的 <code>plugin.json</code> 里显式声明所需能力，
          并声明 <code>standaloneWindow</code> 让它从独立窗口打开：
        </p>
        <pre class="w3d-code">{
  "id": "my-game",
  "name": "我的 3D 游戏",
  "version": "1.0.0",
  "entry": "page.html",
  "standaloneWindow": true,
  "pageCapabilities": ["wasm", "assets", "media", "worker"]
}</pre>
        <p class="w3d-desc">
          可选值：<code>wasm</code>（编译 WASM）· <code>assets</code>（fetch 包内资源）·
          <code>media</code>（音视频）· <code>worker</code>（Web Worker）·
          <code>isolate</code>（跨源隔离 → 多线程 WASM）。未声明的能力一律保持禁止，
          逐项对应一条 CSP 指令。打包与限制见开发文档
          <code>docs/plugin-dev/web3d.md</code>，示例见
          <code>examples/third-party-plugins/webgl-demo/</code>。
        </p>
        <p class="w3d-desc">
          注意 <code>standaloneWindow</code> 与 <code>pageCapabilities</code> 是<strong>两件事</strong>：
          前者决定入口在哪儿，后者决定页面能做什么。带 WASM 后端（要调宿主 <code>plugin_call</code>）的
          插件<strong>不要</strong>声明 <code>standaloneWindow</code>，否则会失去宿主桥。
        </p>
        <p class="w3d-desc">
          装好后把 <code>.zip</code> 拖到本页或从「设置 → 插件」安装，重新进入本页即可看到。
        </p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts" name="Web3dMainPanel">
import { computed } from 'vue'
import { pluginManager } from '@/core/pluginManager'
import { installedPlugins } from '@/core/plugin/registry'
import { launchStandalonePlugin } from '@/core/plugin/launch'
import { isPluginWindowOpen } from '@/core/plugin/window'
import { PAGE_CAPABILITY_LABELS, type InstalledPlugin } from '@/core/plugin/types'

/**
 * 进列表的条件：声明了页面能力**或**声明了独立窗口。
 * 两者取并集而不是只取其一 —— 只声明 `pageCapabilities` 的插件（例如纯 JS 的 three.js）
 * 在面板内就能跑，但用户往往仍想把它扔进独立窗口；只声明 `standaloneWindow` 的插件
 * 页面能力为严档，也可能需要在这里被找到。
 */
const games = computed(() =>
  installedPlugins.value.filter(
    p => p.enabled && ((p.page_capabilities?.length ?? 0) > 0 || p.standalone_window === true)
  )
)

/** 能力的可读名（未知值原样显示，便于发现宿主与文档不同步） */
function capabilityLabel(cap: string): string {
  return PAGE_CAPABILITY_LABELS[cap] ?? cap
}

/** 窗口是否已打开（决定按钮文案） */
function isOpen(id: string): boolean {
  return isPluginWindowOpen(id)
}

/** 统一入口：查记录 → 开窗口 → 失败提示都由 launch.ts 负责 */
function launch(plugin: InstalledPlugin): void {
  void launchStandalonePlugin(plugin.id)
}

function openPanel(id: string): void {
  pluginManager.switchTool(id)
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss';

.w3d-main {
  height: 100%;
  overflow: auto;
  padding: var(--space-6);
  box-sizing: border-box;
}

.w3d-card {
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

.w3d-title {
  margin: 0 0 var(--space-3);
  font-size: var(--font-size-xl);
  font-weight: 500;
}

.w3d-sub {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-md);
  font-weight: 500;
}

.w3d-desc {
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

.w3d-desc-warn {
  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.w3d-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin-top: var(--space-2);
}

.w3d-item {
  padding: var(--space-4);
  border-radius: var(--radius-md);
  border: 1px solid;

  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
    border-color: theme.getVar('borderColor');
  }
}

.w3d-item-head {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.w3d-name {
  font-size: var(--font-size-md);
  font-weight: 500;
}

// 入口形态标签：独立窗口（默认入口）vs 面板内嵌，扫一眼就能区分
.w3d-mode {
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

.w3d-mode-window {
  @include theme.useTheme {
    border-color: theme.getVar('hoverColor');
    color: theme.getVar('textColor');
  }
}

.w3d-ver {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.w3d-item-desc {
  margin: var(--space-1) 0 0;
  font-size: var(--font-size-sm);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.w3d-caps {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-top: var(--space-2);
}

.w3d-cap {
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

.w3d-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-top: var(--space-3);
}

.w3d-btn {
  min-height: 24px;
  padding: var(--space-1) var(--space-3);
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

.w3d-btn-primary {
  @include theme.useTheme {
    border-color: theme.getVar('hoverColor');
  }
}

.w3d-entry {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.w3d-code {
  margin: 0 0 var(--space-3);
  padding: var(--space-3);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  line-height: 1.6;
  overflow: auto;
  border-radius: var(--radius-md);
  border: 1px solid;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border-color: theme.getVar('borderColor');
    color: theme.getVar('textColor');
  }
}
</style>
