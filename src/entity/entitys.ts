class SettingMap {
  public id: string = ''
  public app_setting_item_name: string = ''
  public app_setting_item_value: string = ''
}
class ResponseData<T> {
  public code: number
  public data: T
  public msg: string = ''
  constructor(code: number, data: T, msg: string) {
    this.code = code
    this.data = data
    this.msg = msg
  }
}

class SettingIdValueName<T> {
  public id: string
  public app_setting_item_name: string
  public app_setting_item_value: T
  constructor(id: string, app_setting_item_name: string, app_setting_item_value: T) {
    this.id = id
    this.app_setting_item_name = app_setting_item_name
    this.app_setting_item_value = app_setting_item_value
  }
}

export {
  SettingMap,
  ResponseData,
  SettingIdValueName
}
