import { ElNotification } from "element-plus";

const NotificationUtil = {
  info: (description: string) => {
    ElNotification({
      title: "Info",
      message: description,
      type: "info",
    });
  },
  warn: (description: string) => {
    ElNotification({
      title: "Warning",
      message: description,
      type: "warning",
    });
  },
  success: (description: string) => {
    ElNotification({
      title: "Success",
      message: description,
      type: "success",
    });
  },
  error: (description: string) => {
    ElNotification({
      title: "Error",
      message: description,
      type: "error",
    });
  }
};

export default NotificationUtil;