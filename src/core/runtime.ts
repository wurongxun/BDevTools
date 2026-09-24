/**
 * 框架能力适配层 —— 前端唯一的 @tauri-apps/* import 来源（除 api/rpc.ts）。
 *
 * 业务代码（store / 组件 / 插件）只允许从本文件获取窗口、事件、路径、系统信息、
 * 对话框、存储等桌面壳能力，禁止直接 import '@tauri-apps/*'。
 *
 * 将来更换 UI 框架（Electron / Web 版）时，只需重写本文件的实现，业务代码零改动。
 */

// ==================== 窗口 ====================
export { getCurrentWebviewWindow, WebviewWindow } from '@tauri-apps/api/webviewWindow'
export { getCurrentWindow } from '@tauri-apps/api/window'
export type { Window } from '@tauri-apps/api/window'

// ==================== 全局事件（跨窗口广播） ====================
// 中性 pub/sub 抽象：业务代码不感知 Tauri 的 Event 对象与 UnlistenFn 形状，
// 回调直接收到解包后的 payload。将来迁移 Electron 时只需重写本节实现。
import { listen as tauriListen, emit as tauriEmit } from '@tauri-apps/api/event'

/** 取消事件订阅 */
export type Unsubscribe = () => void

/** 订阅全局事件：任意窗口（或后端）发布的事件本窗口都会收到 */
export async function subscribe<T = void>(
  event: string,
  handler: (payload: T) => void,
): Promise<Unsubscribe> {
  return tauriListen<T>(event, (e) => handler(e.payload))
}

/** 发布全局事件：广播到应用内所有窗口（含发起窗口） */
export function publish(event: string, payload?: unknown): Promise<void> {
  return tauriEmit(event, payload)
}

// ==================== 路径 ====================
export { desktopDir, dirname } from '@tauri-apps/api/path'

// ==================== 应用信息 ====================
export { getName, getTauriVersion, getVersion } from '@tauri-apps/api/app'

// ==================== 文件转 URL（asset protocol） ====================
export { convertFileSrc } from '@tauri-apps/api/core'

// ==================== 持久化存储（settings.json / openfile.json / pdf_store.json） ====================
// 替代 @tauri-apps/plugin-store：走后端 kv_* 命令即时读写磁盘，无内存缓存，
// 消除前端插件缓存与后端直读磁盘的双轨不同步问题。
// API 形状与 plugin-store 的 load/Store 保持一致，业务调用点零改动。
import { call } from '@/api/rpc'

/** 后端统一响应包裹 */
interface ApiResponse<T> {
  code: number
  data: T
  msg?: string
}

/** KV 存储句柄（接口与 plugin-store 的 Store 一致） */
export interface Store {
  get<T = unknown>(key: string): Promise<T | undefined>
  set(key: string, value: unknown): Promise<void>
  delete(key: string): Promise<boolean>
  entries<T = unknown>(): Promise<[string, T][]>
  /** 兼容旧 API：后端即时写盘，无需显式保存 */
  save(): Promise<void>
}

/** 加载一个 KV 存储文件
 * 后端命名空间校验：应用核心文件 settings.json / openfile.json / pdf_store.json（config_dir 根），
 * 或插件文件 plugins/<plugin_id>.json（单层限定防目录穿越）。新增插件直接传新文件名，无需改后端。
 */
export async function load(
  file: string,
  _opts?: { autoSave?: boolean; defaults?: Record<string, unknown> },
): Promise<Store> {
  // 先读一次：校验白名单与文件可用性（不合法时后端返回 500）
  const res = await call<ApiResponse<[string, unknown][]>>('kv_entries', { file })
  if (res.code !== 200) {
    throw new Error(res.msg || `加载存储文件失败: ${file}`)
  }
  return {
    async get<T = unknown>(key: string): Promise<T | undefined> {
      const r = await call<ApiResponse<T | null>>('kv_get', { file, key })
      if (r.code !== 200) throw new Error(r.msg || '读取失败')
      return r.data === null ? undefined : r.data
    },
    async set(key: string, value: unknown): Promise<void> {
      const r = await call<ApiResponse<boolean>>('kv_set', { file, key, value })
      if (r.code !== 200) throw new Error(r.msg || '写入失败')
    },
    async delete(key: string): Promise<boolean> {
      const r = await call<ApiResponse<boolean>>('kv_delete', { file, key })
      if (r.code !== 200) throw new Error(r.msg || '删除失败')
      return r.data
    },
    async entries<T = unknown>(): Promise<[string, T][]> {
      const r = await call<ApiResponse<[string, T][]>>('kv_entries', { file })
      if (r.code !== 200) throw new Error(r.msg || '读取失败')
      return r.data
    },
    async save(): Promise<void> {
      /* 后端即时写盘，无需保存 */
    },
  }
}

// ==================== 系统对话框 ====================
export { open, save } from '@tauri-apps/plugin-dialog'

// ==================== 外部打开（URL / 路径 / 资源管理器定位） ====================
export { openUrl, revealItemInDir } from '@tauri-apps/plugin-opener'

// ==================== 操作系统信息 ====================
// plugin-os 的 `type`/`version` 与浏览器全局命名冲突，统一以 os* 前缀暴露
import * as os from '@tauri-apps/plugin-os'
export const osArch = os.arch
export const osPlatform = os.platform
export const osType = os.type
export const osVersion = os.version

// ==================== 开机自启 ====================
export { enable as autostartEnable, isEnabled as autostartIsEnabled, disable as autostartDisable } from '@tauri-apps/plugin-autostart'

// ==================== 应用更新 ====================
export { check as checkUpdate } from '@tauri-apps/plugin-updater'
export type { Update, DownloadEvent } from '@tauri-apps/plugin-updater'

// ==================== 进程控制 ====================
// 这里曾经 re-export `relaunch`（@tauri-apps/plugin-process），但后端从未注册该插件
// （Cargo.toml / lib.rs 里都没有），调用它只会拿到 IPC 报错——是个「导出即陷阱」的死代码。
// 更新流程也不需要它：Windows 上 tauri-plugin-updater 起完安装器就 std::process::exit(0)，
// 由安装器自己重启应用（NSIS 收到 /R）。将来真要用，先把 tauri-plugin-process 注册上再加回来。

// ==================== 文件系统 ====================
export { writeTextFile } from '@tauri-apps/plugin-fs'
