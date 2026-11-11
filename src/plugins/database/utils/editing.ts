/**
 * 结果网格编辑的**纯逻辑层**（不依赖 Vue / DOM，便于单独跑仿真验证）。
 *
 * 这里集中三件容易做错的事：
 * 1. **行定位**（`resolveRowKeyStrategy`）：主键 → 唯一索引 → 用户指定的虚拟键
 *    → 整行原值匹配（兜底）→ 放弃。前两种是「数据库担保唯一」，后两种是
 *    「我们担保不了」，所以后两种必须配 `expect_one`：命中数不是 1 就整批回滚。
 * 2. **行身份**（`makeRowKey`）：必须用**数据本身**算出来的 key，不能用数组下标。
 *    翻页 / 重查 / 排序 / 客户端过滤都会让下标整体平移，脏数据会挂到另一行上。
 * 3. **NULL 与空串的区分**：网格里两者显示成同一个占位符，用户改不出 NULL，也改不回空串。
 *    所以引入显式的 `SQL_NULL` 标记，代表「我要写 NULL」。
 */
import type { ColumnInfo, IndexInfo } from '../types'

/** 「写 NULL」标记。不能用 `null` 本身 —— 它与「单元格原值是 null、用户没动」无法区分 */
export interface SqlNullMarker {
  readonly __sqlNull: true
}

export const SQL_NULL: SqlNullMarker = Object.freeze({ __sqlNull: true }) as SqlNullMarker

/** 单元格的输入：原始标量（string / number / boolean / null）或 NULL 标记 */
export type CellInput = unknown

export function isSqlNull(v: unknown): v is SqlNullMarker {
  return typeof v === 'object' && v !== null && (v as { __sqlNull?: boolean }).__sqlNull === true
}

/** 单元格展示文本的上限：一列里存了几 MB 的 JSON 时，原样铺到 DOM 里会把整张表拖死 */
const MAX_DISPLAY_CHARS = 300

/**
 * 单元格的展示文本。**网格里所有地方都必须用它**，否则会出现「NULL 和空串长得一样，
 * 但改的时候行为不同」的割裂。
 *
 * `[NULL]` / `[空字符串]` 是刻意的区分：两者在库里是两个不同的东西。
 *
 * 超长值只显示开头一小段并标出总长度 —— 完整值在双击编辑时仍然是完整的
 * （编辑器加载的是原始值，不是这里的展示文本）。
 */
export function cellDisplayText(v: unknown): string {
  if (v === null || v === undefined) return '[NULL]'
  if (v === '') return '[空字符串]'
  let s: string
  if (typeof v === 'object') {
    try {
      s = JSON.stringify(v)
    } catch {
      s = String(v)
    }
  } else {
    s = String(v)
  }
  if (s.length <= MAX_DISPLAY_CHARS) return s
  return `${s.slice(0, MAX_DISPLAY_CHARS)}…（共 ${s.length} 字符）`
}

/** 转成后端要的值：NULL 标记 → null，其余原样 */
export function toWireValue(v: CellInput): unknown {
  return isSqlNull(v) ? null : v
}

/**
 * 「这一格**现在**该显示什么」：脏集合里有待写值就用它，否则用行里的原值。
 *
 * 刻意做成纯函数放在这里，而不写在组件里：
 *   ① 它是「脏值不混进整份数据」这条约束的唯一实现，仿真能直接跑源码验证；
 *   ② 调用点只有单元格插槽一个 ⇒ 只有那一格订阅了 dirtyCells，
 *      编辑时不会出现「改一格 ⇒ 重建整份 rows ⇒ 表格全量重算」。
 */
export function pickCellValue(
  row: Record<string, any>,
  key: string,
  colIdx: number,
  col: string | undefined,
  dirty: Map<string, Map<string, CellInput>>
): CellInput {
  if (col === undefined) return row[String(colIdx)]
  const pending = dirty.get(key)?.get(col)
  return pending === undefined ? row[String(colIdx)] : pending
}

