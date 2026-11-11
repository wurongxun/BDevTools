/**
 * 轻量文档大纲（B5）。
 *
 * 不做语义解析/符号索引（路线图第 9 节明确「不把 files 做成 IDE」），只按行做
 * 启发式扫描：Markdown 标题、常见语言的函数/类/接口/枚举声明、JSON 顶层键。
 * 大文件（>1MB）不在内存里，调用方传空内容时返回空数组即可。
 *
 * 纯函数，无 Vue / Tauri 依赖，便于单测。
 */
export interface OutlineItem {
  line: number
  label: string
  level: number
}

const MAX_ITEMS = 500
const MAX_LABEL = 120

const DECL_RE =
  /^\s{0,24}(?:(?:export|default|pub|public|private|static|async|final|abstract)\s+){0,3}(?:function|class|interface|enum|struct|trait|impl|def|fn|func|sub|type|namespace|module|package|record)\s+([A-Za-z_$][\w$<>.]*)/

const METHOD_RE = /^\s{0,24}(?:def|fn|func)\s+([A-Za-z_$][\w$]*)\s*\(/

export function buildOutline(content: string, language = 'plaintext'): OutlineItem[] {
  if (!content || typeof content !== 'string') return []
  const out: OutlineItem[] = []
  const lines = content.split('\n')
  const isMarkdown = language === 'markdown' || language === 'md'
  const isJson = language === 'json'

  for (let i = 0; i < lines.length && out.length < MAX_ITEMS; i++) {
    const raw = lines[i].replace(/\r$/, '')
    if (raw.length > MAX_LABEL * 2) continue

    if (isMarkdown) {
      const m = /^\s{0,3}(#{1,6})\s+(.+?)\s*#*\s*$/.exec(raw)
      if (m) {
        out.push({ line: i + 1, label: m[2].slice(0, MAX_LABEL), level: m[1].length })
        continue
      }
      continue
    }

    if (isJson) {
      const m = /^\s{0,4}"([^"]{1,80})"\s*:/.exec(raw)
      if (m) out.push({ line: i + 1, label: m[1], level: 1 })
      continue
    }

    const decl = DECL_RE.exec(raw) || METHOD_RE.exec(raw)
    if (decl) {
      const indent = /^\s*/.exec(raw)?.[0].length ?? 0
      out.push({
        line: i + 1,
        label: raw.trim().slice(0, MAX_LABEL),
        level: Math.min(6, Math.floor(indent / 2) + 1)
      })
    }
  }
  return out
}
