import { ElNotification } from 'element-plus'

const NotificationUtil = {
  info: (description: string = '', duration: number = 3000, onClose?: () => void) => {
    ElNotification({
      title: 'Info',
      message: description,
      type: 'info',
      offset: 50,
      duration: duration,
      onClose: onClose
    })
  },
  warn: (description: string = '', duration: number = 3000, onClose?: () => void) => {
    ElNotification({
      title: 'Warning',
      message: description,
      type: 'warning',
      offset: 50,
      duration: duration,
      onClose: onClose
    })
  },
  success: (description: string = '', duration: number = 3000, onClose?: () => void) => {
    ElNotification({
      title: 'Success',
      message: description,
      type: 'success',
      offset: 50,
      duration: duration,
      onClose: onClose
    })
  },
  error: (description: string = '', duration: number = 3000, onClose?: () => void) => {
    ElNotification({
      title: 'Error',
      message: description,
      type: 'error',
      offset: 50,
      duration: duration,
      onClose: onClose
    })
  }
}

export default NotificationUtil