/** 转成数字用于比较；不是数字形态返回 null */
function numericOrNull(v: unknown): number | null {
  if (typeof v === 'number') return Number.isFinite(v) ? v : null
  if (typeof v !== 'string') return null
  const s = v.trim()
  if (s === '') return null
  const n = Number(s)
  return Number.isNaN(n) ? null : n
}

/**
 * 两个值在「写库」意义下是否相同（用于判断某格是不是真的改过）。
 *
 * **只有数值列才按数值比**：那里 `7` 与 `'007'` 落到库里是同一个值，报成「已修改」
 * 纯属噪音。字符串列必须按字符串比 —— `'007'` 改成 `'7'` 是实打实的改动，
 * 用数值比会被悄悄吃掉。
 */
export function sameValue(a: CellInput, b: unknown, kind: ValueKind = 'text'): boolean {
  if (isSqlNull(a)) return b === null || b === undefined
  if (a === null || a === undefined) return b === null || b === undefined
  if (b === null || b === undefined) return false
  if (kind === 'number') {
    const na = numericOrNull(a)
    const nb = numericOrNull(b)
    if (na !== null && nb !== null) return na === nb
  }
  return String(a) === String(b)
}

// ===================================================================
// 行定位策略
// ===================================================================

export type RowKeyStrategy =
  | { kind: 'primary'; cols: string[] }
  | { kind: 'unique'; cols: string[]; indexName: string }
  | { kind: 'virtual'; cols: string[] }
  | { kind: 'full'; cols: string[]; excluded: ExcludedColumn[] }
  | { kind: 'none'; reason: string }

/** 整行匹配时被排除的列及其原因（要能给用户解释「为什么没拿它定位」） */
export interface ExcludedColumn {
  col: string
  reason: string
}

/**
 * 行定位的可选项。**都不填时行为与以前完全一致**（主键 → 唯一索引 → 只读），
 * 也就是说「无身份表可编辑」是显式开启的能力，不会偷跑到既有表上。
 */
export interface RowKeyOptions {
  /** 用户为这张表指定的行标识（虚拟键 / 伪主键），多列按 AND 拼 */
  virtualKey?: string[]
  /** 允许「整行原值匹配」兜底：无主键、无唯一索引、也没指定虚拟键时才走到这里 */
  allowFullMatch?: boolean
}

/**
 * 这类定位手段**没有数据库担保唯一** ⇒ 执行时必须要求「恰好命中一行」，
 * 否则整批回滚（`expect_one` 由后端执行）。
 */
export function needsExactOneMatch(strategy: RowKeyStrategy): boolean {
  return strategy.kind === 'virtual' || strategy.kind === 'full'
}

/** 给用户看的一句定位说明（只读原因、提示条都要用） */
export function describeRowKey(strategy: RowKeyStrategy): string {
  switch (strategy.kind) {
    case 'primary':
      return `主键（${strategy.cols.join('、')}）`
    case 'unique':
      return `唯一索引 ${strategy.indexName}（${strategy.cols.join('、')}）`
    case 'virtual':
      return `手动指定的行标识（${strategy.cols.join('、')}）—— 唯一性由你保证，保存时会校验命中 1 行`
    case 'full':
      return `整行原值匹配（${strategy.cols.length} 列）—— 保存时校验命中 1 行，否则回滚`
    default:
      return ''
  }
}

/** 结果列名 → 表列名的大小写不敏感映射（不同库返回的列名大小写政策不一样） */
function buildColumnMap(resultColumns: string[], tableCols: ColumnInfo[]): Map<string, ColumnInfo> {
  const map = new Map<string, ColumnInfo>()
  for (const col of tableCols) {
    map.set(col.name.toLowerCase(), col)
  }
  // 结果列名优先用精确匹配，其次大小写不敏感
  const out = new Map<string, ColumnInfo>()
  for (const name of resultColumns) {
    const byExact = tableCols.find(c => c.name === name)
    out.set(name, byExact ?? map.get(name.toLowerCase()) ?? (undefined as unknown as ColumnInfo))
  }
  return out
}

