import { pluginManager } from '@/core/pluginManager'
import filesPlugin from './files'
import sshPlugin from './ssh'
import databasePlugin from './database'
import logsPlugin from './logs'
import matPlugin from './mat'

/** 注册所有内置插件 */
export function registerBuiltinPlugins() {
  pluginManager.register(filesPlugin)
  pluginManager.register(sshPlugin)
  pluginManager.register(logsPlugin)
  pluginManager.register(databasePlugin)
  pluginManager.register(matPlugin)
}
