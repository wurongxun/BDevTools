<template>
  <div class="file-editor-container">
    <!--
      分屏（M3）：每个 pane 一个独立 KeepAlive，component key 带 paneId。
      两个 pane 若共用缓存 key 会命中同一实例；同一路径禁止双开，
      否则文档级单份 viewState 会被两个 Monaco 互抢（editor-and-tabs.md:42）。
    -->
    <div class="file-editor-panes" :class="{ 'is-split': enabled }">
      <div
        v-for="pane in panes"
        :key="pane.id"
        class="file-editor-pane"
        :class="{ 'is-active': enabled && activePaneId === pane.id }"
        @mousedown.capture="split.setActivePane(pane.id)"
      >
        <DiffEditor
          v-if="pane.mode === 'diff' && diffMode"
          :original-path="diffMode.originalPath"
          :modified-path="diffMode.modifiedPath"
        />
        <KeepAlive v-else-if="paneDoc(pane.path)" :max="8">
          <component
            :is="editorComponentFor(pane.path)"
            :key="`${pane.id}:${docKey(pane.path)}`"
            :path="pane.path"
            @update-state="handleStateUpdate"
          />
        </KeepAlive>
        <div v-else class="editor-welcome-placeholder">
          <div class="welcome-content">
            <p class="welcome-desc">
              {{ enabled && pane.id === 'right' ? '在标签栏右键选择「在右侧分屏打开」，或 Ctrl+\\ 切换分屏' : '从左侧文件树中选择文件开始编辑' }}
            </p>
          </div>
        </div>
      </div>
    </div>

    <!-- 全局单例：命令面板 / 状态栏都能拉起；偏好与快捷键写 settings.json -->
    <EditorPreferencesDialog />
    <MacroDialog />
    <EncodingConvertDialog />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { useDocumentManagerStore } from '@/store/modules/documents'
import type { DocumentState } from '@/store/modules/documents'
import CodeEditor from './EditorType/CodeEditor.vue'
import ImageEditor from './EditorType/ImageEditor.vue'
import PdfEditor from './EditorType/PdfEditor.vue'
import MdEditor from './EditorType/MdEditor.vue'
import TextView from './EditorType/TextView.vue'
import DiffEditor from './EditorType/DiffEditor.vue'
import EditorPreferencesDialog from '../preferences/EditorPreferencesDialog.vue'
import MacroDialog from '../macros/MacroDialog.vue'
import EncodingConvertDialog from '../encoding/EncodingConvertDialog.vue'
import { initEditorPreferences, commandForKeybinding } from '../preferences/useEditorPreferences'
import { initMacros } from '../macros/useMacros'
import { initUndoHistory } from '../search/useUndoHistory'
import { initBookmarks } from '../navigation/useBookmarks'
import { useSplitView, initSplitView } from '../view/useSplitView'
import { runEditorCommand } from '../commands/editorCommands'

const docStore = useDocumentManagerStore()
const split = useSplitView()
const { enabled, activePaneId, diffMode, panes } = split

function paneDoc(path: string): DocumentState | undefined {
  return path ? docStore.documents.get(path) : undefined
}

/** 与 M1 前一致：大小分诊只看 doc.content 是否为 string（editor-and-tabs.md:10）。 */
function editorComponentFor(path: string) {
  const doc = paneDoc(path)
  if (!doc) return CodeEditor
  switch (doc.type) {
    case 'markdown': return MdEditor
    case 'pdf': return PdfEditor
    case 'image': return ImageEditor
    case 'code':
    case 'text':
      return typeof doc.content === 'string' ? CodeEditor : TextView
    default: return CodeEditor
  }
}

function docKey(path: string): string {
  const doc = paneDoc(path)
  return doc ? `${doc.type}:${doc.key}` : path
}

/**
 * 收子编辑器的视图状态（光标 / 滚动位置）。
 *
 * path 以**上报者**为准，不能用 activeDocumentPath 收口：切 tab 时旧编辑器会先
 * deactivate，此刻 activeDocumentPath 已经指向新文档 —— 用它收口会把旧文档的
 * 滚动位置写到新文档上（两个标签互相污染位置）。子编辑器上报时会带上自己的 path。
 * 分屏禁止同路径双开，因此这里同样不需要按 pane 分流。
 */
function handleStateUpdate(state: any) {
  if (!state) return
  const path = typeof state.path === 'string' && state.path ? state.path : docStore.activeDocumentPath
  if (!path) return
  const { path: _ignored, ...viewState } = state
  docStore.updateViewState(path, viewState)
}

/**
 * 全局快捷键：键位表在偏好里可改。
 * 让位规则：终端查找栏、搜索面板、以及偏好弹窗里的表单控件（用户正在录入键位时
 * 不能把 Ctrl+F2 吞掉）；Monaco 的编辑区不带这些 class，仍可正常触发。
 */
function onGlobalKeydown(e: KeyboardEvent) {
  if (e.defaultPrevented) return
  const target = e.target as HTMLElement | null
  // 表单/输入框里的按键归控件自己：偏好键位、宏命名、大文件行编辑都不能被全局快捷键吞掉
  if (target?.closest('.xterm, .fs-panel, .pref-body, .macro-body, .line-editor')) return
  const id = commandForKeybinding(e)
  if (!id || !runEditorCommand(id)) return
  e.preventDefault()
  e.stopPropagation()
}

onMounted(() => {
  // 先挂监听再做异步初始化：① 初始化失败不能让整套快捷键失效；
  // ② 组件在 await 期间被卸载时，不能出现「先 remove 后 add」的监听器泄漏。
  window.addEventListener('keydown', onGlobalKeydown)
  void Promise.all([
    initEditorPreferences(),
    initBookmarks(),
    initSplitView(),
    initMacros(),
    initUndoHistory()
  ]).catch((e) => {
    console.warn('[files] 编辑器偏好/书签/分屏/宏/撤销历史初始化失败，已沿用默认值', e)
  })
})

onUnmounted(() => {
  window.removeEventListener('keydown', onGlobalKeydown)
})
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

.file-editor-container {
  width: 100%;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.file-editor-panes {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: row;
}

.file-editor-pane {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.file-editor-panes.is-split .file-editor-pane + .file-editor-pane {
  border-left: 1px solid var(--borderColor);
}

.file-editor-panes.is-split .file-editor-pane.is-active {
  box-shadow: inset 0 2px 0 var(--stateInfoFg);
}

.editor-welcome-placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;

  @include theme.useTheme {
    background-color: theme.getVar('appMainBgColor');
  }
}

.welcome-content {
  text-align: center;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.welcome-desc {
  margin: 0;
  padding: 0 var(--space-3);
  font-size: theme.getVar('fontSize');
}
</style>