/** 结果列名 → 对应的表元数据（找不到为 undefined：表达式列 / 跨表列） */
export function mapResultColumns(
  resultColumns: string[],
  tableCols: ColumnInfo[]
): Map<string, ColumnInfo | undefined> {
  const m = buildColumnMap(resultColumns, tableCols)
  const out = new Map<string, ColumnInfo | undefined>()
  for (const name of resultColumns) {
    const v = m.get(name)
    out.set(name, v ?? undefined)
  }
  return out
}

/**
 * 决定用哪些列来唯一定位一行。
 *
 * 优先级：
 * 1. 主键（支持复合主键 —— 之前只取第一列，复合主键会匹配到多行）
 * 2. 单列唯一索引（只接受**单列**：多列唯一索引若只取其中一列并不唯一）
 * 3. 没有 ⇒ 不可编辑
 *
 * 注意：结果必须**包含**这些键列，否则 WHERE 里拼不出值。
 */
export function resolveRowKeyStrategy(
  resultColumns: string[],
  tableCols: ColumnInfo[],
  indexes: IndexInfo[] = [],
  opts: RowKeyOptions = {}
): RowKeyStrategy {
  const metaOf = (name: string): ColumnInfo | undefined =>
    tableCols.find(c => c.name.toLowerCase() === name.toLowerCase())
  /** 元数据里没有 = 不是这张表的列；结果里没有 = 拼不出 WHERE 的值 */
  const usable = (name: string) =>
    !!metaOf(name) && resultColumns.some(c => String(c).toLowerCase() === name.toLowerCase())

  /** 去重：同一列可能既被标成主键又出现在 PRIMARY 索引里 */
  const dedupe = (names: string[]) => {
    const seen = new Set<string>()
    const out: string[] = []
    for (const n of names) {
      const k = n.toLowerCase()
      if (seen.has(k)) continue
      seen.add(k)
      out.push(n)
    }
    return out
  }

  // 主键有**两个来源**，都要看：
  // 1. 列元数据的 `is_primary_key`（MySQL 的 COLUMN_KEY=PRI 之类）
  // 2. 索引里的 `PRIMARY`
  // 有些库 / 版本只给得其中一个（见过 DDL 明明有 PRIMARY KEY、列元数据却全是 false 的），
  // 只认一处就会把能编辑的表判成只读。
  const flagged = tableCols.filter(c => c.is_primary_key).map(c => c.name)
  const fromIndex = indexes.filter(i => i.is_primary).map(i => i.column_name)
  const pk = dedupe(flagged.length > 0 ? flagged : fromIndex)
  if (pk.length > 0 && pk.every(usable)) {
    return { kind: 'primary', cols: pk }
  }

  // 唯一索引：整组列拼起来是唯一的 ⇒ **复合唯一索引同样能定位一行**（早先只收单列，
  // 白白放弃了 `UNIQUE (a, b)` 这种完全可用的身份）。
  // 但**可空列上的唯一索引不可信**：MySQL / PG 允许唯一索引里有多行 NULL，
  // `col IS NULL` 会一次命中多行 —— 那不是编辑，是事故。
  const groups = new Map<string, string[]>()
  for (const idx of indexes) {
    if (!idx.is_unique || idx.is_primary) continue
    const arr = groups.get(idx.name) ?? []
    arr.push(idx.column_name)
    groups.set(idx.name, arr)
  }
  const candidates = [...groups.entries()]
    .map(([name, cols]) => ({ name, cols: dedupe(cols) }))
    .filter(c => c.cols.length > 0 && c.cols.every(usable))
    .filter(c => c.cols.every(n => metaOf(n)?.is_nullable === false))
    .sort((a, b) => a.cols.length - b.cols.length)
  const best = candidates[0]
  if (best) return { kind: 'unique', cols: best.cols, indexName: best.name }

  // 3. 用户手动指定的行标识（虚拟键 / 伪主键）。
  //    唯一性是**用户声明**的，不是数据库担保的 —— 所以它由 `needsExactOneMatch`
  //    挂上「命中必须 1 行」的校验，挑错了列也不会误伤一片。
  const wanted = opts.virtualKey ?? []
  if (wanted.length > 0 && wanted.every(usable)) {
    return { kind: 'virtual', cols: dedupe(wanted) }
  }

  // 4. 兜底：整行的原值当键（DataGrip / Navicat 的做法）。
  //    这是**最后**的手段：只有前三种都用不上、且调用方明确允许时才走，
  //    并且同样靠「命中必须 1 行」兜住 —— 撞上重复行只会失败，不会误改。
  if (opts.allowFullMatch) {
    const { cols, excluded } = matchableColumns(resultColumns, tableCols)
    if (cols.length > 0) return { kind: 'full', cols, excluded }
  }

  // 理由要能自查：把「主键/索引到底读到什么」说清楚，别只丢一句「无法定位」
  const parts: string[] = []
  if (pk.length === 0) {
    parts.push('元数据里没有主键标记')
  } else {
    parts.push(`主键列 ${pk.filter(n => !usable(n)).join('、')} 不在结果集中`)
  }
  parts.push(
    groups.size === 0
      ? '也没有唯一索引'
      : '唯一索引的列不在结果集、或允许 NULL（多行 NULL 不唯一）'
  )
  if (opts.allowFullMatch) parts.push('也没有可用于整行匹配的列')
  // 可操作的下一步：不然用户看到「无法定位」只能干瞪眼
  const hint = '可在列头右键「设为行标识」指定一列'
  return {
    kind: 'none',
    reason: `无法唯一定位行：${parts.join('，')}（已读 ${tableCols.length} 列 / ${indexes.length} 条索引；${hint}）`,
  }
}

