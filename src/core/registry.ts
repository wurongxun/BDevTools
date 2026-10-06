/**
 * 通用贡献点注册表（宿主侧唯一事实源）。
 *
 * ## 它解决什么
 *
 * UI 不再认识「插件」这个具体形状，只认识**贡献条目**：活动栏要图标就查 `activityBarItem`，
 * 主区要面板就查 `view`。插件怎么组织自己的代码是插件的事，宿主只按 kind 取用。
 *
 * 这条边界是「契约高于实现」的落点 —— 插件之间互不知道对方存在，因为它们都只跟注册表说话。
 *
 * ## 它不解决什么
 *
 * - **不负责可见性**：「这个窗口显示哪些插件」是窗口策略（见 `store/modules/layout.ts`
 *   的 `enabledPlugins`），由调用方过滤，注册表只回答「系统里有什么」。
 * - **不负责排序权重的语义**：顺序由 `pluginManager.applyOrder` 同步进来（注册表不认识
 *   `pluginManager`，保持单向依赖）。
 * - **不负责懒加载**：组件现在是直接引用，期 3 的 `activationEvents` 会把它换成按需加载
 *   的包装，那时条目形状不变。
 *
 * ## 为什么用「整组注册 / 整组撤销」而不是逐条增删
 *
 * 一个插件的条目是一个整体：插件停用时必须**全部**消失，漏一条就会留下点不动的孤儿图标。
 * 以插件为粒度撤销，就没有「删一半」的中间态。
 */
import { ref, type Component } from 'vue'

/** 宿主提供的扩展点种类（新增一种 = 宿主多认一种贡献） */
export type ContributionKind = 'activityBarItem' | 'view' | 'command' | 'statusBarItem'

/**
 * 视图槽位：`view` 条目的分区。
 *
 * 槽位是宿主定的（主区 / 侧栏 / 停靠 / Tab 栏），插件只声明「我往这个槽位放一个组件」，
 * 不关心布局怎么排。
 */
export type ViewSlot = 'sidebar' | 'main' | 'dock' | 'tabBar'

/** 一条贡献 */
export interface Contribution<TPayload = unknown> {
  /** 条目 ID（kind 内唯一；约定 `<pluginId>:<局部名>`） */
  id: string
  kind: ContributionKind
  /** 贡献者（撤销时按它整组移除） */
  pluginId: string
  payload: TPayload
}

/** 活动栏图标 */
export interface ActivityBarItemPayload {
  /** 图标组件（单色 SVG 必须自带 `fill="currentColor"`） */
  icon: Component
  /** 无障碍名称与 tooltip 的基础文案 */
  label: string
  /** 是否固定在活动栏底部（宿主常驻入口，如「设置」） */
  pinnedBottom: boolean
  /** 停靠面板位置：有值时图标是「显隐开关」而不是「切主面板」 */
  dockPosition?: 'bottom'
  /** 独立窗口型插件：图标是「打开/前置窗口」 */
  standaloneWindow?: boolean
}

/** 视图（面板 / Tab 栏） */
export interface ViewPayload {
  /** 宿主槽位 */
  slot: ViewSlot
  /** 面板组件 */
  component: Component
  /** 仅 dock 槽位：停靠位置 */
  position?: 'bottom'
  /** 仅 dock 槽位：默认高度（px） */
  defaultHeight?: number
  /** 仅 dock 槽位：最小高度（px） */
  minHeight?: number
  /** 仅 sidebar 槽位：宽度约束（未声明由宿主用全局缺省值） */
  minWidth?: number
  maxWidth?: number
  defaultWidth?: number
}

/** 命令（命令面板 / 快捷键 / 菜单共用） */
export interface CommandPayload {
  /** 命令 ID（全局唯一，约定 `<pluginId>.<name>`） */
  command: string
  /** 命令面板里显示的标题 */
  title: string
  /** 分组（命令面板按它归类展示） */
  category?: string
  /**
   * 是否当下可用（返回 false 的命令在面板里隐藏）。
   * 用于「没有打开的连接时，断开连接命令不该出现」这类场景。
   */
  when?: () => boolean
  /** 执行体 */
  run: () => void | Promise<void>
}

/** 便捷类型：各类贡献的条目形态 */
export type ActivityBarItem = Contribution<ActivityBarItemPayload>
export type ViewItem = Contribution<ViewPayload>
export type CommandItem = Contribution<CommandPayload>

/** 未纳入统一排序的插件权重基准（保证它们排在已排序插件之后） */
const DEFAULT_ORDER_BASE = 1000

/**
 * 贡献点注册表。
 *
 * 查询返回的是**新数组**：调用方可以随意 filter / map，不会动到注册表内部状态。
 */
