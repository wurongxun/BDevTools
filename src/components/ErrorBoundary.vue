<template>
  <!-- 故障边界：把子孙组件渲染/setup 里的异常圈在这一块里。
       正常时不产生任何包装节点（slot 直接展开），只有真出错才替换成下面的兜底面板。 -->
  <div v-if="failed" class="plugin-error-boundary" role="alert">
    <div class="plugin-error-boundary-title">{{ scope }}出错了</div>
    <p class="plugin-error-boundary-msg">{{ message }}</p>
    <p class="plugin-error-boundary-hint">
      其它面板不受影响。堆栈已记进「错误」列表，方便反馈时一并给出。
    </p>
    <div class="plugin-error-boundary-actions">
      <app-button size="small" @click="reset">重新加载面板</app-button>
    </div>
  </div>
  <slot v-else :retry-key="retryKey" />
</template>

<script setup lang="ts" name="ErrorBoundary">
/**
 * ErrorBoundary —— 插件面板的故障边界
 *
 * 之前整棵组件树只靠 `main.ts` 里挂的全局 `app.config.errorHandler` 兜底：
 * 一个插件面板在渲染里抛异常（取不到后端数据、第三方组件炸了），
 * 故障半径是**整个窗口** —— 页面剩余部分跟着被卸载，用户只看到界面突然白掉。
 *
 * 这里把半径收回到「一块面板」：
 * - `onErrorCaptured` 捕获子孙错误后返回 `false` 阻断继续冒泡，改由本组件渲染兜底 UI；
 * - 手动调一次全局 handler，保证错误**也**进错误列表（恰好一次，不会重复两条）；
 * - 「重新加载面板」换 key 强制重建 —— KeepAlive 会缓存崩过的实例，用同一个 key 重试
 *   会把同一个坏实例再挂上来。
 */
import { computed, onErrorCaptured, ref, watch } from 'vue'
import AppButton from '@/components/AppButton.vue'
import errorHandler from '@/errorHandler'

const props = withDefaults(defineProps<{
  /** 出错位置的可读名字（如「数据库 · 主面板」），让用户知道坏的是哪一块 */
  scope: string
}>(), {})

/** 捕获到的错误（null = 当前没出错） */
const failed = ref<unknown>(null)

/** 重试序号：每次重试 +1，用作内部组件的 key 以绕开 KeepAlive 缓存 */
const reloadSeq = ref(0)
const retryKey = computed(() => String(reloadSeq.value))

const message = computed(() => {
  const err = failed.value
  if (err instanceof Error) return err.message || String(err)
  return typeof err === 'string' ? err : '未知错误'
})

onErrorCaptured((err) => {
  failed.value = err
  // 返回 false 会拦住继续冒泡，全局 handler 就收不到了 —— 所以这里手动转交一次，
  // 让「错误」列表仍然有堆栈（而且只有一条）。
  errorHandler(err)
  return false
})

// 面板归属换了（切插件 / 换了 slot）就复位：上一次崩溃不该永久占着这块位置
watch(() => props.scope, () => {
  failed.value = null
})

function reset() {
  failed.value = null
  reloadSeq.value += 1
}
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as *;
@use '@/styles/theme.scss' as theme;

.plugin-error-boundary {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  height: 100%;
  padding: var(--space-4);
  box-sizing: border-box;
  overflow: auto;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
  }
}

.plugin-error-boundary-title {
  font-size: var(--font-size-md);
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('dangerText');
  }
}

.plugin-error-boundary-msg {
  margin: 0;
  font-size: var(--font-size-sm);
  font-family: var(--font-family-mono);
  word-break: break-word;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.plugin-error-boundary-hint {
  margin: 0;
  font-size: var(--font-size-xs);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.plugin-error-boundary-actions {
  display: flex;
  gap: var(--space-2);
}
</style>
