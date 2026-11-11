/**
 * 生成内置插件清单 `src/plugins/builtin.generated.ts`。
 *
 * 用法：
 *   node scripts/gen-builtin-plugins.mjs                  # 全部内置插件（默认）
 *   node scripts/gen-builtin-plugins.mjs database         # 只要 database（settings 始终保留）
 *   node scripts/gen-builtin-plugins.mjs database,logs
 *
 * 为什么用「生成代码」而不是运行时过滤：
 *   插件入口是**静态 import**，不生成的话整棵插件树都会被 rollup 打进 bundle，
 *   运行时禁用只是不显示、代码还在。要真正瘦身只能在构建前把 import 列表改掉。
 *
 * ## 元数据来自 `plugin.json`，不再写在代码里
 *
 * 每个内置插件目录下都有一份 `plugin.json`（`id` / `name` / `description`），**与第三方插件
 * 清单同一份 schema**：本脚本的合法键集合直接从 `src-tauri/src/plugin_host/host.rs` 的
 * `MANIFEST_KEYS` 解析（单一事实源，避免两处硬编码漂移），键写错在这里就报错，而不是
 * 装完以后表现为「我明明写了却没生效」。
 *
 * 于是插件的 `index.ts` 只提供组件与宿主编排字段（order / pinnedBottom / 侧栏宽度等），
 * 由 `BuiltinPluginImpl` 类型强制它不能再写元数据 —— 元数据只有 `plugin.json` 一个真源。
 *
 * 与后端的关系：这里的 id 必须与 Cargo feature 同开同关（files→plugin-files …），
 * 否则会出现「前端有入口、后端无命令」。由 .workbuddy/tools/verify-plugin-selection.mjs 守着。
 */
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const OUT = resolve(ROOT, 'src/plugins/builtin.generated.ts')
const PLUGIN_DIR = resolve(ROOT, 'src/plugins')
const HOST_RS = resolve(ROOT, 'src-tauri/src/plugin_host/host.rs')

/** 全部内置插件，数组顺序 = 默认排列顺序（order 字段会被设置里的排序覆盖） */
const ALL = [
  { id: 'files', comment: '文件管理：目录浏览、打开与编辑、最近打开与会话恢复' },
  { id: 'ssh', comment: 'SSH 终端与 SFTP 文件传输，支持远程日志拉取' },
  { id: 'database', comment: '数据库连接、SQL 执行、表数据编辑与导入导出' },
  { id: 'logs', comment: '大日志文件的服务端索引、筛选、搜索与流式读取' },
  { id: 'mat', comment: 'Java 堆转储（HPROF）分析：直方图、泄漏嫌疑、引用追踪' },
  { id: 'terminal', comment: '本地终端（底部停靠面板）' },
  { id: 'web3d', comment: '3D 与游戏：独立窗口型插件的集中入口与兜底' },
  { id: 'placeholder', comment: '占位插件：无侧栏，用于验证布局适配（验证完可删）' },
  { id: 'settings', comment: '设置：宿主级常驻入口（pinnedBottom，不可禁用）', pinned: true },
]

/** 有后端模块的插件 → 对应 Cargo feature（守卫脚本按同一规则校验，勿改命名） */
const FEATURE_OF = {
  files: 'plugin-files',
  ssh: 'plugin-ssh',
  database: 'plugin-database',
  logs: 'plugin-logs',
  mat: 'plugin-mat',
  terminal: 'plugin-terminal',
}

/**
 * 第三方清单的合法键集合：从 Rust 常量解析。
 * 内置清单必须能被同一套校验接受，这才是「同一份 schema」的硬证据。
 */
function readManifestKeys() {
  const src = readFileSync(HOST_RS, 'utf-8')
  const m = src.match(/const MANIFEST_KEYS:\s*\[&str;\s*\d+\]\s*=\s*\[([\s\S]*?)\n\];/)
  if (!m) {
    throw new Error(`[gen-builtin-plugins] 无法从 ${HOST_RS} 解析 MANIFEST_KEYS`)
  }
  return [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1])
}