/** 能安全写进 `WHERE` 的字符列长度上限：再长就别进条件了（几 KB 的 WHERE 既慢又难读） */
const MATCHABLE_MAX_CHARS = 1000
/** 浮点回显的文本与库内二进制值常常对不上（`0.1` 的千年老问题）⇒ 一律不进 WHERE */
const FLOAT_HINTS = ['float', 'double', 'real']

/**
 * 这一列为什么**不能**用来定位一行；返回 null 表示可以用。
 *
 * 整行匹配的 WHERE 是把「所有列的原值」拼起来，所以每一列都必须能可靠地相等比较：
 * 比不出来（JSON 写法不唯一）、比不准（浮点）、比不动（二进制 / 超长文本）的列都得排除。
 * 排除是**按列**决定的，不是按值 —— 否则同一列在不同行里有时进 WHERE 有时不进，
 * 行的身份就会随数据漂移。
 */
export function unmatchableReason(col: ColumnInfo | undefined): string | null {
  if (!col) return '不属于这张表'
  if (col.is_generated) return '生成列（值由数据库算）'
  const t = (col.data_type || '').toLowerCase()
  if (BINARY_HINTS.some(h => t.includes(h))) return '二进制列'
  if (FLOAT_HINTS.some(h => t.includes(h))) return '浮点列（回显值与库内值可能不相等）'
  if (t.includes('json')) return 'JSON 列（写法不唯一，比不出来）'
  if ((col.character_max_length ?? 0) > MATCHABLE_MAX_CHARS) return `超长列（>${MATCHABLE_MAX_CHARS} 字符）`
  return null
}

/**
 * 整行兜底时可用作 `WHERE` 的列（**结果列的子集**，按结果列顺序）。
 *
 * 返回的 `excluded` 要展示给用户：被排除的列意味着「这一列的值改了也不会用来定位」，
 * 剩下能用的列越少，撞上重复行被拒的概率越高 —— 这必须让用户知道，而不是默默发生。
 */
