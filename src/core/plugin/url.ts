/**
 * 插件沙箱页面地址。
 *
 * 自定义协议在 wry 上各平台形态不同（与 tauri `convertFileSrc` 的分支一致）：
 * - Windows / Android：`{scheme}://localhost/` 会被改写成 `http://{scheme}.localhost/`，
 *   所以必须用 `http://plugin.localhost/` 才能命中注册的协议
 * - macOS / Linux / iOS：原生自定义 scheme，用 `plugin://localhost/`
 *
 * 路径首段固定是插件 ID（`plugin://localhost/<plugin_id>/<相对路径>`），
 * 这与后端 `plugin_host::split_plugin_uri` 的解析约定一致。
 */
import { osPlatform } from '@/core/runtime'

let assetOrigin = 'plugin://localhost/'

/** 初始化资源地址前缀（在注册第三方插件之前 await 一次） */
export async function initPluginAssetOrigin(): Promise<void> {
  try {
    const platform = await osPlatform()
    assetOrigin = platform === 'windows' || platform === 'android'
      ? 'http://plugin.localhost/'
      : 'plugin://localhost/'
  } catch {
    assetOrigin = 'plugin://localhost/'
  }
}

/** 当前生效的资源地址前缀（调试/自检用） */
export function pluginAssetOrigin(): string {
  return assetOrigin
}

/** 拼装插件包内资源 URL；`rel` 必须是包内相对路径 */
export function pluginAssetUrl(pluginId: string, rel: string): string {
  const clean = String(rel || '').replace(/^\.?\/+/, '')
  const segs = clean
    .split('/')
    .filter(Boolean)
    .map(encodeURIComponent)
    .join('/')
  return `${assetOrigin}${encodeURIComponent(pluginId)}/${segs}`
}
