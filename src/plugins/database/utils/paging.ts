/**
 * 分页相关的纯逻辑。
 *
 * 放在这里而不是写在组件里，是为了能被 `.workbuddy/artifacts/` 下的仿真脚本直接 import
 * 源码来跑（Node 22 能直接执行 TS）——规则一旦漂移，仿真立刻变红。
 */

/**
 * 归一化「每页行数」输入。
 *
 * 设计取向：**不设上限**。能一次拉多少行由使用者自己判断（结果表已换成虚拟滚动，
 * 行数不再决定渲染开销），界面不该偷偷替他把数字改小 —— 那会造成「我填了 5000，
 * 回来的却只有 1000」的假象，比拒绝输入更难排查。
 *
 * 唯一的约束是「必须是安全范围内的正整数」：
 * - 空串 / 非数字 / 0 / 负数 ⇒ 保持原值不变（`changed: false`）
 * - 超出 `Number.MAX_SAFE_INTEGER` ⇒ 同上（JS 数字精度已不足以表达，交给 IP 层更是灾难）
 *
 * @param input 输入框里的原始文本（已由调用方过滤掉非数字字符）
 * @param current 当前生效的每页行数，非法输入回退到这里
 * @returns `{ size, changed }` —— `changed` 为 false 时调用方不必重新查询
 */
export function normalizePageSize(
  input: string | number | undefined | null,
  current: number
): { size: number; changed: boolean } {
  if (input === null || input === undefined) return { size: current, changed: false }
  const n = typeof input === 'number' ? Math.trunc(input) : parseInt(String(input), 10)
  if (!Number.isFinite(n) || !Number.isSafeInteger(n) || n <= 0) {
    return { size: current, changed: false }
  }
  return { size: n, changed: n !== current }
}

/**
 * 检测「请求了 N 行却只回来 M 行」的静默降级。
 *
 * 分页路径的契约是：只要 `has_more` 为真，返回行数就**必然等于**请求行数
 * （后端多取一行判断有没有下一页，再截断回请求行数）。
 * 因此 `返回行数 < 请求行数 && has_more` 只能说明中间有人（后端/驱动/连接层）
 * 把行数改小了 —— 这时候如果不吭声，用户只会看到「我填了 10000，界面毫无变化」。
 *
 * @param requested 本次请求的行数
 * @param returned 实际返回的行数
 * @param hasMore 后端是否声明还有下一页
 * @returns 提示文案；一切正常则返回 null
 */
export function describeRowLimitGap(
  requested: number,
  returned: number,
  hasMore: boolean
): string | null {
  if (!Number.isFinite(requested) || requested <= 0) return null
  if (!Number.isFinite(returned) || returned < 0) return null
  if (!hasMore) return null // 数据就这么多了，不算降级
  if (returned >= requested) return null // 正常分页：想要多少给多少
  return `实际只返回 ${returned.toLocaleString()} 行（请求 ${requested.toLocaleString()} 行）—— 请求行数被下层限制了，请检查是否生效到最新的后端版本`
}

/**
 * 总页数。**总行数未知时必须返回 null，而不是 0 / 1** ——
 *
 * 后端只在「纯 SELECT 的首页」才顺带统计总行数，翻页后 `total` 靠前端带过来，
 * 走表浏览 / 非查询语句时压根没有。把「不知道」显示成「共 1 页」是撒谎：
 * 用户看到「共 1 页」就不会点下一页，哪怕下面还有数据。
 *
 * @param total 总行数（null / undefined / 负数 = 未知）
 * @param pageSize 每页行数
 * @returns 总页数；总行数未知时返回 null
 */
export function computeTotalPages(
  total: number | null | undefined,
  pageSize: number
): number | null {
  if (typeof total !== 'number' || !Number.isFinite(total) || total < 0) return null
  if (!Number.isFinite(pageSize) || pageSize <= 0) return null
  return Math.max(1, Math.ceil(total / pageSize))
}

/**
 * 归一化「跳到第几页」的输入。
 *
 * 已知总页数时按 `[1, maxPage]` 收敛 —— 跳到没有数据的页码只会拿到一张空表，
 * 而页码显示却变成了那个越界的值，比不跳更让人困惑。
 * 未知总页数时只保证 ≥ 1：后端命中不到数据会返回空结果，用户自己看得见，
 * 也还能点「上一页」回来，不需要我们替他猜上限。
 */
export function normalizePageNumber(
  input: string | number | undefined | null,
  current: number,
  maxPage: number | null
): { page: number; changed: boolean } {
  if (input === null || input === undefined) return { page: current, changed: false }
  const n = typeof input === 'number' ? Math.trunc(input) : parseInt(String(input), 10)
  if (!Number.isFinite(n) || !Number.isSafeInteger(n) || n <= 0) {
    return { page: current, changed: false }
  }
  const page = maxPage == null ? n : Math.min(n, maxPage)
  return { page, changed: page !== current }
}

/**
 * 「读取所有行」时向后端要的行数。
 *
 * 已知总行数就按总数要（正好取完，`has_more` 必为 false）；
 * 未知时只能给一个兜底值 —— 这个值是**明知的上限**而不是撒谎：
 * 取回来若 `has_more` 仍为 true，说明数据比兜底值还多，
 * 调用方要跳过「行数降级检测」（那次报警说的是「下层偷偷钳制」，而这里是主动限流）。
 */
export const FETCH_ALL_FALLBACK_ROWS = 1_000_000

export function fetchAllSize(total: number | null | undefined): number {
  return typeof total === 'number' && Number.isFinite(total) && total > 0
    ? Math.trunc(total)
    : FETCH_ALL_FALLBACK_ROWS
}

/**
 * 定时刷新间隔（秒）的归一化：非法值一律按「关闭」处理。
 *
 * 与 pageSize 相反，这里不好回退到旧值——间隔是个一次性动作，
 * 填了 0 或负数就应当停止定时器，而不是继续按上次的间隔跑。
 */
export function normalizeIntervalSeconds(
  input: string | number | undefined | null
): number {
  if (input === null || input === undefined) return 0
  const n = typeof input === 'number' ? Math.trunc(input) : parseInt(String(input), 10)
  if (!Number.isFinite(n) || n <= 0) return 0
  return Math.min(n, 24 * 3600)
}
