/**
 * MAT（Memory Analyzer Tool）插件类型定义
 */

/** 一个已打开的堆转储文件 */
export interface MatDump {
  /** 会话内唯一 id（path 派生，重复打开去重） */
  id: string
  /** 绝对路径 */
  path: string
  /** 展示名（文件名） */
  name: string
  /** 文件大小（字节） */
  size: number
  /** 格式名：JAVA PROFILE 1.0.2 / PHD / UNKNOWN */
  format: string
  /** id 宽度（4/8），HPROF 才有 */
  idSize: number
  /** dump 时间文本（本地时区） */
  timestampText: string
  /** 该格式是否可解析（当前仅 JAVA PROFILE 家族） */
  supported: boolean
  /** 不支持时的说明 / 识别附加信息 */
  detail: string
  /** 打开过程中的错误信息（header 识别失败等） */
  error?: string
  /** 是否正在加载元信息 */
  loading: boolean
}

/** 概览 → 直方图等分析视图的 key（与 DumpViewer 子 tab 对应） */
export type MatViewKey = 'overview' | 'histogram' | 'leak' | 'dominator' | 'refs' | 'oql' | 'reserve'
