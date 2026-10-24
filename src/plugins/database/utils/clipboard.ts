/**
 * 剪贴板写入（带结果提示）。
 *
 * 抽出来的原因：复制按钮会出现在各处（DDL、对象源码、字段名…），每处自己写一遍
 * 就会各自漏一种失败情况。这里把三种结局统一掉：
 * - 成功 → success 提示
 * - 环境不支持（非安全上下文 / 老内核）→ warning，并告诉用户可以手动选中
 * - 被浏览器拒绝（权限策略）→ error
 *
 * **绝不静默失败**：用户点了复制却什么也没发生，比报错更让人困惑。
 */
export async function copyText(text: string, okMsg = '已复制'): Promise<void> {
  const { ElMessage } = await import('element-plus')
  if (!text) return
  if (!navigator.clipboard?.writeText) {
    ElMessage.warning('当前环境不支持剪贴板，请手动选中复制')
    return
  }
  try {
    await navigator.clipboard.writeText(text)
    ElMessage.success(okMsg)
  } catch {
    ElMessage.error('复制失败：剪贴板不可用或被浏览器拒绝')
  }
}