export function matchableColumns(
  resultColumns: string[],
  tableCols: ColumnInfo[]
): { cols: string[]; excluded: ExcludedColumn[] } {
  const byLower = new Map(tableCols.map(c => [c.name.toLowerCase(), c]))
  const cols: string[] = []
  const excluded: ExcludedColumn[] = []
  for (const name of resultColumns) {
    const meta = tableCols.find(c => c.name === name) ?? byLower.get(String(name).toLowerCase())
    const reason = unmatchableReason(meta)
    if (reason) excluded.push({ col: String(name), reason })
    else cols.push(String(name))
  }
  return { cols, excluded }
}

/**
 * 行 key：由**键列的原值**算出，跨翻页 / 重查 / 排序稳定。
 * 无定位策略（或键值为空）时退化成下标，调用方必须据此判定为只读。
 *
 * 主键 / 唯一索引里出现 NULL ⇒ 算不出身份（`col IS NULL` 会一次命中多行），直接放弃。
 * 而虚拟键 / 整行匹配**允许 NULL**：这两种定位本来就不由数据库担保唯一，
 * NULL 写成 `IS NULL` 参与匹配，能不能定位到唯一一行交给 `expect_one` 在运行时判定
 * —— 判不出来就是失败并回滚，不会变成误改。
 */
export function makeRowKey(keyValues: unknown[], strategy: RowKeyStrategy): string {
  if (strategy.kind === 'none') return ''
  const vals = keyValues.map(v => (v === null || v === undefined ? null : String(v)))
  if (!needsExactOneMatch(strategy) && vals.some(v => v === null)) return ''
  return `${strategy.kind}:${JSON.stringify(vals)}`
}

// ===================================================================
// 列的可编辑性与编辑器类型
// ===================================================================

export type ValueKind =
  | 'text'
  | 'number'
  | 'bool'
  | 'date'
  | 'datetime'
  | 'time'
  | 'longtext'
  | 'binary'

const BINARY_HINTS = ['blob', 'binary', 'varbinary', 'bytea', 'image', 'raw', 'geometry', 'geography']
const LONGTEXT_HINTS = ['text', 'json', 'xml', 'clob', 'ntext', 'memo', 'longtext']
const NUMBER_HINTS = [
  'int', 'serial', 'smallint', 'bigint', 'tinyint', 'mediumint', 'integer',
  'decimal', 'numeric', 'dec', 'float', 'double', 'real', 'money', 'number',
]
const DATETIME_HINTS = ['datetime', 'timestamp', 'smalldatetime', 'datetime2']
const BOOL_HINTS = ['bool', 'boolean', 'bit']

/** 依据列元数据推断该用哪种编辑器 */
export function inferValueKind(col?: ColumnInfo): ValueKind {
  if (!col) return 'text'
  const t = (col.data_type || '').toLowerCase()
  if (BINARY_HINTS.some(h => t.includes(h))) return 'binary'
  if (BOOL_HINTS.includes(t) || t === 'tinyint(1)') return 'bool'
  if (DATETIME_HINTS.some(h => t.includes(h))) return 'datetime'
  if (t === 'date') return 'date'
  if (t === 'time') return 'time'
  if (LONGTEXT_HINTS.some(h => t.includes(h))) return 'longtext'
  // 长度很大的字符列也按长文本处理（单行输入框里编辑一段 JSON 是灾难）
  if ((col.character_max_length ?? 0) > 255) return 'longtext'
  if (NUMBER_HINTS.some(h => t.includes(h))) return 'number'
  return 'text'
}

export interface Editability {
  editable: boolean
  reason?: string
  kind: ValueKind
}

/** 这一列能不能写（不含「整表是否可编辑」的判断） */
export function columnEditability(col: ColumnInfo | undefined): Editability {
  const kind = inferValueKind(col)
  if (!col) return { editable: false, reason: '结果列不属于该表', kind }
  if (col.is_generated) return { editable: false, reason: '计算列（由数据库生成，不可写）', kind }
  if (kind === 'binary') return { editable: false, reason: '二进制列暂不支持编辑', kind }
  return { editable: true, kind }
}

// ===================================================================
// 客户端校验（能本地发现的问题不要等服务端报错）
// ===================================================================

