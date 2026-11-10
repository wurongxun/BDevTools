/**
 * 「转换文件编码 / 换行符」对话框的模块级单例状态（M4 / E4+E5 遗留）。
 *
 * 与 `TextView` 的编码下拉严格区分：
 * - TextView 的 `switchEncoding` 只换**读取解码方式**，不动磁盘一个字节；
 * - 本对话框是**把磁盘字节真正重写**成目标编码/EOL，属于不可逆操作，
 *   必须先确认、必须备份 .bak、必须拒绝脏文档（否则内存里的编辑会被覆盖）。
 *
 * 状态放在模块级是为了让状态栏、命令面板、TextView 菜单共用一个入口，
 * 与 `editorPreferencesVisible` / `macroDialogVisible` 同范式。
 */
import { ref } from 'vue'

export const convertDialogVisible = ref(false)
/** 目标文件路径；空串表示「跟随当前活动文档」 */
export const convertTargetPath = ref('')

export function openConvertDialog(path = ''): void {
  convertTargetPath.value = path
  convertDialogVisible.value = true
}

export function closeConvertDialog(): void {
  convertDialogVisible.value = false
}

/** 供 UI / 逻辑回归复用的编码与换行选项（与 TextView 的读取编码列表保持同源口径）。 */
/**
 * 标签用 WHATWG label（`Encoding::for_label` 能解析）；`encoding_rs::name()` 返回的大小写
 * 不一定一致（GB18030 的 name 是小写 gb18030、Big5 是 Big5），比较一律忽略大小写。
 */
export const CONVERT_ENCODINGS = [
  'UTF-8',
  'GBK',
  'GB18030',
  'BIG5',
  'UTF-16LE',
  'UTF-16BE',
  'Shift_JIS',
  'EUC-JP',
  'EUC-KR',
  'windows-1252'
] as const

export const CONVERT_EOLS = ['LF', 'CRLF', 'CR'] as const

export interface ConvertForm {
  encoding: string
  eol: string
  bom: string
}

/** 纯函数：目标三元组是否相对现状有实际变化（UI 用它禁用「无变化」的转换按钮）。 */
export function hasConvertChange(
  form: ConvertForm,
  current: { encoding?: string; eol?: string; bom?: string }
): boolean {
  const enc = (form.encoding || '').toUpperCase()
  const curEnc = (current.encoding || 'UTF-8').toUpperCase()
  const eol = (form.eol || 'LF').toUpperCase()
  const curEol = (current.eol || 'LF').toUpperCase()
  const bom = (form.bom || 'none').toLowerCase()
  const curBom = (current.bom || 'none').toLowerCase()
  // UTF-16 目标会被后端强制写成对应 BOM，这里按后端口径归一，避免按钮误亮
  const effectiveBom = enc === 'UTF-16LE' ? 'utf16le' : enc === 'UTF-16BE' ? 'utf16be' : bom
  return enc !== curEnc || eol !== curEol || effectiveBom !== curBom
}
