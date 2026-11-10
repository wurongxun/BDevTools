/**
 * 宏：录制 → 命名保存 → 回放（M4 / F2，场景 10「重复排版动作一键完成」）。
 *
 * 设计口径：
 * - **只录两类可安全回放的步骤**：编辑器命令（命令表里的 id）与文本插入片段。
 *   不录光标移动、纯删除、选区状态 —— 回放时上下文不同，硬回放只会破坏文档。
 * - 命令步骤统一从 `runEditorCommand` 这一个执行体挂钩，命令面板、全局快捷键、
 *   标签右键三条入口录到的是同一份步骤。
 * - 回放通过 window CustomEvent 把意图交给「持有 Monaco 实例」的组件，宏服务自己
 *   不 import editorCommands（避免循环依赖），也不直接碰 monaco。
 * - 宏落 `settings.json`，与偏好 / 书签 / 查找历史同一套 KV 通道。
 * - 有硬上限（每宏 200 步、最多 20 个宏、单片段 2000 字符）和可中断的回放。
 */
import { ref } from 'vue'
import { load, type Store } from '@/core/runtime'
import NotificationUtil from '@/utils/NotificationUtil'
import { showContextMenu } from '@/components/context-menu/useContextMenu'

export interface MacroStep {
  kind: 'command' | 'text'
  command?: string
  text?: string
}

export interface Macro {
  id: string
  name: string
  steps: MacroStep[]
  createdAt: number
}

const STORE_KEY = 'files_macros'
const MAX_STEPS = 200
const MAX_MACROS = 20
const MAX_TEXT = 2000

const MACRO_COMMAND_EVENT = 'files:macro-command'
const MACRO_INSERT_EVENT = 'files:macro-insert'

export const macros = ref<Macro[]>([])
export const macroDialogVisible = ref(false)
export const recording = ref(false)
export const playing = ref(false)
export const recordedCount = ref(0)
/** 停止录制后待用户命名的步骤；不命名就不落盘，避免攒一堆无名宏 */
export const pendingSteps = ref<MacroStep[]>([])

let current: MacroStep[] = []
let playToken = 0
let macroSeq = 0
let initialized = false
let storeHandle: Promise<Store> | null = null
let limitWarned = false

function store(): Promise<Store> {
  if (!storeHandle) {
    storeHandle = load('settings.json', { autoSave: false, defaults: {} }).catch((e: unknown) => {
      storeHandle = null
      throw e
    })
  }
  return storeHandle
}

function sanitizeSteps(input: unknown): MacroStep[] {
  if (!Array.isArray(input)) return []
  const out: MacroStep[] = []
  for (const raw of input) {
    if (!raw || typeof raw !== 'object') continue
    const step = raw as Partial<MacroStep>
    if (step.kind === 'command' && typeof step.command === 'string' && step.command) {
      out.push({ kind: 'command', command: step.command })
    } else if (
      step.kind === 'text' &&
      typeof step.text === 'string' &&
      step.text.length > 0 &&
      step.text.length <= MAX_TEXT
    ) {
      out.push({ kind: 'text', text: step.text })
    }
    if (out.length >= MAX_STEPS) break
  }
  return out
}

export async function initMacros(): Promise<void> {
  if (initialized) return
  initialized = true
  try {
    const s = await store()
    const saved = await s.get<{ value: Macro[] }>(STORE_KEY)
    if (Array.isArray(saved?.value)) {
      macros.value = saved.value
        .filter((m) => m && typeof m.id === 'string' && typeof m.name === 'string')
        .slice(0, MAX_MACROS)
        .map((m) => ({
          id: m.id,
          name: m.name,
          steps: sanitizeSteps(m.steps),
          createdAt: typeof m.createdAt === 'number' ? m.createdAt : Date.now()
        }))
        .filter((m) => m.steps.length > 0)
    }
  } catch (e) {
    console.warn('[files/macros] 读取宏失败，使用空列表', e)
  }
}

async function persist(): Promise<void> {
  try {
    const s = await store()
    await s.set(STORE_KEY, { value: JSON.parse(JSON.stringify(macros.value)) })
  } catch (e) {
    console.warn('[files/macros] 保存宏失败', e)
  }
}

// ===== 事件通道：回放意图交给持有编辑器 / 命令执行体的一方 =====

export function requestMacroCommand(id: string): void {
  if (!id) return
  window.dispatchEvent(new CustomEvent(MACRO_COMMAND_EVENT, { detail: { id } }))
}

export function onMacroCommand(handler: (id: string) => void): () => void {
  const listener = (e: Event) => {
    const id = (e as CustomEvent<{ id: string }>).detail?.id
    if (typeof id === 'string' && id) handler(id)
  }
  window.addEventListener(MACRO_COMMAND_EVENT, listener)
  return () => window.removeEventListener(MACRO_COMMAND_EVENT, listener)
}

