/**
 * 命令贡献点：命令面板 / 快捷键 / 菜单的统一入口。
 *
 * ## 为什么命令是「贡献」而不是插件方法
 *
 * 命令让插件**暴露能力而不暴露自己**：调用方只拿一个字符串 ID，不需要 import 任何模块。
 * 这就是「插件 A 与插件 B 互不知道对方存在」的落点 —— 它们唯一的交互面是这张命令表。
 *
 * ## 两类命令
 *
 * - **插件自己的命令**：插件调 `registerCommands`（「切换到 X」就是每个内置插件注册的第一条，
 *   注册动作在 `pluginManager.register` 里统一发生）。随插件停用一起消失 ——
 *   注册表按贡献者整组撤销，见 `contributionRegistry.withdraw`。
 * - **宿主命令**：菜单 / 快捷键层面的宿主能力（打开设置、切主题…），由 UI 层注册，
 *   因为它们要动 layout store 与主题，那些依赖不该反向塞进 `core/`。
 */
import { contributionRegistry } from '@/core/registry'
import type { CommandItem, CommandPayload } from '@/core/registry'

/** 宿主自身的贡献者 ID（不与任何插件 ID 冲突） */
export const HOST_CONTRIBUTOR = 'host'

/**
 * 注册命令（追加到该插件已有的贡献上）。
 *
 * 条目 ID 就是命令 ID，所以同一个命令重复注册只会生效一次。
 */
export function registerCommands(pluginId: string, cmds: CommandPayload[]): void {
  contributionRegistry.add(
    pluginId,
    cmds.map(c => ({ kind: 'command' as const, id: c.command, payload: c })),
  )
}

/** 全部可用命令（`when()` 为假的已剔除，例如「没有连接时隐藏断开连接」） */
export function allCommands(): CommandItem[] {
  return contributionRegistry.commands().filter(e => e.payload.when?.() !== false)
}

/**
 * 按命令 ID 执行。
 *
 * @returns 是否找到并执行（**不含执行是否抛错** —— 异常向上抛，由调用方决定怎么提示用户，
 *   静默吞掉只会变成最难排查的「点了没反应」）。
 */
export async function executeCommand(command: string): Promise<boolean> {
  const entry = contributionRegistry.get<CommandPayload>('command', command)
    ?? allCommands().find(e => e.payload.command === command)
  if (!entry) return false
  await entry.payload.run()
  return true
}
