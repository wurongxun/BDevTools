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

/**
 * 拼装插件包内资源 URL；`rel` 必须是包内相对路径。
 *
 * `query` 会被拼成 `?k=v`：后端 `split_plugin_uri` 只用 `uri.path()` 解析 `<插件ID>/<相对路径>`，
 * **query 不参与路径解析**，所以追加参数是安全的（不会 404、也不会被当成目录名）。
 * 这是「独立窗口插件页拿宿主初值」的唯一通道 —— 那种窗口拿不到桥，见 `window.ts` 文件头。
 */
export function pluginAssetUrl(
  pluginId: string,
  rel: string,
  query?: Record<string, string | undefined>,
): string {
  const clean = String(rel || '').replace(/^\.?\/+/, '')
  const segs = clean
    .split('/')
    .filter(Boolean)
    .map(encodeURIComponent)
    .join('/')
  const base = `${assetOrigin}${encodeURIComponent(pluginId)}/${segs}`
  const params = new URLSearchParams()
  for (const [k, v] of Object.entries(query ?? {})) {
    if (v !== undefined && v !== '') params.set(k, v)
  }
  const qs = params.toString()
  return qs ? `${base}?${qs}` : base
}