export function requestMacroInsert(text: string): void {
  if (!text) return
  window.dispatchEvent(new CustomEvent(MACRO_INSERT_EVENT, { detail: { text } }))
}

export function onMacroInsert(handler: (text: string) => void): () => void {
  const listener = (e: Event) => {
    const text = (e as CustomEvent<{ text: string }>).detail?.text
    if (typeof text === 'string' && text) handler(text)
  }
  window.addEventListener(MACRO_INSERT_EVENT, listener)
  return () => window.removeEventListener(MACRO_INSERT_EVENT, listener)
}

// ===== 录制 =====

export function recordMacroStep(step: MacroStep): void {
  if (!recording.value || playing.value) return
  if (current.length >= MAX_STEPS) {
    if (!limitWarned) {
      limitWarned = true
      NotificationUtil.warn(`单个宏最多 ${MAX_STEPS} 步，后续动作不再记录`)
    }
    return
  }
  // 逐字输入会瞬间产生上百条片段，连续文本步骤合并成一条
  if (step.kind === 'text' && step.text && current.length && current[current.length - 1].kind === 'text') {
    const prev = current[current.length - 1]
    const merged = (prev.text || '') + step.text
    if (merged.length <= MAX_TEXT) {
      current[current.length - 1] = { kind: 'text', text: merged }
      recordedCount.value = current.length
      return
    }
  }
  if (step.kind === 'command' && step.command) {
    current.push({ kind: 'command', command: step.command })
  } else if (step.kind === 'text' && step.text && step.text.length <= MAX_TEXT) {
    current.push({ kind: 'text', text: step.text })
  }
  recordedCount.value = current.length
}

export function startRecording(): void {
  if (playing.value) {
    NotificationUtil.warn('正在回放宏，无法开始录制')
    return
  }
  current = []
  pendingSteps.value = []
  recordedCount.value = 0
  limitWarned = false
  recording.value = true
  NotificationUtil.info('开始录制：编辑器命令与输入片段会被记录（不录光标与纯删除）')
}

export function stopRecording(): void {
  if (!recording.value) return
  recording.value = false
  pendingSteps.value = [...current]
  current = []
  recordedCount.value = 0
  // 交给对话框显式命名，未命名前不落盘
  macroDialogVisible.value = true
  if (!pendingSteps.value.length) {
    NotificationUtil.warn('本次录制没有可用步骤，未生成宏')
  }
}

export function toggleRecording(): void {
  if (recording.value) stopRecording()
  else startRecording()
}

export function discardPendingMacro(): void {
  pendingSteps.value = []
}

export function savePendingMacro(name: string): boolean {
  const steps = sanitizeSteps(pendingSteps.value)
  if (!steps.length) return false
  if (macros.value.length >= MAX_MACROS) {
    NotificationUtil.warn(`最多保存 ${MAX_MACROS} 个宏，请先删除一个`)
    return false
  }
  macroSeq += 1
  const macro: Macro = {
    id: `macro-${Date.now()}-${macroSeq}`,
    name: name.trim() || `宏 ${macros.value.length + 1}`,
    steps,
    createdAt: Date.now()
  }
  macros.value = [macro, ...macros.value]
  pendingSteps.value = []
  void persist()
  NotificationUtil.success(`已保存宏「${macro.name}」（${steps.length} 步）`)
  return true
}

export function deleteMacro(id: string): void {
  macros.value = macros.value.filter((m) => m.id !== id)
  void persist()
}

// ===== 回放 =====

export function stopPlayback(): void {
  playToken += 1
  playing.value = false
}

export async function runMacro(id: string): Promise<void> {
  const macro = macros.value.find((m) => m.id === id)
  if (!macro || playing.value) return
  if (recording.value) stopRecording()
  const token = ++playToken
  playing.value = true
  try {
    for (const step of macro.steps) {
      if (token !== playToken) break
      if (step.kind === 'command' && step.command) requestMacroCommand(step.command)
      else if (step.kind === 'text' && step.text) requestMacroInsert(step.text)
      // 让 Vue/Monaco 处理完上一步再发下一步，避免命令与输入挤在同一 tick
      await new Promise((resolve) => setTimeout(resolve, 0))
    }
  } finally {
    if (token === playToken) playing.value = false
  }
}

/** 命令面板「回放宏」：用宿主右键菜单列出宏，不新建第二套列表组件。 */
export function playMacroMenu(): void {
  if (!macros.value.length) {
    NotificationUtil.warn('还没有保存的宏，请先录制')
    return
  }
  const items = macros.value.slice(0, 20).map((m) => ({
    label: `${m.name}（${m.steps.length} 步）`,
    title: m.name,
    onClick: () => { void runMacro(m.id) }
  }))
  showContextMenu({ x: Math.max(80, window.innerWidth / 3), y: 120, items })
}
