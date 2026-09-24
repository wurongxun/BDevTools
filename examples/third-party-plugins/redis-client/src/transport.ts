// Redis 传输层：UI / client 只依赖 RedisTransport 接口，不关心底层是内存模拟还是真实 TCP。
// 要接真实 Redis，只需实现一个新的 RedisTransport 子类并在 transports 注册，UI 切换 connection.mode 即可。

import { MockEngine } from './engine'
import { bridge, reserved } from './bridge'
import type { Reply, TransportMode, ConnectionConfig } from './types'

export interface ConnectResult {
  ok: boolean
  mode: TransportMode
  id: string
}

export abstract class RedisTransport {
  abstract connect(cfg: ConnectionConfig): Promise<ConnectResult>
  abstract disconnect(): void
  abstract isConnected(): boolean
  abstract exec(args: string[]): Promise<Reply>
}

// ---------------------------------------------------------- P0/P1：内存模拟传输
export class MockRedisTransport extends RedisTransport {
  private engine: MockEngine | null = null
  private connected = false

  async connect(cfg: ConnectionConfig): Promise<ConnectResult> {
    this.engine = new MockEngine()
    if (cfg.seed) this.engine.loadSeedDemo()
    this.connected = true
    return { ok: true, mode: 'mock', id: cfg.id || 'mock' }
  }

  disconnect(): void {
    this.engine = null
    this.connected = false
  }

  isConnected(): boolean {
    return this.connected
  }

  exec(args: string[]): Promise<Reply> {
    return new Promise((resolve, reject) => {
      if (!this.connected || !this.engine) {
        return reject(new Error('未连接到 Redis（mock 模式请先「连接」）'))
      }
      try {
        resolve(this.engine.execute(args))
      } catch (e) {
        reject(e)
      }
    })
  }
}

// ---------------------------------------------------------- 真实 TCP 传输（插件自带 WASM 后端）
// 沙箱页面不能开 socket，**插件自己也不能**——真实连接由本插件自带的 WASM 后端建立：它经宿主
// 提供的 `net:` 网络访问授权（原始 socket 出口）收发字节，并**自己实现 RESP 协议**（源码见 `../_guest/redis-client`，
// 编译产物即包内的 `plugin.wasm`）。本传输层只把「动作」经桥发给 WASM 的 `handle`，业务代码零改动。
//
// 动作映射（P2 分析动作由本层翻译成后端动作名，避免污染 client 层）：
//   exec 普通命令        -> { action:'exec',           connId, args }
//   RCANALYZE            -> { action:'analyze_memory', connId }
//   RCSLOW [n]           -> { action:'slowlog',        connId, count }
//   RCCLUSTER            -> { action:'cluster',        connId }
//   RCREPL               -> { action:'replication',    connId }
//   RCSENTINEL [sub]     -> { action:'sentinel',       connId, sub, name }
export class WasmRedisTransport extends RedisTransport {
  private connId: string | null = null
  private connected = false

  /**
   * 调一次 WASM 后端动作，解出统一信封 `{ ok, message, data }`；`ok !== true` 即抛错。
   * 返回的是信封里的 `data`（形状与 MockEngine 一致：exec 是 Reply，分析是分析对象）。
   */
  private async call(action: string, payload: Record<string, unknown> = {}): Promise<any> {
    const res = await bridge.call('handle', { action, connId: this.connId, ...payload })
    const env = JSON.parse(res) as { ok?: boolean; message?: string; data?: any }
    if (!env || env.ok !== true) {
      throw new Error(env && env.message ? env.message : 'WASM 后端返回异常')
    }
    return env.data
  }

  async connect(cfg: ConnectionConfig): Promise<ConnectResult> {
    this.connId = cfg.id
    const data = await this.call('connect', {
      host: cfg.host,
      port: cfg.port,
      password: cfg.password || '',
      db: cfg.db ?? 0
    })
    this.connected = !!(data && data.ok === true)
    return { ok: this.connected, mode: 'tcp', id: cfg.id }
  }

  disconnect(): void {
    if (this.connId) {
      // 断开是「尽力而为」：失败也不阻塞前端状态复位
      this.call('disconnect').catch(() => undefined)
    }
    this.connected = false
    this.connId = null
  }

  isConnected(): boolean {
    return this.connected
  }

  async exec(args: string[]): Promise<Reply> {
    if (!this.connected || !this.connId) {
      throw new Error('未连接到 Redis（tcp 模式请先「连接」）')
    }
    const head = (args[0] || '').toUpperCase()
    let action = 'exec'
    let payload: Record<string, unknown> = { args }
    if (head === 'RCANALYZE') {
      action = 'analyze_memory'
      payload = {}
    } else if (head === 'RCSLOW') {
      action = 'slowlog'
      payload = { count: parseInt(args[1] || '20', 10) }
    } else if (head === 'RCCLUSTER') {
      action = 'cluster'
      payload = {}
    } else if (head === 'RCREPL') {
      action = 'replication'
      payload = {}
    } else if (head === 'RCSENTINEL') {
      action = 'sentinel'
      payload = { sub: args[1] || 'MASTERS', name: args[2] || '' }
    }
    return (await this.call(action, payload)) as Reply
  }
}

// ---------------------------------------------------------- 预留：伴随代理传输（方案 B）
export class ProxyRedisTransport extends RedisTransport {
  private connected = false

  async connect(_cfg: ConnectionConfig): Promise<ConnectResult> {
    throw reserved('ProxyRedisTransport.connect', '需 wasm 后端 + net:<proxy-host> 权限，访问本地 RESP→HTTP 代理')
  }

  disconnect(): void {
    this.connected = false
  }

  isConnected(): boolean {
    return this.connected
  }

  exec(): Promise<Reply> {
    throw reserved('ProxyRedisTransport.exec', '见 ProxyRedisTransport.connect 注释')
  }
}

export const transports: Record<TransportMode, () => RedisTransport> = {
  mock: () => new MockRedisTransport(),
  tcp: () => new WasmRedisTransport(),
  proxy: () => new ProxyRedisTransport()
}

export const DefaultTransport: TransportMode = 'mock'
