import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// 构建为「单文件 IIFE」而非 ESM 模块：插件页的 CSP 是 `script-src 'self'`（无 eval、无内联），
// 且 iframe 不透明源下 `crossorigin` 模块脚本会因缺少 CORS 头而加载失败。
// 经典 `<script src=...>`（IIFE）规避了这两点，与宿主示例 text-toolkit 的写法一致。
// Vue 经 @vitejs/plugin-vue 在构建期预编译模板（不依赖运行时编译器 / eval），满足 CSP。
export default defineConfig({
  plugins: [vue()],
  base: './',
  // Vue 3.5 的 esm-bundler 构建要求构建期把 `process.env.NODE_ENV` 替换掉，
  // 否则运行时 `process` 未定义、IIFE 首行即抛错（现象：点开插件白屏/报错）。
  // Vite 8 / rolldown 的 lib 模式不会自动注入该 define，必须显式声明。
  define: {
    'process.env.NODE_ENV': JSON.stringify('production')
  },
  build: {
    outDir: '.',
    emptyOutDir: false, // 不清除源码与 plugin.json，只追加 assets/
    cssCodeSplit: false,
    assetsInlineLimit: 0, // 强制把 CSS 抽成独立文件（便于 page.html 稳定引用）
    // Vite 8 / rolldown 不再内置 esbuild 作为 minifier；仓库未单独安装 esbuild，
    // 故关闭压缩。功能等价，仅产物体积偏大（插件场景可接受）。
    minify: false,
    lib: {
      entry: 'src/main.ts',
      formats: ['iife'],
      name: 'RedisClientApp',
      fileName: () => 'assets/app.js'
    },
    rollupOptions: {
      output: {
        // 唯一的非 JS 产物是 CSS，固定命名为 assets/app.css，便于 page.html 引用
        assetFileNames: (assetInfo) => {
          const name = assetInfo.name || ''
          if (name.endsWith('.css') || name === 'style' || (assetInfo.names && assetInfo.names.some((n: string) => n.endsWith('.css')))) {
            return 'assets/app.css'
          }
          return 'assets/[name][extname]'
        }
      }
    }
  }
})
