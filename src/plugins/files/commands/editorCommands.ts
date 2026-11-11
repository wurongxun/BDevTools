/**
 * files 的命令表与命令执行体（命令面板 / 全局快捷键 / 标签右键共用同一个入口）。
 *
 * 单一真源：命令面板注册的是这里导出的 CommandPayload，全局 keydown 命中的也是同一个
 * `runEditorCommand(id)` —— 不另建第二套快捷键系统（CODEBUDDY.md「命令面板接入编辑器命令」）。
 * 键位可在偏好弹窗里改，默认表在 useEditorPreferences.ts。
 */
import type { CommandPayload } from '@/core/registry'
import NotificationUtil from '@/utils/NotificationUtil'
import { showContextMenu } from '@/components/context-menu/useContextMenu'
import { useDocumentManagerStore } from '@/store/modules/documents'
import { openSearchPanel } from '../search/useFileSearch'
import { useSplitView } from '../view/useSplitView'
import { useBookmarks } from '../navigation/useBookmarks'
import { revealLine, requestLineOp } from '../navigation/editorEvents'
import type { LineOp } from '../navigation/editorEvents'
import {
  editorPreferencesVisible,
  zoomBy
} from '../preferences/useEditorPreferences'
import {
  macroDialogVisible,
  onMacroCommand,
  recordMacroStep,
  toggleRecording,
  playMacroMenu
} from '../macros/useMacros'
import { openInTerminal, runCurrentFile } from '../terminal/useTerminalLaunch'
import { openConvertDialog } from '../encoding/useEncodingConvert'

/** Monaco 的光标在组件内部，命令面板/快捷键拿不到；由 CodeEditor 在光标变化时上报。 */
let activeCursor = { path: '', line: 1 }
export function setEditorCursor(path: string, line: number) {
  activeCursor = { path, line }
}

function cursorContext() {
  const docStore = useDocumentManagerStore()
  const path = activeCursor.path && docStore.documents.has(activeCursor.path)
    ? activeCursor.path
    : docStore.activeDocumentPath || ''
  const line = activeCursor.path === path ? activeCursor.line : 1
  return { path, line }
}

function compareWithAnother() {
  const docStore = useDocumentManagerStore()
  const current = docStore.activeDocumentPath || ''
  if (!current) {
    NotificationUtil.warn('请先打开一个文件')
    return
  }
  const candidates = docStore.documentList.filter((d) => d.id !== current)
  if (candidates.length === 0) {
    NotificationUtil.warn('请先再打开一个文件用于比较')
    return
  }
  const items: any[] = candidates.slice(0, 30).map((d) => ({
    label: d.name,
    title: d.id,
    onClick: () => useSplitView().openDiff(current, d.id)
  }))
  showContextMenu({ x: Math.max(80, window.innerWidth / 3), y: 120, items })
}

/**
 * 快捷键 / 命令面板 / 宏回放共用执行体。
 * 抽出来是为了让「宏录制的命令步骤」只从对外那一个入口挂钩，回放时直接调 core
 * 不会把回放动作本身再录成新步骤。
 */
function runEditorCommandCore(id: string): boolean {
  const split = useSplitView()
  const bookmarks = useBookmarks()
  const { path, line } = cursorContext()

  // 行操作（A6）统一按 ID 前缀分发给持有 editor 的 CodeEditor
  const LINE_PREFIX = 'files.line.'
  if (id.startsWith(LINE_PREFIX)) {
    if (!path) return false
    requestLineOp(path, id.slice(LINE_PREFIX.length) as LineOp)
    return true
  }

  switch (id) {
    case 'files.view.toggleSplit':
      split.toggleSplit()
      return true
    case 'files.view.toggleSyncScroll':
      split.syncScroll.value = !split.syncScroll.value
      NotificationUtil.info(split.syncScroll.value ? '分屏同步滚动：开' : '分屏同步滚动：关')
      return true
    case 'files.view.compare':
      compareWithAnother()
      return true
    case 'files.bookmark.toggle':
      if (!path) return false
      bookmarks.toggle(path, line)
      return true
    case 'files.bookmark.next': {
      if (!path) return false
      const target = bookmarks.next(path, line)
      if (target == null) { NotificationUtil.warn('当前文件没有行书签'); return true }
      revealLine(path, target)
      return true
    }
    case 'files.bookmark.prev': {
      if (!path) return false
      const target = bookmarks.prev(path, line)
      if (target == null) { NotificationUtil.warn('当前文件没有行书签'); return true }
      revealLine(path, target)
      return true
    }
    case 'files.editor.preferences':
      editorPreferencesVisible.value = true
      return true
    case 'files.editor.convertEncoding':
      if (!path) { NotificationUtil.warn('请先打开一个文件'); return true }
      openConvertDialog(path)
      return true
    case 'files.macro.record':
      toggleRecording()
      return true
    case 'files.macro.play':
      playMacroMenu()
      return true
    case 'files.macro.manage':
      macroDialogVisible.value = true
      return true
    case 'files.terminal.openHere':
      return openInTerminal()
    case 'files.terminal.runCurrent':
      return runCurrentFile()
    case 'files.editor.zoomIn':
      zoomBy(1)
      return true
    case 'files.editor.zoomOut':
      zoomBy(-1)
      return true
    case 'files.search.inFiles':
    case 'files.search.replaceInFiles':
      openSearchPanel()
      return true
    case 'files.save.active': {
      const docStore = useDocumentManagerStore()
      if (docStore.activeDocumentPath) void docStore.saveDocument(docStore.activeDocumentPath)
      return true
    }
    default:
      return false
  }
}