/**
 * 读取并校验一个内置插件的清单。
 *
 * `dependencies` 的目标限定为**已知内置插件 id**：内置插件不能依赖第三方插件
 * （第三方可能根本没装，而内置插件是编译期就定死的，依赖一个可选的东西没有意义）。
 * 第三方插件之间的依赖由宿主在运行期校验（后端只校验声明本身是否合法）。
 */
function readManifest(id, keys, knownIds) {
  const rel = `src/plugins/${id}/plugin.json`
  const file = resolve(PLUGIN_DIR, id, 'plugin.json')
  if (!existsSync(file)) {
    throw new Error(`[gen-builtin-plugins] 缺少清单：${rel}`)
  }
  let raw
  try {
    raw = JSON.parse(readFileSync(file, 'utf-8'))
  } catch (e) {
    throw new Error(`[gen-builtin-plugins] ${rel} 不是合法 JSON：${e.message}`)
  }
  const unknown = Object.keys(raw).filter((k) => !keys.includes(k))
  if (unknown.length) {
    throw new Error(
      `[gen-builtin-plugins] ${rel} 含第三方清单不认识的键：${unknown.join(', ')}\n` +
        `  合法键：${keys.join(', ')}`
    )
  }
  if (raw.id !== id) {
    throw new Error(`[gen-builtin-plugins] ${rel} 的 id（${raw.id}）与目录名（${id}）不一致`)
  }
  for (const f of ['name', 'description']) {
    if (typeof raw[f] !== 'string' || !raw[f].trim()) {
      throw new Error(`[gen-builtin-plugins] ${rel} 的 ${f} 必须是非空字符串`)
    }
  }
  if (raw.dependencies !== undefined) {
    if (!Array.isArray(raw.dependencies) || raw.dependencies.some((d) => typeof d !== 'string' || !d)) {
      throw new Error(`[gen-builtin-plugins] ${rel} 的 dependencies 必须是非空字符串数组`)
    }
    for (const d of raw.dependencies) {
      if (d === id) {
        throw new Error(`[gen-builtin-plugins] ${rel} 的 dependencies 不能指向自己（${d}）`)
      }
      if (!knownIds.includes(d)) {
        throw new Error(
          `[gen-builtin-plugins] ${rel} 依赖了未知内置插件 id：${d}\n` +
            `  已知：${knownIds.join(', ')}`
        )
      }
    }
  }
  return raw
}

/**
 * 依赖闭包：选中 X 时把 X 的依赖（递归）一起带上。
 *
 * 与后端 Cargo feature 同一条规则 —— `plugin-database = ["plugin-files", ...]`，
 * 所以「只要 database」实际上也会把 files 编进来。两边口径必须一致，否则会出现
 * 「前端裁掉 files、后端仍编了 files 命令」的错位。
 */
function expandDependencies(ids, depsOf) {
  const picked = new Set(ids)
  let grew = true
  while (grew) {
    grew = false
    for (const id of [...picked]) {
      for (const dep of depsOf(id)) {
        if (!picked.has(dep)) {
          picked.add(dep)
          grew = true
        }
      }
    }
  }
  return [...picked]
}

const arg = (process.argv[2] || '').trim()
const requested = arg ? arg.split(',').map((s) => s.trim()).filter(Boolean) : null

if (requested) {
  const unknown = requested.filter((id) => !ALL.some((p) => p.id === id))
  if (unknown.length) {
    console.error(`[gen-builtin-plugins] 未知插件 id: ${unknown.join(', ')}`)
    console.error(`可选项: ${ALL.map((p) => p.id).join(', ')}`)
    process.exit(1)
  }
}

const MANIFEST_KEYS = readManifestKeys()
const ALL_IDS = ALL.map((p) => p.id)
// 先读全部清单：依赖闭包要在「还没裁剪」的全集上算，否则选了 database 却不知道它依赖 files
const allManifests = Object.fromEntries(
  ALL.map((p) => [p.id, readManifest(p.id, MANIFEST_KEYS, ALL_IDS)])
)
const depsOf = (id) => allManifests[id]?.dependencies ?? []

