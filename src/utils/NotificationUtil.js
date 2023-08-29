import {notification } from 'ant-design-vue';
let duration=3.5;
let placement='bottomRight'
let _ = {}
_.error= function (description) {
    notification.error({
        duration: duration,
        message: `通知`,
        description: description,
        placement: placement,
      });
}
_.success= function (description) {
    notification.success({
        duration: duration,
        message: `通知`,
        description: description,
        placement: placement,
      });
}
_.warning= function (description) {
    notification.warning({
        duration: duration,
        message: `通知`,
        description: description,
        placement: placement,
      });
}
export default _;