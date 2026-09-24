import { load } from '@/core/runtime'
import type { Store } from '@/core/runtime'

/**
 * `settings.json` 句柄（设置页内共享）。
 *
 * 设置页拆成多个分区面板后，「应用设置」和「窗口设置」都要读写同一个文件。
 * `load()` 本身是无状态的 RPC 薄封装（后端即时读写磁盘、前端不缓存），
 * 所以多建几个句柄并不会互相覆盖；这里仍然只取一次，纯粹是为了不重复握手。
 *
 * 失败不缓存：否则一次网络/后端抖动会把 rejected 的 Promise 永久钉死在这里。
 */
let handle: Promise<Store> | null = null

export function useSettingsStore(): Promise<Store> {
  if (!handle) {
    handle = load('settings.json', { autoSave: false, defaults: {} }).catch((e: unknown) => {
      handle = null
      throw e
    })
  }
  return handle
}