const resolved = requested ? expandDependencies(requested, depsOf) : null
if (resolved) {
  const brought = resolved.filter((id) => !requested.includes(id))
  if (brought.length) {
    console.log(`[gen-builtin-plugins] 依赖带入：${brought.join(', ')}`)
  }
}

// settings 是宿主级入口：不打包它，用户就再也找不到设置页 —— 始终保留
const selected = resolved
  ? ALL.filter((p) => resolved.includes(p.id) || p.pinned)
  : ALL

const manifests = Object.fromEntries(selected.map((p) => [p.id, allManifests[p.id]]))

const imports = selected.map((p) => `import ${p.id}Plugin from './${p.id}'`).join('\n')
const items = selected
  .map((p) => {
    const m = `MANIFESTS.${p.id}`
    return (
      `  // ${p.comment}\n` +
      `  {\n` +
      `    id: ${m}.id,\n` +
      `    label: ${m}.name,\n` +
      `    description: ${m}.description,\n` +
      `    ...${p.id}Plugin,\n` +
      // 放在展开之后：依赖只能来自清单，插件实现体里写不了（BuiltinPluginImpl 已 Omit）
      `    dependencies: ${JSON.stringify(depsOf(p.id))},\n` +
      `  },`
    )
  })
  .join('\n')

const featureNote = Object.entries(FEATURE_OF)
  .map(([k, v]) => `${k}→${v}`)
  .join('、')

const header = `/* eslint-disable */
// 自动生成，**请勿手改**：由 \`scripts/gen-builtin-plugins.mjs\` 产出（构建前置步骤）。
${
  requested
    ? `// 本次裁剪：${selected.map((p) => p.id).join(', ')}（未打包：${ALL.filter((p) => !selected.includes(p)).map((p) => p.id).join(', ')}）`
    : '// 本次为全量内置插件（未指定裁剪参数）'
}
//
// 与后端 Cargo feature 的对应关系：${featureNote}。web3d / settings / placeholder 只有前端。
//
// 元数据（id / label / description）来自各插件目录下的 \`plugin.json\`，与第三方插件同一份
// schema（键集合由 \`plugin_host/host.rs\` 的 MANIFEST_KEYS 校验）；插件的 index.ts 只提供组件与
// 宿主编排字段（order / pinnedBottom / 侧栏宽度）。
//
// 注意：这里是「**编译进应用**的插件清单」，与「当前注册了哪些插件」是两件事，不能合并：
// 内置插件也支持运行期启用/禁用，禁用会把它从 \`pluginManager\` 注销（活动栏与布局都不再为它留位）。
// 如果设置页的可见性只看 \`pluginManager.plugins\`，被禁用的内置插件连卡片都会消失 ——
// 那个把它开回来的开关就再也找不到了。
//
// 依赖按**闭包**展开：选了 database 会带上 files，因为 database 的脚本管理复用 files 的文件
// 读写命令（后端 Cargo feature 也是同一条规则：plugin-database = ["plugin-files", ...]）。
// 两边必须同开同关，否则会出现「前端裁掉 files、后端仍编了 files 命令」的错位。
import type { ToolPlugin } from '@/core/pluginManager'
`

const body = `${imports}

/**
 * 本次打包进来的内置插件清单（元数据真源 = 各插件目录下的 \`plugin.json\`）。
 *
 * 刻意**不导出**：目前没有任何消费点，等有真实消费点时再导出
 * 名字与简介也没有消费点，声明了没人读只会变成死数据。
 */
const MANIFESTS = ${JSON.stringify(manifests, null, 2)} as const

export const BUILTIN_PLUGINS: ToolPlugin[] = [
${items}
]

/** 本次构建打包了哪些内置插件（供守卫脚本与运行时展示） */
export const BUILTIN_PLUGIN_IDS_BUILT: string[] = [${selected
  .map((p) => `'${p.id}'`)
  .join(', ')}]
`

mkdirSync(dirname(OUT), { recursive: true })
writeFileSync(OUT, header + body, 'utf-8')
console.log(
  `[gen-builtin-plugins] 已生成 src/plugins/builtin.generated.ts：${
    selected.map((p) => p.id).join(', ')
  }`
)