/**
 * 归一化单元格输入：**把「用户清空了输入框」翻译成数据库真正接受的东西**。
 *
 * 返回 `{ value, error }`：
 * - `error` 非空 ⇒ 不许提交，提示就地显示在编辑框里；
 * - `value` 可能与输入不同 —— 最典型的就是把数字列的空输入变成 `SQL_NULL`。
 *
 * 为什么要这一层：MySQL 严格模式下 `UPDATE t SET i = ''`（i 是 INT）直接报
 * `Incorrect integer value: ''`，PG 报 `invalid input syntax for type integer`。
 * 服务端报错用户看不懂也不知道该怎么做 —— 而「按 backspace 清空单元格」的意图
 * 从来只有一个：**把这个值清掉**。这几类列上「清掉」只有 NULL 一种写法，所以在这里
 * 翻译掉，而不是把报错推给数据库。
 */
export interface NormalizedCell {
  value: CellInput
  error: string | null
}

/**
 * 这几类列没有「空串」这个值：数字 / 布尔 / 时间 / JSON。
 * 文本列（char / varchar / text）的空串是货真价实的值，必须原样写回。
 */
function emptyStringIsInvalid(kind: ValueKind, col: ColumnInfo): boolean {
  if (kind === 'number' || kind === 'bool') return true
  if (kind === 'date' || kind === 'datetime' || kind === 'time') return true
  return (col.data_type || '').toLowerCase().includes('json')
}

/** 把这一格的输入翻译成数据库真正接受的值（见 `normalizeCellInput`） */
export function normalizeCellInput(v: CellInput, col: ColumnInfo | undefined): NormalizedCell {
  if (!col) return { value: v, error: '结果列不属于该表' }
  const kind = inferValueKind(col)
  if (isSqlNull(v)) {
    return col.is_nullable ? { value: v, error: null } : { value: v, error: '该列不允许 NULL' }
  }
  const s = v === null || v === undefined ? '' : String(v)

  if (emptyStringIsInvalid(kind, col) && s.trim() === '') {
    if (!col.is_nullable) {
      const label =
        kind === 'number' ? '数值' : kind === 'bool' ? '布尔' : (col.data_type || '').toLowerCase().includes('json') ? 'JSON' : '时间'
      return {
        value: v,
        error: `${label}列不能写成空字符串，而该列又不允许 NULL —— 请填写具体值`,
      }
    }
    return { value: SQL_NULL, error: null }
  }

  if (kind === 'number' && Number.isNaN(Number(s.trim()))) return { value: v, error: '不是合法数字' }
  const max = col.character_max_length ?? 0
  if (max > 0 && countChars(s) > max) return { value: v, error: `超出列长度上限 ${max}` }
  return { value: v, error: null }
}

/** 按字符数计长度（MySQL 的 character_max_length 是字符数不是字节数） */
function countChars(s: string): number {
  // 代理对（emoji 等）算一个字符
  let n = 0
  for (let i = 0; i < s.length; i++) {
    if (i + 1 < s.length && s.charCodeAt(i) >= 0xd800 && s.charCodeAt(i) <= 0xdbff) i++
    n++
  }
  return n
}

// ===================================================================
// 新增行（幽灵行）
//
// 「表格底部常驻一条空行，输入即转正为待插入行」是 DBeaver / DataGrip / Navicat
// 的共同做法 —— 新增行往往需要参考上面几行的样子、边滚边填，弹窗会把上下文挡掉。
//
// 这里的函数都是纯函数：新行的**唯一真源是脏集合**（键为 `__new:N`），
// 不必额外维护一份「这行准备好没有」的状态 —— 一旦某行被填上值，它就在脏集合里出现；
// 全部清空则自动消失。这条规则让「连续新增多行」和「取消某行」都不需要额外代码。
// ===================================================================

/** 新增行的 `__key` 前缀。刻意带冒号，避免与任何真实业务 ID 相撞 */
export const NEW_ROW_PREFIX = '__new:'

