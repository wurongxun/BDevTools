/**
 * MAT 结果导出工具（纯前端 Blob 下载，不经过后端 fs 权限）。
 *
 * 大表导出只导出**当前已取到的数据**（服务端分页时是当前页），
 * 与界面列名、顺序保持一致；不做「后端全量导出」，避免把 GB 级结果一次性塞进内存。
 */

export interface ExportColumn {
  key: string
  label: string
}

/** 任意值 → CSV 单元格文本（对象序列化为 JSON，避免 [object Object]） */
function cellText(v: unknown): string {
  if (v == null) return ''
  if (typeof v === 'object') return JSON.stringify(v)
  return String(v)
}

function csvCell(v: unknown): string {
  const s = cellText(v)
  return /[",\r\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s
}

/** 生成带 UTF-8 BOM 的 CSV 文本（Excel 直接打开不乱码） */
export function toCsv(columns: ExportColumn[], rows: readonly unknown[]): string {
  const head = columns.map(c => csvCell(c.label)).join(',')
  const body = rows.map(r => columns.map(c => csvCell((r as Record<string, unknown>)?.[c.key])).join(','))
  return '\ufeff' + [head, ...body].join('\r\n')
}

export function downloadText(filename: string, content: string, mime: string): void {
  const blob = new Blob([content], { type: mime })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.style.display = 'none'
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  // 立即 revoke 在部分 WebView 会打断下载，延后一拍释放
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

export function exportCsv(
  filename: string,
  columns: ExportColumn[],
  rows: readonly unknown[],
): void {
  downloadText(filename, toCsv(columns, rows), 'text/csv;charset=utf-8')
}

export function escapeHtml(s: unknown): string {
  return String(s ?? '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

/** 生成可独立打开的 HTML 报告（内联样式，不依赖 CDN / 外部资源） */
export function reportHtml(title: string, meta: string[], bodyHtml: string): string {
  const items = meta.map(m => `<li>${escapeHtml(m)}</li>`).join('')
  return `<!doctype html>
<html lang="zh-CN"><head><meta charset="utf-8"><title>${escapeHtml(title)}</title>
<style>
body{font:14px/1.6 -apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;color:#1f2328;background:#fff;margin:32px auto;max-width:960px;padding:0 20px}
h1{font-size:20px;font-weight:500;border-bottom:1px solid #d0d7de;padding-bottom:8px}
h2{font-size:16px;font-weight:500;margin-top:28px}
ul.meta{color:#57606a;font-size:12px;padding-left:18px}
table{border-collapse:collapse;width:100%;font-size:13px;margin:12px 0}
th,td{border:1px solid #d0d7de;padding:6px 10px;text-align:left}
th{background:#f6f8fa;font-weight:500}
td.num{text-align:right;font-variant-numeric:tabular-nums}
.card{border:1px solid #d0d7de;border-radius:8px;padding:12px 16px;margin:12px 0}
.bar{height:8px;background:#dbeafe;border-radius:4px;overflow:hidden}
.bar > i{display:block;height:100%;background:#3b82f6}
.muted{color:#57606a;font-size:12px}
</style></head><body>
<h1>${escapeHtml(title)}</h1><ul class="meta">${items}</ul>
${bodyHtml}
<p class="muted">由 BDevTools · Java 堆转储分析生成 · ${escapeHtml(new Date().toLocaleString())}</p>
</body></html>`
}

export function exportHtml(filename: string, html: string): void {
  downloadText(filename, html, 'text/html;charset=utf-8')
}
