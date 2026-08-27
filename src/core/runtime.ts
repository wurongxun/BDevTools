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

// ==================== 事件 ====================
export { listen, emit } from '@tauri-apps/api/event'
export type { UnlistenFn } from '@tauri-apps/api/event'

// ==================== 路径 ====================
export { desktopDir, dirname } from '@tauri-apps/api/path'

// ==================== 应用信息 ====================
export { getName, getTauriVersion, getVersion } from '@tauri-apps/api/app'

// ==================== 文件转 URL（asset protocol） ====================
export { convertFileSrc } from '@tauri-apps/api/core'

// ==================== 持久化存储（settings.json 等） ====================
export { load, Store } from '@tauri-apps/plugin-store'

// ==================== 系统对话框 ====================
export { open, save } from '@tauri-apps/plugin-dialog'

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
export { relaunch } from '@tauri-apps/plugin-process'

// ==================== 文件系统 ====================
export { writeTextFile } from '@tauri-apps/plugin-fs'
