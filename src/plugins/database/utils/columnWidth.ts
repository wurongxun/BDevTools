/**
 * 结果网格：列语义归类 + 列宽估算 + 表头类型文案。
 *
 * 为什么要有这个模块：
 *
 * 1. SQL 查询分支拿不到类型。operations.rs 的 SqlResult.columns 只有 Vec<String>（列名），
 *    所以以前每列只能退回统一的 min-width —— 一列 100px，布尔列、自增 ID 列照样占满，
 *    横向一半以上的像素是空的。这里按首屏采样值 + 列名估算贴合内容的宽度，
 *    让短值列真的变短，省下来的宽度直接变成「多看几列」。
 *
 * 2. 表浏览分支有完整类型却没显示。TableDataResult.columns 是 Vec<ColumnInfo>
 *    （类型 / 可空 / 主键齐全），但那些信息只能靠点开 Metadata 面板才看得到，
 *    盯着数据的时候屏幕上一条都没有。
 *
 * 于是两条路一起补：查得到真类型就用真类型，查不到就采样推断。
 * 但推断出来的必须能在视觉上区分（给调用方 inferred 标记），不能当成数据库给的类型。
 */

import type { ColumnInfo } from '../types'

/** 列的数据语义归类。这是给 UI 用的「够用分类」，不是 SQL 类型系统。 */
export type ColumnKind =
  | 'int'
  | 'num'
  | 'bool'
  | 'date'
  | 'time'
  | 'datetime'
  | 'uuid'
  | 'json'
  | 'text'

// ============================== 值识别 ==============================

const UUID_RE =
  /^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$/
const DATE_RE = /^\d{4}-\d{2}-\d{2}$/
const TIME_RE = /^\d{1,2}:\d{2}(:\d{2})?(\.\d+)?$/
/** MySQL TIMESTAMP 不带时区后缀、PG timestamptz 必须带：两侧都要认 */
const DATETIME_RE = /^\d{4}-\d{2}-\d{2}[T ]\d{1,2}:\d{2}(:\d{2})?(\.\d+)?(Z|[+-]\d{2}:?\d{2})?$/
/** 刻意排除前导零：007 / 0123 这类是编号，被认成数字会让表头说谎 */
const INT_RE = /^-?(0|[1-9]\d*)$/
const NUM_RE = /^-?\d+\.\d+$/

/** 非 Latin-1 一律按全角估宽：CJK 与全角符号的实际宽度都接近 1em */
const WIDE_CHAR_RE = /[^\u0000-\u00FF]/g

/** 绝大多数非空值落在同一类才认 —— 宁可不标，也不标错 */
const KIND_RATIO = 0.8

function classifyValue(v: unknown): ColumnKind | null {
  if (v === null || v === undefined) return null
  // 每处都直接判 `typeof v`（不要存成中间变量再判）—— 只有这种写法 TS 才窄化出具体类型
  if (typeof v === 'boolean') return 'bool'
  if (typeof v === 'number')
    return Number.isFinite(v) ? (Number.isInteger(v) ? 'int' : 'num') : 'text'
  if (typeof v === 'object') return 'json'
  if (typeof v !== 'string') return null
  const s = v.trim()
  if (s === '') return null
  if (UUID_RE.test(s)) return 'uuid'
  if (DATETIME_RE.test(s)) return 'datetime'
  if (DATE_RE.test(s)) return 'date'
  if (TIME_RE.test(s)) return 'time'
  if (INT_RE.test(s)) return 'int'
  if (NUM_RE.test(s)) return 'num'
  if (s === 'true' || s === 'false') return 'bool'
  if (s.charAt(0) === '{' || s.charAt(0) === '[') return 'json'
  return 'text'
}

/** 把采样值转成参与宽度估算的文本（对象走 JSON，与 ResultTable 的 formatCell 同口径） */
function sampleText(v: unknown): string {
  if (typeof v === 'string') return v
  if (typeof v === 'object') {
    try {
      return JSON.stringify(v) ?? String(v)
    } catch {
      return String(v)
    }
  }
  return String(v)
}

// ============================== 列宽估算 ==============================

/**
 * 结果网格正文 12px 下的字符宽度近似值。
 *
 * 估算只用于「给列排宽度」，不需要像素级精确 —— vxe 自己会对溢出内容做 ellipsis，
 * 差几像素不影响可用性，误差只是体现在最终要不要省略号上。
 */
const CHAR_W_NARROW = 6.4
const CHAR_W_WIDE = 12
/** 表头字号比正文略大，估宽跟着放大 */
const HEADER_W_SCALE = 1.12

