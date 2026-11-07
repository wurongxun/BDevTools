/* eslint-disable */
// 自动生成，**请勿手改**：由 `scripts/gen-builtin-plugins.mjs` 产出（构建前置步骤）。
// 本次为全量内置插件（未指定裁剪参数）
//
// 与后端 Cargo feature 的对应关系：files→plugin-files、ssh→plugin-ssh、database→plugin-database、logs→plugin-logs、mat→plugin-mat、terminal→plugin-terminal。web3d / settings / placeholder 只有前端。
//
// 元数据（id / label / description）来自各插件目录下的 `plugin.json`，与第三方插件同一份
// schema（键集合由 `plugin_host/host.rs` 的 MANIFEST_KEYS 校验）；插件的 index.ts 只提供组件与
// 宿主编排字段（order / pinnedBottom / 侧栏宽度）。
//
// 注意：这里是「**编译进应用**的插件清单」，与「当前注册了哪些插件」是两件事，不能合并：
// 内置插件也支持运行期启用/禁用，禁用会把它从 `pluginManager` 注销（活动栏与布局都不再为它留位）。
// 如果设置页的可见性只看 `pluginManager.plugins`，被禁用的内置插件连卡片都会消失 ——
// 那个把它开回来的开关就再也找不到了。
//
// 依赖按**闭包**展开：选了 database 会带上 files，因为 database 的脚本管理复用 files 的文件
// 读写命令（后端 Cargo feature 也是同一条规则：plugin-database = ["plugin-files", ...]）。
// 两边必须同开同关，否则会出现「前端裁掉 files、后端仍编了 files 命令」的错位。
import type { ToolPlugin } from '@/core/pluginManager'
import filesPlugin from './files'
import sshPlugin from './ssh'
import databasePlugin from './database'
import logsPlugin from './logs'
import matPlugin from './mat'
import terminalPlugin from './terminal'
import web3dPlugin from './web3d'
import placeholderPlugin from './placeholder'
import settingsPlugin from './settings'

/**
 * 本次打包进来的内置插件清单（元数据真源 = 各插件目录下的 `plugin.json`）。
 *
 * 刻意**不导出**：目前没有任何消费点，等有真实消费点时再导出
 * 名字与简介也没有消费点，声明了没人读只会变成死数据。
 */
const MANIFESTS = {
  "files": {
    "id": "files",
    "name": "文件",
    "description": "文件管理：目录浏览、打开与编辑、最近打开与会话恢复"
  },
  "ssh": {
    "id": "ssh",
    "name": "SSH 连接",
    "description": "SSH 终端与 SFTP 文件传输，支持远程日志拉取"
  },
  "database": {
    "id": "database",
    "name": "数据库",
    "description": "多数据库连接管理：SQL 执行、表数据浏览与导入导出",
    "dependencies": [
      "files"
    ]
  },
  "logs": {
    "id": "logs",
    "name": "日志",
    "description": "大文件日志工作台：分块懒加载、正则检索、级别配色与高亮规则"
  },
  "mat": {
    "id": "mat",
    "name": "内存分析",
    "description": "Java 堆转储分析：直方图、泄漏检测、支配树、引用追踪、OQL、多 dump 对比与专用报告（支持 .hprof.gz 与本地索引加速）"
  },
  "terminal": {
    "id": "terminal",
    "name": "终端",
    "description": "本地交互式终端：多标签 / 分屏 / 可拖拽停靠，支持 shell 集成（cwd、退出码）"
  },
  "web3d": {
    "id": "web3d",
    "name": "3D 与游戏",
    "description": "在新窗口里运行声明了页面能力 / 独立窗口的第三方 3D / 游戏插件"
  },
  "placeholder": {
    "id": "placeholder",
    "name": "占位示例",
    "description": "无侧栏插件的布局适配示例（只有主面板，无侧栏/标签条）"
  },
  "settings": {
    "id": "settings",
    "name": "设置",
    "description": "应用设置、窗口预设与插件管理"
  }
} as const

export const BUILTIN_PLUGINS: ToolPlugin[] = [
  // 文件管理：目录浏览、打开与编辑、最近打开与会话恢复
  {
    id: MANIFESTS.files.id,
    label: MANIFESTS.files.name,
    description: MANIFESTS.files.description,
    ...filesPlugin,
    dependencies: [],
  },
  // SSH 终端与 SFTP 文件传输，支持远程日志拉取
  {
    id: MANIFESTS.ssh.id,
    label: MANIFESTS.ssh.name,
    description: MANIFESTS.ssh.description,
    ...sshPlugin,
    dependencies: [],
  },
  // 数据库连接、SQL 执行、表数据编辑与导入导出
  {
    id: MANIFESTS.database.id,
    label: MANIFESTS.database.name,
    description: MANIFESTS.database.description,
    ...databasePlugin,
    dependencies: ["files"],
  },
  // 大日志文件的服务端索引、筛选、搜索与流式读取
  {
    id: MANIFESTS.logs.id,
    label: MANIFESTS.logs.name,
    description: MANIFESTS.logs.description,
    ...logsPlugin,
    dependencies: [],
  },
  // Java 堆转储（HPROF）分析：直方图、泄漏嫌疑、引用追踪
  {
    id: MANIFESTS.mat.id,
    label: MANIFESTS.mat.name,
    description: MANIFESTS.mat.description,
    ...matPlugin,
    dependencies: [],
  },
  // 本地终端（底部停靠面板）
  {
    id: MANIFESTS.terminal.id,
    label: MANIFESTS.terminal.name,
    description: MANIFESTS.terminal.description,
    ...terminalPlugin,
    dependencies: [],
  },
  // 3D 与游戏：独立窗口型插件的集中入口与兜底
  {
    id: MANIFESTS.web3d.id,
    label: MANIFESTS.web3d.name,
    description: MANIFESTS.web3d.description,
    ...web3dPlugin,
    dependencies: [],
  },
  // 占位插件：无侧栏，用于验证布局适配（验证完可删）
  {
    id: MANIFESTS.placeholder.id,
    label: MANIFESTS.placeholder.name,
    description: MANIFESTS.placeholder.description,
    ...placeholderPlugin,
    dependencies: [],
  },
  // 设置：宿主级常驻入口（pinnedBottom，不可禁用）
  {
    id: MANIFESTS.settings.id,
    label: MANIFESTS.settings.name,
    description: MANIFESTS.settings.description,
    ...settingsPlugin,
    dependencies: [],
  },
]

/** 本次构建打包了哪些内置插件（供守卫脚本与运行时展示） */
export const BUILTIN_PLUGIN_IDS_BUILT: string[] = ['files', 'ssh', 'database', 'logs', 'mat', 'terminal', 'web3d', 'placeholder', 'settings']
