/**
 * 离线正则工坊 + 批量替换影响面（M4 差异化）。
 *
 * **口径说明**：当前宿主没有可复用的 LLM 通道（不能为了一个输入框引入云服务/密钥），
 * 因此这里用**本地规则模板**做等价能力：自然语言关键词 → 常用正则候选 + 解释 + 示例，
 * 以及把预览结果翻译成人能读懂的风险清单。纯函数、无副作用、可单测；
 * 将来接了模型只需替换 `suggestRegex` 的实现，UI 契约不变。
 */
import type { ReplaceOutcome } from './fileSearchApi'

export interface RegexSuggestion {
  id: string
  label: string
  pattern: string
  explanation: string
  example: string
}

export interface ImpactLine {
  level: 'info' | 'warn' | 'danger'
  text: string
}

interface Rule extends RegexSuggestion {
  /** 命中这些关键词就推荐该模板（大小写不敏感） */
  keywords: string[]
}

const RULES: Rule[] = [
  {
    id: 'email', label: '邮箱地址', pattern: '[\\w.+-]+@[\\w-]+\\.[\\w.-]+',
    explanation: '匹配常见邮箱：用户名允许点、加号、连字符，域名至少一段点分后缀。',
    example: 'dev+test@example.com', keywords: ['邮箱', '邮件', 'email', 'mail', '@']
  },
  {
    id: 'ipv4', label: 'IPv4 地址', pattern: '\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b',
    explanation: '四段 1–3 位数字；不校验每段 ≤255，需要精确校验请在替换前人工确认。',
    example: '192.168.1.10', keywords: ['ip', '地址', 'ipv4', '服务器']
  },
  {
    id: 'url', label: 'HTTP(S) 网址', pattern: 'https?://[^\\s"\'<>]+',
    explanation: '匹配到第一个空白或引号/尖括号为止，适合日志与配置里的链接。',
    example: 'https://old.example.com/api', keywords: ['网址', '链接', 'url', 'http', '域名', 'domain']
  },
  {
    id: 'date', label: '日期（年-月-日）', pattern: '\\b\\d{4}[-/]\\d{1,2}[-/]\\d{1,2}\\b',
    explanation: '支持 / 或 - 分隔，月日允许一位或两位。',
    example: '2026-11-10', keywords: ['日期', '时间', 'date', '年月日']
  },
  {
    id: 'phone', label: '中国大陆手机号', pattern: '\\b1[3-9]\\d{9}\\b',
    explanation: '1 开头、第二位 3–9、共 11 位，前后需为单词边界。',
    example: '13800138000', keywords: ['手机', '电话', 'phone', '手机号']
  },
  {
    id: 'loglevel', label: '日志级别', pattern: '\\b(?:ERROR|WARN(?:ING)?|INFO|DEBUG|TRACE|FATAL)\\b',
    explanation: '匹配常见日志级别；替换大小写敏感，需要忽略大小写请勾选面板上的选项。',
    example: 'ERROR', keywords: ['日志', '级别', 'log', 'error', 'warn', 'debug']
  },
  {
    id: 'quoted', label: '引号内内容（捕获组 1）', pattern: '["\']([^"\']+)["\']',
    explanation: '第 1 捕获组可写进替换串的 $1，适合批量改引号里的值。',
    example: '"old-value"', keywords: ['引号', '字符串', 'quote', 'string', '捕获']
  },
  {
    id: 'blank', label: '空行', pattern: '^\\s*$',
    explanation: '整行只有空白字符；替换成空串等于删除所有空行。',
    example: '（空白行）', keywords: ['空行', '空白', 'blank', 'empty']
  },
  {
    id: 'todo', label: 'TODO / FIXME 标记', pattern: '\\b(?:TODO|FIXME|XXX|HACK)\\b',
    explanation: '匹配代码注释里的常见待办标记。',
    example: '// TODO: refactor', keywords: ['todo', 'fixme', '待办', '标记']
  },
  {
    id: 'hexcolor', label: '十六进制颜色', pattern: '#[0-9a-fA-F]{6}\\b',
    explanation: '匹配 CSS/配置中的 6 位颜色值，不含 3 位简写。',
    example: '#ff8800', keywords: ['颜色', 'color', 'css', 'hex']
  },
  {
    id: 'number', label: '整数或小数', pattern: '\\b\\d+(?:\\.\\d+)?\\b',
    explanation: '匹配独立数字（不含千分位逗号与科学计数法）。',
    example: '1234.56', keywords: ['数字', 'number', '数值', '整数', '小数']
  },
  {
    id: 'dupword', label: '连续重复单词', pattern: '\\b(\\w+)\\s+\\1\\b',
    explanation: '反向引用 \\1 找出 "the the" 这类重复，替换成 $1 即可去重。',
    example: 'the the', keywords: ['重复', 'duplicate', '叠词']
  }
]

