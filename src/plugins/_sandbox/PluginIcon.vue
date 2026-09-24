<script setup lang="ts">
/**
 * 第三方插件活动栏图标壳。
 *
 * 插件清单里的 `icon` 只是一段**描述**，不是组件：宿主只接受
 * 「内置图标名（白名单）」或「包内静态资源相对路径」两种形态，
 * 其余一律回落默认图标——插件永远拿不到往宿主组件树里塞组件的入口。
 */
import { computed, ref, watch } from 'vue'
import * as Icons from '@element-plus/icons-vue'
import { Grid } from '@element-plus/icons-vue'
import { pluginAssetUrl } from '@/core/plugin/url'
import { PLUGIN_ICON_ASSET_RE, PLUGIN_ICON_WHITELIST } from '@/core/plugin/types'

const props = defineProps<{
  /** 清单里的图标描述（内置图标名或包内资源相对路径） */
  icon?: string
  /** 所属插件 ID（资源图标需要） */
  pluginId?: string
}>()

/** 资源图标加载失败标记（回落默认图标） */
const failed = ref(false)

type IconMode = 'builtin' | 'asset' | 'none'

const mode = computed<IconMode>(() => {
  const icon = (props.icon ?? '').trim()
  if (!icon) return 'none'
  if (PLUGIN_ICON_WHITELIST.includes(icon)) return 'builtin'
  if (PLUGIN_ICON_ASSET_RE.test(icon)) return 'asset'
  return 'none'
})

const builtinComp = computed(() => {
  if (mode.value !== 'builtin') return Grid
  const all = Icons as unknown as Record<string, unknown>
  return (all[props.icon ?? ''] as typeof Grid) ?? Grid
})

const assetSrc = computed(() =>
  pluginAssetUrl(props.pluginId ?? '', (props.icon ?? '').replace(/^\.?\//, '')),
)

/**
 * 资源图标用 **CSS mask** 渲染，而不是 `<img>`。
 *
 * 为什么不用 `<img src>`：SVG 经 `<img>` 加载时跑在**隔离的 image 文档**里，
 * 里面的 `fill="currentColor"` 只能解析到图片自身文档的默认 `color`（黑色），
 * 宿主 `<img>`/按钮的 `color` 传不进去——于是切换主题时按钮 `color` 变了，图标却纹丝不动。
 *
 * 用 mask：遮罩形状取自 SVG 的 alpha 通道（实心字形 = 不透明），填充色用
 * `background-color: currentColor`（`currentColor` 继承自 `.activity-item` 的主题色，
 * 含普通/悬停/选中三态）。这样图标随主题变色，且 SVG 内部是 `currentColor` 还是
 * 品牌色都无所谓（mask 只看 alpha）。不需要前端主动 fetch（资源由 CSS mask 直接加载），
 * 也没有 XSS 面——SVG 作为 mask 被加载时只采样 alpha、不渲染 / 不执行脚本。
 * 代价是协议侧必须给跨源资源返回 `Access-Control-Allow-Origin`（mask 要采样像素，
 * Chromium 强制要求 CORS，见 `src-tauri/src/lib.rs` 的 plugin:// 协议处理器）。
 */
const maskStyle = computed(() => {
  const url = `url("${assetSrc.value}")`
  return {
    WebkitMaskImage: url,
    maskImage: url,
  }
})

// icon / pluginId 变化时重置失败标记（同一个壳会被复用给不同插件）
watch(
  () => [props.icon, props.pluginId],
  () => { failed.value = false },
)
</script>

<template>
  <!-- 包内资源图标：CSS mask 内联着色，随主题变色 -->
  <span
    v-if="mode === 'asset' && !failed"
    class="plugin-icon-asset"
    :style="maskStyle"
    aria-hidden="true"
  />
  <!-- 资源加载失败 / 无图标 / 未知描述：回落默认图标 -->
  <component :is="builtinComp" v-else />
</template>

<style scoped>
.plugin-icon-asset {
  display: block;
  width: 22px;
  height: 22px;
  /* 填充色继承父级 .activity-item 的 color（普通/悬停/选中三态），随主题切换 */
  background-color: currentColor;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
  -webkit-mask-position: center;
  mask-position: center;
  -webkit-mask-size: contain;
  mask-size: contain;
}
</style>
