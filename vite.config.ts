import { fileURLToPath, URL } from 'node:url'
// @ts-ignore
import { createSvg } from './src/icons/index'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import VueSetupExtend from 'vite-plugin-vue-setup-extend'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue(), vueJsx(), VueSetupExtend(), createSvg('./src/icons/svg/')],
  base: './',
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  optimizeDeps: {
    include: ['countup.js']
  },
  css: {
    preprocessorOptions: {
      scss: {
        additionalData: `
          @import "@/styles/theme.scss";
        `
      }
    }
  }
})
