// 宿主桥：纯前端插件只通过 parent.postMessage 与宿主通信。
// P0/P1 不依赖 WASM 后端，但 RC.bridge.call() 已就绪，供后期真实传输 / KV 持久化接入。

// v2：`theme` 合并进 `context`（多了 `locale`）。宿主兼容 v1——用哪个版本发 ready 就用哪个
// 版本回，但 v1 应答里没有 `locale`。
export const BRIDGE_VERSION = 2 // 必须与宿主 PLUGIN_BRIDGE_VERSION 一致

type Listener = (payload: any) => void

class Bridge {
  runtimeWasm: boolean | null = null
  /** 宿主当前语言（context.locale）；插件页自己带字典时按它切换 */
  locale: string | null = null
  private seq = 0
  private pending: Record<number, { resolve: (v: any) => void; reject: (e: any) => void }> = {}
  private listeners: Record<string, Listener[]> = { context: [], theme: [], ready: [] }

  post(msg: Record<string, any>): void {
    msg.v = BRIDGE_VERSION
    parent.postMessage(msg, '*')
  }

  call(func: string, input: any): Promise<string> {
    return new Promise((resolve, reject) => {
      const reqId = ++this.seq
      this.pending[reqId] = { resolve, reject }
      this.post({ kind: 'call', reqId, func, input })
    })
  }

  on(evt: string, fn: Listener): void {
    ;(this.listeners[evt] || (this.listeners[evt] = [])).push(fn)
  }

  handleMessage(data: any): void {
    if (!data || data.v !== BRIDGE_VERSION) return
    // v1 页面（还活在旧协议上的插件）收到的是 theme 消息，这里一并兼容
    if (data.kind === 'theme') {
      this.runtimeWasm = data.wasm === true
      this.emit('theme', data)
      return
    }
    if (data.kind === 'context') {
      this.runtimeWasm = data.wasm === true
      this.locale = typeof data.locale === 'string' ? data.locale : null
      this.emit('context', data)
      // 同时发 theme：老订阅方不必改一次全部改完
      this.emit('theme', data)
      return
    }
    if (typeof data.reqId === 'number') {
      const p = this.pending[data.reqId]
      if (!p) return
      delete this.pending[data.reqId]
      if (data.ok) p.resolve(data.data)
      else p.reject(new Error(data.error))
    }
  }

  ready(): void {
    this.post({ kind: 'ready' })
  }

  private emit(evt: string, payload: any): void {
    ;(this.listeners[evt] || []).forEach((f) => f(payload))
  }
}

export const bridge = new Bridge()

// 持久化抽象：不透明源下 localStorage 不可用，连接配置走抽象层。
export interface Persistence {
  get(k: string): any
  set(k: string, v: any): void
  remove(k: string): void
  keys(): string[]
}

export class MemoryPersistence implements Persistence {
  private m: Record<string, any> = {}
  get(k: string): any {
    return this.m[k]
  }
  set(k: string, v: any): void {
    this.m[k] = v
  }
  remove(k: string): void {
    delete this.m[k]
  }
  keys(): string[] {
    return Object.keys(this.m)
  }
}

// 预留：KV 持久化（需 wasm 后端 + 'kv' 权限）。接入时替换下面的实例。
export class KvPersistence implements Persistence {
  available = false
  get(): any {
    throw reserved('KvPersistence.get', '需 wasm 后端 + kv 权限')
  }
  set(): void {
    throw reserved('KvPersistence.set', '需 wasm 后端 + kv 权限')
  }
  remove(): void {
    throw reserved('KvPersistence.remove', '需 wasm 后端 + kv 权限')
  }
  keys(): string[] {
    return []
  }
}

export const persistence: Persistence = new MemoryPersistence()
export const persistenceMode = 'memory' // 'memory' | 'kv'(预留)

/**
 * 预留接口标记工具：任何「计划后期实现」的扩展点都抛这个错误，便于全局检索。
 * 后期直接删除 throw 并实现逻辑即可。
 */
export function reserved(name: string, detail?: string): Error {
  const msg = `[redis-client] 预留接口尚未实现: ${name}${detail ? ' — ' + detail : ''}`
  if (typeof console !== 'undefined' && console.warn) console.warn(msg)
  const err: any = new Error(msg)
  err.reserved = true
  err.feature = name
  return err
}
