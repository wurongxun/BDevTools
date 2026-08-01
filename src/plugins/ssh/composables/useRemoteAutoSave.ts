import { watch, onUnmounted } from 'vue'
import { useDocumentManagerStore } from '@/store/modules/documents'
import NotificationUtil from '@/utils/NotificationUtil'

/**
 * 远程文件自动保存 composable
 * 监听文档内容变化，debounce 2秒后自动上传回远程服务器
 */
export function useRemoteAutoSave() {
  const docStore = useDocumentManagerStore()
  let saveTimer: ReturnType<typeof setTimeout> | null = null
  let isSaving = false

  // 监听所有文档的内容变化
  const stopWatch = watch(
    () => {
      // 收集所有有变化的远程文档
      const dirtyRemoteDocs: string[] = []
      for (const [key, doc] of docStore.documents) {
        if (doc.isRemote && doc.isDirty && doc.localTempPath && doc.remotePath && doc.sessionId) {
          dirtyRemoteDocs.push(key)
        }
      }
      return dirtyRemoteDocs.join(',')
    },
    (dirtyKeys) => {
      if (!dirtyKeys) return

      // 清除之前的定时器
      if (saveTimer) {
        clearTimeout(saveTimer)
      }

      // debounce 2秒后自动保存
      saveTimer = setTimeout(async () => {
        if (isSaving) return
        isSaving = true

        const keys = dirtyKeys.split(',').filter(Boolean)
        for (const key of keys) {
          try {
            const success = await docStore.saveRemoteDocument(key)
            if (success) {
              const doc = docStore.documents.get(key)
              if (doc) {
                console.log(`[AutoSave] 远程文件已保存: ${doc.remotePath}`)
              }
            }
          } catch (error) {
            console.error(`[AutoSave] 自动保存失败: ${key}`, error)
          }
        }

        isSaving = false
      }, 2000)
    },
    { immediate: false }
  )

  onUnmounted(() => {
    if (saveTimer) {
      clearTimeout(saveTimer)
    }
    stopWatch()
  })

  return {
    isSaving
  }
}