/** 对外唯一执行入口：命令面板 / 全局快捷键命中后都走这里，宏录制在这一层挂钩。 */
export function runEditorCommand(id: string): boolean {
  const handled = runEditorCommandCore(id)
  // 宏自身命令与「打开终端 / 运行脚本 / 打开确认对话框」这类外部副作用都不进宏：
  // 回放时重放开终端、跑脚本、弹确认框会造成用户没预期的动作
  if (
    handled &&
    !id.startsWith('files.macro.') &&
    !id.startsWith('files.terminal.') &&
    id !== 'files.editor.convertEncoding'
  ) {
    recordMacroStep({ kind: 'command', command: id })
  }
  return handled
}

// 宏回放的命令步骤绕开上面的录制钩子，且只在浏览器环境安装监听（SSR 回归脚本可安全加载）
if (typeof window !== 'undefined') {
  onMacroCommand((id) => { runEditorCommandCore(id) })
}

const LINE_COMMANDS: { op: LineOp; title: string }[] = [
  { op: 'copyDown', title: '向下复制当前行/选区' },
  { op: 'copyUp', title: '向上复制当前行/选区' },
  { op: 'moveUp', title: '上移当前行/选区' },
  { op: 'moveDown', title: '下移当前行/选区' },
  { op: 'delete', title: '删除当前行/选区' },
  { op: 'sortAsc', title: '选区行升序排序' },
  { op: 'sortDesc', title: '选区行降序排序' },
  { op: 'dedup', title: '选区行去重' },
  { op: 'removeEmpty', title: '删除选区空行' },
  { op: 'upper', title: '选区转大写' },
  { op: 'lower', title: '选区转小写' }
]

/** 注册到命令面板的全部命令（M2 三条 + M3 编辑器命令 + A6 行操作）。 */
export function filesCommands(): CommandPayload[] {
  return [
    { command: 'files.search.inFiles', title: '在文件中查找（跨文件搜索）', category: '搜索', run: () => openSearchPanel() },
    { command: 'files.search.replaceInFiles', title: '在文件中替换（批量替换）', category: '搜索', run: () => openSearchPanel() },
    { command: 'files.save.active', title: '保存当前文件', category: '文件', run: () => { void runEditorCommand('files.save.active') } },
    { command: 'files.view.toggleSplit', title: '切换分屏双视图', category: '视图', run: () => { runEditorCommand('files.view.toggleSplit') } },
    { command: 'files.view.toggleSyncScroll', title: '切换分屏同步滚动', category: '视图', run: () => { runEditorCommand('files.view.toggleSyncScroll') } },
    { command: 'files.view.compare', title: '比较当前文件与另一份已打开文件', category: '视图', run: compareWithAnother },
    { command: 'files.bookmark.toggle', title: '切换当前行书签', category: '导航', run: () => { runEditorCommand('files.bookmark.toggle') } },
    { command: 'files.bookmark.next', title: '跳到下一个行书签', category: '导航', run: () => { runEditorCommand('files.bookmark.next') } },
    { command: 'files.bookmark.prev', title: '跳到上一个行书签', category: '导航', run: () => { runEditorCommand('files.bookmark.prev') } },
    { command: 'files.editor.preferences', title: '编辑器偏好与快捷键', category: '编辑器', run: () => { editorPreferencesVisible.value = true } },
    { command: 'files.editor.convertEncoding', title: '转换文件编码 / 换行符（带备份）', category: '文件', run: () => { runEditorCommand('files.editor.convertEncoding') } },
    { command: 'files.editor.zoomIn', title: '放大编辑器字号', category: '编辑器', run: () => zoomBy(1) },
    { command: 'files.editor.zoomOut', title: '缩小编辑器字号', category: '编辑器', run: () => zoomBy(-1) },
    { command: 'files.macro.record', title: '开始 / 停止录制宏', category: '宏', run: () => { runEditorCommand('files.macro.record') } },
    { command: 'files.macro.play', title: '回放宏（选择已保存的宏）', category: '宏', run: () => { runEditorCommand('files.macro.play') } },
    { command: 'files.macro.manage', title: '宏管理（录制 / 回放 / 删除）', category: '宏', run: () => { runEditorCommand('files.macro.manage') } },
    { command: 'files.terminal.openHere', title: '在集成终端打开所在目录', category: '终端', run: () => { runEditorCommand('files.terminal.openHere') } },
    { command: 'files.terminal.runCurrent', title: '在集成终端运行当前脚本', category: '终端', run: () => { runEditorCommand('files.terminal.runCurrent') } },
    ...LINE_COMMANDS.map(({ op, title }) => ({
      command: `files.line.${op}`,
      title,
      category: '行操作',
      run: () => { runEditorCommand(`files.line.${op}`) }
    }))
  ]
}