function escapeLiteral(v: string): string {
  return v.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

/** 自然语言 → 正则候选。空输入给常用模板；无命中时给「按字面量匹配」兜底。 */
export function suggestRegex(input: string): RegexSuggestion[] {
  const q = input.trim().toLowerCase()
  const strip = (r: Rule): RegexSuggestion => ({
    id: r.id, label: r.label, pattern: r.pattern,
    explanation: r.explanation, example: r.example
  })
  if (!q) return RULES.slice(0, 6).map(strip)

  const scored = RULES
    .map((r) => ({
      r,
      score: r.keywords.reduce(
        (n, k) => n + (q.includes(k.toLowerCase()) || k.toLowerCase().includes(q) ? 1 : 0),
        0
      )
    }))
    .filter((x) => x.score > 0)
    .sort((a, b) => b.score - a.score)
    .map((x) => strip(x.r))

  if (scored.length) return scored.slice(0, 8)

  // 没匹配到模板时也不返回空：给一个字面量转义版，避免用户输入被当正则元字符炸掉
  const literal = input.trim()
  return [{
    id: 'literal',
    label: '按字面量匹配',
    pattern: escapeLiteral(literal),
    explanation: '没有匹配的模板时，把输入当普通文本查找（已转义所有正则元字符）。',
    example: literal
  }, ...RULES.slice(0, 3).map(strip)]
}

export interface ImpactOptions {
  /** 已打开且未保存的文档路径（外部注入，保持纯函数） */
  dirtyOpenPaths?: string[]
  /** 本次预览是否只勾选了部分文件 */
  selectedOnly?: boolean
}

/** 把一次替换预览翻译成可读的风险清单。 */
export function describeReplaceImpact(
  outcome: ReplaceOutcome | null,
  opts: ImpactOptions = {}
): ImpactLine[] {
  if (!outcome) return []
  const items = outcome.items || []
  const affected = items.filter((i) => i.replacements > 0)
  const failed = items.filter((i) => i.code !== 200 && i.msg !== '无匹配')
  const out: ImpactLine[] = []

  out.push({
    level: 'info',
    text: `将替换 ${outcome.total_replacements} 处，影响 ${affected.length} 个文件${opts.selectedOnly ? '（仅勾选文件）' : '（全部命中文件）'}。`
  })

  const nonUtf8 = affected.filter(
    (i) => i.encoding && !/^utf-?8$/i.test(i.encoding)
  )
  if (nonUtf8.length) {
    out.push({
      level: 'warn',
      text: `${nonUtf8.length} 个文件不是 UTF-8（${[...new Set(nonUtf8.map((i) => i.encoding))].slice(0, 3).join('、')}）：会按原编码回写，新内容若含该编码无法表示的字符将被 415 拒绝，不会写半截文件。`
    })
  }
  const crlf = affected.filter((i) => i.eol === 'CRLF').length
  if (crlf) {
    out.push({ level: 'info', text: `${crlf} 个 CRLF 文件：换行符按原值保留，不会被改成 LF。` })
  }
  if (failed.length) {
    out.push({
      level: 'danger',
      text: `${failed.length} 个文件在预览阶段报错（${failed.slice(0, 2).map((i) => i.msg).join('；')}），应用时会跳过它们。`
    })
  }
  const dirty = opts.dirtyOpenPaths?.length || 0
  if (dirty) {
    out.push({
      level: 'danger',
      text: `${dirty} 个命中文件在编辑器里有未保存改动：替换写盘后不会自动刷新这些标签，保存时会弹出外部修改冲突处理。`
    })
  }
  const heavy = affected.filter((i) => i.replacements > 1000)
  if (heavy.length) {
    out.push({
      level: 'warn',
      text: `${heavy.length} 个文件单文件替换超过 1000 处，建议先确认关键词范围；应用前会为每个文件生成 .bak 备份。`
    })
  }
  out.push({
    level: 'info',
    text: '应用时逐文件「备份 → 原子写 → 记录撤销条目」；409 外部修改冲突会跳过该文件。撤销清单会落盘，重启后仍可在「撤销历史」中恢复。'
  })
  return out
}
