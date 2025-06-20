import { ElNotification } from "element-plus";
let _ = {
  info: function (description: string) {
    ElNotification({
      title: "Info",
      message: description,
      type: "info",
    });
  },
  warn: function (description: string) {
    ElNotification({
      title: "Warning",
      message: description,
      type: "warning",
    });
  },
  success: function (description: string) {
    ElNotification({
      title: "Success",
      message: description,
      type: "success",
    });
  },
};

export default _;
