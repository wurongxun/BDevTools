import { pluginManager } from '@/core/pluginManager'
import filesPlugin from './files'
import sshPlugin from './ssh'

/** 注册所有内置插件 */
export function registerBuiltinPlugins() {
  pluginManager.register(filesPlugin)
  pluginManager.register(sshPlugin)
}