/** 单元格左右内边距 + 一点余量，不然最后一个字符会贴着边框 */
const CELL_PADDING = 18
/** 表头还要给排序箭头和列宽手柄留位置，否则会被挤到下一行 */
const HEADER_PADDING = 26

/**
 * 表头第二行参与宽度竞争时的上限。
 *
 * 类型标注是「顺手塞进来的信息」，绝不能反过来把列撑宽 —— 没有这个上限，
 * 一个 `time without time zone` 就能把列顶到 160px，比不显示还糟。
 * 超出部分交给 ellipsis（完整值仍在 title 里）。
 */
const HEADER_SUB_MAX_WIDTH = 84

/** 采样行数。够代表一列的实际宽度，又不至于逐格扫描（大数据集开销敏感） */
const DEFAULT_SAMPLE_ROWS = 50
/** 单个值参与估算时最多算多少字符：一个超长 JSON 不该把整列撑到 800px */
const VALUE_CHAR_CAP = 48

/** 各语义的宽度上限。作用是兜住异常值，不是给常态宽度 */
const KIND_MAX_WIDTH: Record<ColumnKind, number> = {
  bool: 72,
  int: 96,
  num: 112,
  date: 104,
  time: 96,
  datetime: 184,
  uuid: 272,
  json: 300,
  text: 300,
}

const clamp = (v: number, min: number, max: number) => Math.min(Math.max(v, min), max)

function estimateWidth(text: string, narrowW: number, wideW: number): number {
  if (!text) return 0
  const wideCount = text.match(WIDE_CHAR_RE)?.length ?? 0
  return wideCount * wideW + (text.length - wideCount) * narrowW
}

// ============================== 对外 API ==============================

/**
 * 按首屏采样值推断每列的语义归类。查不到（数据为空、混了多种类型）时返回 null。
 *
 * 返回值要么是某一类，要么是 null —— 中间加不了「大概」这一档，因为 UI 上
 * 「不显示」永远好过「显示错的类型」。
 */
export function inferColumnKinds(
  columns: string[],
  rows: Record<string, any>[],
  sampleRows = DEFAULT_SAMPLE_ROWS
): (ColumnKind | null)[] {
  const sample = rows.slice(0, sampleRows)
  const out: (ColumnKind | null)[] = []
  for (let c = 0; c < columns.length; c++) {
    const key = String(c)
    const counts = new Map<ColumnKind, number>()
    let seen = 0
    let bestCount = 0
    let best: ColumnKind | null = null
    for (const row of sample) {
      const kind = classifyValue(row?.[key])
      if (!kind) continue
      seen++
      const n = (counts.get(kind) ?? 0) + 1
      counts.set(kind, n)
      if (n > bestCount) {
        bestCount = n
        best = kind
      }
    }
    out.push(seen > 0 && best !== null && bestCount / seen >= KIND_RATIO ? best : null)
  }
  return out
}

export interface ColumnWidthOptions {
  /** 列最小宽度（同时也是兜底：整列无数据时表头至少要有这么宽） */
  minWidth?: number
  /** 采样行数 */
  sampleRows?: number
  /** 表头第二行文本（类型标注），参与宽度竞争 */
  headerSubLabels?: (string | null)[]
  /**
   * 已算好的语义归类（由 inferColumnKinds 产出）。
   * 传进来可以避免为取宽度上限再遍历一遍采样值。
   */
  kinds?: (ColumnKind | null)[]
}

/**
 * 估算每列的贴合宽度：内容是主力，表头是下限，语义是上限。
 *
 * 三者之间的优先级没有含糊空间：
 * - 全部为 NULL 的列退化成「表头宽度」，不会缩成一条缝；
 * - 含有超长文本的列被语义上限兜住，不会一根独柱吃掉整屏。
 */
export function computeColumnWidths(
  columns: string[],
  rows: Record<string, any>[],
  opts: ColumnWidthOptions = {}
): number[] {
  const minWidth = opts.minWidth ?? 56
  const sampleRows = opts.sampleRows ?? DEFAULT_SAMPLE_ROWS
  const subLabels = opts.headerSubLabels
  const kinds = opts.kinds
  const sample = rows.slice(0, sampleRows)
  const headerNarrow = CHAR_W_NARROW * HEADER_W_SCALE
  const headerWide = CHAR_W_WIDE * HEADER_W_SCALE
  const out: number[] = []

  for (let c = 0; c < columns.length; c++) {
    const key = String(c)
    let contentW = 0
    for (const row of sample) {
      const v = row?.[key]
      if (v === null || v === undefined || v === '') continue
      const s = sampleText(v)
      const w = estimateWidth(
        s.length > VALUE_CHAR_CAP ? s.slice(0, VALUE_CHAR_CAP) : s,
        CHAR_W_NARROW,
        CHAR_W_WIDE
      )
      if (w > contentW) contentW = w
    }

    const nameW = estimateWidth(String(columns[c] ?? ''), headerNarrow, headerWide)
    const sub = subLabels?.[c]
    const subW = sub
      ? Math.min(estimateWidth(sub, headerNarrow, headerWide), HEADER_SUB_MAX_WIDTH)
      : 0
    const headerW = Math.max(nameW, subW) + HEADER_PADDING

    const kind = kinds?.[c] ?? null
    const maxW = Math.max(minWidth, KIND_MAX_WIDTH[kind ?? 'text'])
    out.push(Math.round(clamp(Math.max(contentW + CELL_PADDING, headerW), minWidth, maxW)))
  }
  return out
}