export function isNewRowKey(key: string | undefined | null): boolean {
  return typeof key === 'string' && key.startsWith(NEW_ROW_PREFIX)
}

export function makeNewRowKey(seq: number): string {
  return `${NEW_ROW_PREFIX}${seq}`
}

/** 反解出行号（用于按输入顺序生成 SQL，保证同一批操作每次预览都一致） */
export function newRowSeqOf(key: string): number {
  if (!isNewRowKey(key)) return -1
  const n = Number(key.slice(NEW_ROW_PREFIX.length))
  return Number.isInteger(n) ? n : -1
}

/** 脏集合的形状：`行 key → (列名 → 待写值)` */
type PendingRows = ReadonlyMap<string, ReadonlyMap<string, CellInput>>

/** 某个键眼下是不是「真的填了东西」的新行（空 Map 说明最后一格也被清掉了） */
function isCommittedNewRow(
  key: string,
  cells: ReadonlyMap<string, CellInput> | undefined
): boolean {
  return isNewRowKey(key) && (cells?.size ?? 0) > 0
}

/**
 * 下一条新增行的 key —— 也就是表格尾部那条「幽灵行」的身份。
 *
 * 只在**真的填了东西**的行之后取最大值 +1：于是用户在幽灵行里敲下第一个值的瞬间，
 * 这个 key 自动往后挪一位，一条新的空行随之出现（可以连续加多行）；
 * 反过来把某行的值全部清掉，它就退出序列，尾部又自然回落。
 */
export function nextNewRowKey(dirty: PendingRows): string {
  let max = -1
  for (const [key, cells] of dirty) {
    if (!isCommittedNewRow(key, cells)) continue
    const n = newRowSeqOf(key)
    if (n > max) max = n
  }
  return makeNewRowKey(max + 1)
}

/**
 * 已转正的新增行（按输入顺序）。幽灵行本身不在其中 —— 它还没有任何值。
 *
 * 这里刻意**同时**校验前缀与「真的有值」：只看前缀的话，一个残留的空 Map
 * 就会被当成待插入行，凭空多出一条空 INSERT。
 */
export function committedNewRowKeys(dirty: PendingRows): string[] {
  return [...dirty.keys()]
    .filter(key => isCommittedNewRow(key, dirty.get(key)))
    .sort((a, b) => newRowSeqOf(a) - newRowSeqOf(b))
}

/** 待写入的值算不算「有内容」：NULL 标记与空串都不算（它们等于没填） */
export function hasMeaningfulValue(v: CellInput | undefined): boolean {
  if (v === undefined || v === null) return false
  if (isSqlNull(v)) return false
  return String(v) !== ''
}

/**
 * 该列是不是「没有兜底的必填列」：NOT NULL、且没有默认值、也不是自增 / 生成列。
 *
 * 这类列 INSERT 时如果省略，数据库只会回一句 `Column 'x' cannot be null`，
 * 而且是在点过保存、事务已经开始之后 —— 不如在填写阶段就说清楚。
 */
export function isRequiredWithoutDefault(col: ColumnInfo | undefined): boolean {
  if (!col) return false
  if (col.is_nullable) return false
  if (col.is_auto_increment || col.is_generated) return false
  const d = col.default_value
  return d === null || d === undefined || String(d) === ''
}

/** 待插入行缺失的必填列名（顺序按结果列顺序，方便照着补齐） */
export function missingRequiredColumns(
  columns: string[],
  pending: Map<string, CellInput> | undefined,
  metaOf: (col: string) => ColumnInfo | undefined
): string[] {
  return columns.filter(c => {
    if (!isRequiredWithoutDefault(metaOf(c))) return false
    return !hasMeaningfulValue(pending?.get(c))
  })
}

/** INSERT 时要写入的列：只挑「填了东西」的，其余让它落回默认值 / NULL */
export function filledColumns(pending: Map<string, CellInput>, columns: string[]): string[] {
  return columns.filter(c => hasMeaningfulValue(pending.get(c)))
}
