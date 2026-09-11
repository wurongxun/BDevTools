/**
 * 日志行解析与搜索所用的正则常量
 *
 * 集中放置便于统一调优：解析是"尽力而为"的启发式，不要求覆盖所有格式，
 * 后续接入后端索引后可切换为服务端解析。
 */

/** 时间戳：ISO8601 / yyyy-MM-dd HH:mm:ss[.SSS] / yyyy/M/d HH:mm:ss / MM-dd HH:mm:ss（无年份） */
export const TS_RE = new RegExp(
  '^\\s*(\\[?)(' +
    [
      '\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(?:[.,]\\d{1,9})?(?:Z|[+-]\\d{2}:?\\d{2})?',
      '\\d{4}/\\d{1,2}/\\d{1,2}[T ]\\d{2}:\\d{2}:\\d{2}(?:[.,]\\d{1,3})?',
      '\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(?:[.,]\\d{1,3})?',
    ].join('|') +
    ')(\\]?)',
)

/** 级别词（大小写不敏感，顺序保证 WARNING 优先于 WARN 匹配） */
export const LEVEL_RE =
  /\b(FATAL|CRITICAL|CRIT|SEVERE|PANIC|ERROR|ERR|EXCEPTION|WARNING|WARN|INFORMATION|INFO|NOTICE|DEBUG|TRACE|VERBOSE|FINEST|FINER|FINE)\b/i

/** 线程名：[main] / [http-nio-8080-exec-1] */
export const THREAD_RE = /\[([^\]\s][^\]]{0,63})\]/

/** logger / 类名：a.b.c.ClassName，后接分隔符 - : | 或空格 */
export const LOGGER_RE = /((?:[a-zA-Z_$][\w$]*\.)+[A-Za-z_$][\w$]*)\s*(?:-|–|:|\||│)\s/

// ==================== 异常堆栈 ====================

/** Java 堆栈帧：at com.foo.Bar.baz(Bar.java:123) */
export const JAVA_FRAME_RE = /^\s*at\s+((?:[\w$]+\.)+)([\w$<>]+)\(([^)]*)\)\s*$/

/** Java 异常头：com.foo.MyException: 消息 */
export const JAVA_THROWN_RE = /^\s*((?:[\w$]+\.)*[A-Z][\w$]*(?:Exception|Error|Throwable))(?::\s?(.*))?$/

/** Java 根因：Caused by: java.lang.NullPointerException: 消息 */
export const JAVA_CAUSE_RE = /^\s*Caused by:\s*((?:[\w$]+\.)+[\w$]+)(?::\s?(.*))?$/

/** Python 堆栈帧：File "/app/main.py", line 12, in main */
export const PYTHON_FRAME_RE = /^\s*File\s+"([^"]+)",\s*line\s+(\d+)(?:,\s*in\s+(.+))?\s*$/

/** 省略提示：... 25 more / ... 12 common frames omitted */
export const STACK_MORE_RE = /^\s*\.\.\.\s+(\d+)\s+(?:more|common frames omitted)\s*$/

/** Python 回溯起始行：Traceback (most recent call last): */
export const PYTHON_TRACEBACK_RE = /^\s*Traceback \(most recent call last\):\s*$/

/** 堆栈帧参数中的 文件:行号 */
export const FILE_LINE_RE = /^([\w$.-]+\.[A-Za-z]+):(\d+)$/

// ==================== 搜索正则模板 ====================

export interface RegexTemplate {
  label: string
  pattern: string
}

/** 常用正则模板（一键套用，PCRE 风格，浏览器 RegExp 可直接执行） */
export const REGEX_TEMPLATES: RegexTemplate[] = [
  { label: 'IPv4 地址', pattern: '\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b' },
  { label: '时间戳', pattern: '\\d{4}-\\d{2}-\\d{2}[T ]\\d{2}:\\d{2}:\\d{2}(?:[.,]\\d{1,3})?' },
  { label: 'Java 异常类', pattern: '(?:[A-Za-z_$][\\w$]*\\.)+[A-Z][\\w$]*(?:Exception|Error|Throwable)' },
  { label: '异常堆栈行', pattern: '^\\s+(?:at\\s+|File ").+' },
  { label: 'UUID', pattern: '\\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\\b' },
  { label: 'URL', pattern: 'https?://[^\\s"\'）)]+' },
  { label: '耗时', pattern: '\\b\\d+(?:\\.\\d+)?\\s?(?:ms|s)\\b' },
  { label: '引号内容', pattern: '"[^"]*"' },
  { label: 'TraceId', pattern: '\\b(?:trace[-_]?id|tid)[=:\\s]+([A-Za-z0-9_-]{8,})\\b' },
]
