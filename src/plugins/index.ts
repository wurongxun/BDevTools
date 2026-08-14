import { pluginManager } from '@/core/pluginManager'
import filesPlugin from './files'
import sshPlugin from './ssh'
import databasePlugin from './database'

/** 注册所有内置插件 */
export function registerBuiltinPlugins() {
  pluginManager.register(filesPlugin)
  pluginManager.register(sshPlugin)
  pluginManager.register(databasePlugin)
}
