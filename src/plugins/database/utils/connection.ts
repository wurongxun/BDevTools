/**
 * 确保连接可用。
 *
 * 抽出来的原因：多个查看器（表 / 视图 / 例程）在加载数据前都需要这一步，
 * 否则**关闭标签页触发断连、再打开旧标签页**时会全部 fail — 连接状态在我们的模型里
 * 是与标签页生命周期绑定的。
 */
import { ElMessage } from 'element-plus'
import { useConnectionStore } from '../store'

/**
 * 已连接则直接返回 true；未连接尝试重连，失败时**给出提示**（不静默返回 false，
 * 否则调用方的表现是「什么都没加载出来」，用户不知道该重连还是重试）。
 */
export async function ensureConnection(configId: string): Promise<boolean> {
  if (!configId) return false
  const store = useConnectionStore()
  if (store.isConnected(configId)) return true
  const res = await store.connect(configId)
  if (res.code !== 200) {
    ElMessage.error('连接失败：' + (res.msg || '未知错误'))
    return false
  }
  return true
}