class ContributionRegistry {
  private _entries = ref<Contribution[]>([])

  /** 插件顺序：`pluginId → 序号`，由 `pluginManager.applyOrder` 同步 */
  private _pluginOrder = ref<Record<string, number>>({})

  /** 条目注册序号（同一插件内按注册顺序稳定排序） */
  private _seq = new Map<string, number>()
  private _seqCounter = 0

  /** 变更计数：供「不依赖具体条目」的场景订阅（如命令面板刷新） */
  version = ref(0)

  private rank(entry: Contribution): number {
    const idx = this._pluginOrder.value[entry.pluginId]
    const base = idx !== undefined ? idx : DEFAULT_ORDER_BASE
    // 插件序号占高位，条目序号占低位：同插件的条目聚在一起且保持注册顺序
    return base * 1000 + (this._seq.get(`${entry.kind}:${entry.id}`) ?? 0)
  }

  /**
   * 注册一个插件的全部贡献（**先整组撤销再写入**，重复调用幂等）。
   * @param pluginId 贡献者
   * @param items 条目（不需要带 pluginId）
   */
  contribute(
    pluginId: string,
    items: Array<{ kind: ContributionKind; id: string; payload: unknown }>,
  ): void {
    this.withdraw(pluginId)
    const next = this._entries.value.filter(e => e.pluginId !== pluginId)
    for (const it of items) {
      const entry: Contribution = { id: it.id, kind: it.kind, pluginId, payload: it.payload }
      this._seq.set(`${it.kind}:${it.id}`, this._seqCounter++)
      next.push(entry)
    }
    this._entries.value = next
    this.version.value++
  }

  /**
   * 追加条目（**不**撤销已有条目），用于运行期动态注册（典型：命令）。
   *
   * 与 `contribute` 的区别就是这一条：`contribute` 是「一个插件的完整声明」，重复调用要幂等；
   * 追加则是「在已有声明上再加几条」。两者写入的条目在同一个池子里，`withdraw` 时一起消失。
   */
  add(
    pluginId: string,
    items: Array<{ kind: ContributionKind; id: string; payload: unknown }>,
  ): void {
    const next = [...this._entries.value]
    for (const it of items) {
      const key = `${it.kind}:${it.id}`
      if (next.some(e => e.kind === it.kind && e.id === it.id)) continue
      this._seq.set(key, this._seqCounter++)
      next.push({ id: it.id, kind: it.kind, pluginId, payload: it.payload })
    }
    this._entries.value = next
    this.version.value++
  }

  /** 撤销某插件的全部贡献（插件停用 / 卸载时调用） */
  withdraw(pluginId: string): void {
    const before = this._entries.value.length
    this._entries.value = this._entries.value.filter(e => e.pluginId !== pluginId)
    if (this._entries.value.length !== before) this.version.value++
  }

  /** 同步插件顺序（ID 序列，越靠前序号越小） */
  setPluginOrder(ids: string[]): void {
    const next: Record<string, number> = {}
    ids.forEach((id, i) => {
      next[id] = i
    })
    this._pluginOrder.value = next
    this.version.value++
  }

  /** 查询某类贡献（按插件顺序 + 注册顺序排序） */
  query<TPayload = unknown>(
    kind: ContributionKind,
    opts?: { slot?: ViewSlot; pluginId?: string },
  ): Contribution<TPayload>[] {
    const filtered = this._entries.value.filter(e => {
      if (e.kind !== kind) return false
      if (opts?.pluginId && e.pluginId !== opts.pluginId) return false
      if (opts?.slot) {
        const payload = e.payload as ViewPayload
        if (payload?.slot !== opts.slot) return false
      }
      return true
    })
    return [...filtered].sort((a, b) => this.rank(a) - this.rank(b)) as Contribution<TPayload>[]
  }

  /** 取单条（未找到返回 undefined） */
  get<TPayload = unknown>(kind: ContributionKind, id: string): Contribution<TPayload> | undefined {
    return this._entries.value.find(e => e.kind === kind && e.id === id) as
      | Contribution<TPayload>
      | undefined
  }

  /** 某槽位的视图条目（语法糖） */
  views<TPayload = ViewPayload>(slot: ViewSlot): Contribution<TPayload>[] {
    return this.query<TPayload>('view', { slot })
  }

  /** 活动栏条目（语法糖） */
  activityItems(): ActivityBarItem[] {
    return this.query<ActivityBarItemPayload>('activityBarItem')
  }

  /** 命令条目（语法糖） */
  commands(): CommandItem[] {
    return this.query<CommandPayload>('command')
  }
}

/** 全局单例 */
export const contributionRegistry = new ContributionRegistry()