// ============================== 表头文案 ==============================

/** 推断类型的显示标签。text 刻意返回 null —— 「这列是文本」没有信息量，别占表头 */
const INFERRED_LABEL: Record<ColumnKind, string | null> = {
  int: 'int',
  num: 'num',
  bool: 'bool',
  date: 'date',
  time: 'time',
  datetime: 'datetime',
  uuid: 'uuid',
  json: 'json',
  text: null,
}

/**
 * 各库原始类型名 → 统一短名。
 *
 * 直接显示原类型名的话，同一列在 MySQL 是 tinyint(1)、SQL Server 是 bit、
 * PG 是 boolean —— 视觉上三份不同的东西，白白占用注意力。
 */
const TYPE_ALIAS: Record<string, string> = {
  int: 'int',
  integer: 'int',
  smallint: 'int',
  tinyint: 'int',
  mediumint: 'int',
  serial: 'int',
  bigint: 'bigint',
  bigserial: 'bigint',
  decimal: 'decimal',
  numeric: 'decimal',
  money: 'decimal',
  number: 'num',
  float: 'float',
  real: 'float',
  double: 'float',
  'double precision': 'float',
  bool: 'bool',
  boolean: 'bool',
  bit: 'bool',
  date: 'date',
  time: 'time',
  timetz: 'time',
  'time without time zone': 'time',
  datetime: 'datetime',
  timestamp: 'datetime',
  'timestamp without time zone': 'datetime',
  'timestamp with time zone': 'datetime',
  timestamptz: 'datetime',
  datetime2: 'datetime',
  smalldatetime: 'datetime',
  datetimeoffset: 'datetime',
  tinytext: 'text',
  text: 'text',
  mediumtext: 'text',
  longtext: 'text',
  clob: 'text',
  char: 'varchar',
  nchar: 'varchar',
  varchar: 'varchar',
  nvarchar: 'varchar',
  varchar2: 'varchar',
  nvarchar2: 'varchar',
  'character varying': 'varchar',
  string: 'varchar',
  json: 'json',
  jsonb: 'json',
  uuid: 'uuid',
  uniqueidentifier: 'uuid',
  blob: 'binary',
  bytea: 'binary',
  binary: 'binary',
  varbinary: 'binary',
  longblob: 'binary',
  mediumblob: 'binary',
  raw: 'binary',
}

function normalizeTypeName(raw: string): string {
  // MySQL 的 `bigint unsigned` / `int zerofill` —— 修饰符不带 (±取值区间) 之外的有用信息
  const base = String(raw ?? '')
    .trim()
    .toLowerCase()
    .replace(/\(.*\)\s*$/, '')
    .replace(/\s+(unsigned|signed|zerofill)\s*$/, '')
    .replace(/\s+/g, ' ')
  return TYPE_ALIAS[base] ?? base
}

/** 长度 / 精度只有在「短到能一眼读完」时才拼上去，否则整个标注就比列名还长了 */
function decorate(base: string, info: ColumnInfo): string {
  const len = info.character_max_length
  if ((base === 'varchar' || base === 'text' || base === 'binary') && len && len > 0 && len <= 999) {
    return `${base}(${len})`
  }
  const p = info.numeric_precision
  if (base === 'decimal' && p && p > 0) return `${base}(${p},${info.numeric_scale ?? 0})`
  return base
}

/** 表头要显示的真实类型文案（来自 ColumnInfo） */
export function typeLabelFromInfo(info: ColumnInfo): string {
  return decorate(normalizeTypeName(info.data_type), info)
}

/** 表头要显示的推断类型文案；无可显示内容（text / 未识别）时为 null */
export function inferredKindLabel(kind: ColumnKind | null): string | null {
  return kind ? INFERRED_LABEL[kind] : null
}
