<template>
  <!--
    单色图标：`fill="currentColor"` 不能删，也不要换成 stroke。

    SVG 的 fill **默认值是 black，不是 currentColor** —— 不声明它，图标就恒为黑色，
    完全无视主题和父级的 color。本组件由 `plugins/terminal/index.ts` 当插件图标**直接**渲染进
    活动栏按钮（`<component :is="plugin.icon" class="activity-icon">`，**没有 `<el-icon>` 包裹**），
    因此拿不到 Element Plus `.el-icon{fill:currentColor}` 那层兜底 —— 暗色下就是黑图标压深底。

    这个字形是**实心填充**画的（`>_` 与外框都用闭合路径表达，外框是「挖空」的环），
    所以着色属性是 fill 而不是 stroke：改 stroke 只会给内外两条轮廓各描一条边，
    填色依旧是黑的；何况没写 `stroke-width` 时默认只有 1 个用户单位，viewBox 1024 缩到 22px
    后约 0.02px，等于看不见。

    声明之后颜色交给 CSS 的 `color`，活动栏已按主题给了色：
    普通 `.activity-item{color:textSecondary}` / 悬停 `textColor` / 选中 `.activity-item-active{color:activityBarActive}`，
    于是图标随主题与状态变化，与同一排的 EP 图标（Folder / Setting 等）表现一致。

    反例：彩色图标（`PDF` / `MarkDownColorful` / `Documents`）的 path 自带 `fill="#xxxxxx"`。
    那种图标**不要**加这一行 —— currentColor 会把品牌色一并抹成单色。
  -->
  <svg t="1789695944830" class="icon" fill="currentColor" viewBox="0 0 1024 1024" version="1.1" xmlns="http://www.w3.org/2000/svg" p-id="6900" width="200" height="200"><path d="M512 512a32 32 0 0 1-12 24.992l-160 128a32.064 32.064 0 1 1-40-49.984L428.768 512l-128.768-103.008a32 32 0 1 1 40-49.984l160 128A32 32 0 0 1 512 512z m192 96h-160a32 32 0 0 0 0 64h160a32 32 0 0 0 0-64z m224-384v576a64 64 0 0 1-64 64H160a64 64 0 0 1-64-64V224a64 64 0 0 1 64-64h704a64 64 0 0 1 64 64z m-64 576V224H160v576h704z" p-id="6901" ></path></svg>
</template>
