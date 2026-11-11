#!/usr/bin/env node
/**
 * 发版脚本（Windows + Gitee）
 * ==========================================================================
 * 一条命令打通「自动安装更新」的发布侧：
 *
 *   版本号 → 构建并签名 → 本地校验签名 → 上传 Gitee Release → 更新仓库里的 latest.json
 *
 * 用法
 *   node scripts/release.mjs all                  # 完整发版（构建 + 发布）
 *   node scripts/release.mjs build                # 只构建 + 签名 + 校验
 *   node scripts/release.mjs verify [文件...]     # 校验签名；不给文件则校验构建产物
 *   node scripts/release.mjs publish              # 只发布（复用已有产物）
 *
 * 环境变量
 *   GITEE_TOKEN                        Gitee 私人令牌（publish 必需；需勾 projects 权限）
 *                                      也可放 .workbuddy/release.env 或 ~/.gitee_token（避免进命令行历史）
 *   TAURI_SIGNING_PRIVATE_KEY_PATH     私钥路径，默认 ~/.tauri/backend_dev_tools.key
 *   TAURI_SIGNING_PRIVATE_KEY_PASSWORD 私钥口令。**不设**时按此顺序解析，全没有则按空口令：
 *                                        1) 本环境变量（显式设成空串也算「已指定」）
 *                                        2) 与私钥同路径 + `.password` 的文件（例：
 *                                           ~/.tauri/backend_dev_tools.key.password，取首行）
 *                                        3) 空串
 *                                      为什么必须显式给：tauri CLI 拿不到「已给定口令」的信号时会
 *                                      转交互式密码提示，在脚本/CI 里表现为一直挂住而不是报错。
 *   GITEE_OWNER / GITEE_REPO / GITEE_BRANCH / GITEE_REMOTE   覆盖默认值
 *
 * 参数
 *   --bump patch|minor|major   先升版本号（同步改 tauri.conf.json / Cargo.toml / package.json）
 *   --version x.y.z            直接指定版本号
 *   --dry-run                  不发任何写请求，只打印将要发生什么
 *   --force                    同名 release 已存在时先删除再重建
 *   --skip-build               跳过构建（产物已就绪）
 *   --skip-sync-check          跳过「本地 HEAD 是否已推送到远端」检查
 *   --prerelease               标记为预发布
 *   --notes "..."              发布说明
 *   --notes-file <路径>        从文件读发布说明
 *   --no-commit-manifest       只写本地 latest.json，不通过 API 提交到仓库
 */

import { spawnSync } from 'node:child_process'
import { createHash, createPublicKey, verify as edVerify } from 'node:crypto'
import { existsSync } from 'node:fs'
import { readFile, writeFile, readdir, rm } from 'node:fs/promises'
import { homedir, tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

// ==================== 路径与常量 ====================

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const CONF_FILE = path.join(ROOT, 'src-tauri', 'tauri.conf.json')
const CARGO_FILE = path.join(ROOT, 'src-tauri', 'Cargo.toml')
const NPM_FILE = path.join(ROOT, 'package.json')
const BUNDLE_NSIS = path.join(ROOT, 'src-tauri', 'target', 'release', 'bundle', 'nsis')
const MANIFEST_FILE = path.join(ROOT, 'latest.json')
/**
 * tauri 缓存的 NSIS 工具链目录（与已有的 WiX 工具同级）。
 * `tauri build --bundles nsis` 首次会往这里下载 makensis 及插件，缺了才联网。
 */
const NSIS_CACHE_DIR = process.env.LOCALAPPDATA
  ? path.join(process.env.LOCALAPPDATA, 'tauri', 'NSIS')
  : ''
/** 走 GitHub 镜像预置 NSIS 工具链的脚本（见其头部说明） */
const NSIS_FETCH_SCRIPT = path.join(ROOT, 'scripts', 'fetch-nsis.ps1')

const API = 'https://gitee.com/api/v5'
const OWNER = process.env.GITEE_OWNER || 'wurongxun'
const REPO = process.env.GITEE_REPO || 'BackendDevTools'
const BRANCH = process.env.GITEE_BRANCH || 'BackendDevToolsV1.4'
const REMOTE = process.env.GITEE_REMOTE || 'origin'
const MANIFEST_PATH_IN_REPO = 'latest.json'
/** 更新清单里 Windows 平台条目的 key（与 tauri-plugin-updater 的 target 一致） */
const PLATFORM_KEY = 'windows-x86_64'

const C = {
  dim: (s) => `\x1b[2m${s}\x1b[0m`,
  red: (s) => `\x1b[31m${s}\x1b[0m`,
  green: (s) => `\x1b[32m${s}\x1b[0m`,
  yellow: (s) => `\x1b[33m${s}\x1b[0m`,
  cyan: (s) => `\x1b[36m${s}\x1b[0m`,
  bold: (s) => `\x1b[1m${s}\x1b[0m`
}
const log = (...a) => console.log(...a)
const step = (n, t) => log(`\n${C.cyan(`[${n}]`)} ${C.bold(t)}`)
const ok = (t) => log(`  ${C.green('✓')} ${t}`)
const warn = (t) => log(`  ${C.yellow('!')} ${t}`)
const fail = (t) => {
  throw new Error(t)
}

// ==================== 命令行参数 ====================

function parseArgs(argv) {
  const opts = { command: argv[0] || 'all', files: [] }
  for (let i = 1; i < argv.length; i++) {
    const a = argv[i]
    if (a === '--bump') opts.bump = argv[++i]
    else if (a === '--version') opts.version = argv[++i]
    else if (a === '--notes') opts.notes = argv[++i]
    else if (a === '--notes-file') opts.notesFile = argv[++i]
    else if (a === '--dry-run') opts.dryRun = true
    else if (a === '--force') opts.force = true
    else if (a === '--skip-build') opts.skipBuild = true
    else if (a === '--skip-sync-check') opts.skipSyncCheck = true
    else if (a === '--prerelease') opts.prerelease = true
    else if (a === '--no-commit-manifest') opts.noCommitManifest = true
    else if (a.startsWith('--')) fail(`未知参数：${a}`)
    else opts.files.push(a)
  }
  if (!['all', 'build', 'publish', 'verify'].includes(opts.command)) {
    fail(`未知命令：${opts.command}（可用：all / build / publish / verify）`)
  }
  return opts
}

// ==================== 通用工具 ====================

async function readJson(file) {
  return JSON.parse(await readFile(file, 'utf8'))
}

/** 保持原文件换行风格地写回 JSON（避免整仓 CRLF/LF 混战） */
async function writeTextKeepingEol(file, text) {
  const cur = await readFile(file, 'utf8')
  const eol = cur.includes('\r\n') ? '\r\n' : '\n'
  await writeFile(file, text.replace(/\r?\n/g, eol), 'utf8')
}

function run(cmd, args, { cwd = ROOT, env, quiet = false } = {}) {
  const r = spawnSync(cmd, args, {
    cwd,
    env: { ...process.env, ...env },
    stdio: quiet ? ['ignore', 'pipe', 'pipe'] : 'inherit'
  })
  if (r.status !== 0) {
    const detail = quiet ? `\n${r.stdout?.toString() || ''}${r.stderr?.toString() || ''}` : ''
    fail(`命令失败（exit ${r.status}）：${cmd} ${args.join(' ')}${detail}`)
  }
  return r
}

/**
 * env 语义与 run() 一致：值设为 `undefined` 表示「从子进程环境里删掉这个键」
 * （Node 会忽略 env 中值为 undefined 的键，正好当删除用）。
 */
/**
 * tauri CLI 把环境变量 TAURI_SIGNING_PRIVATE_KEY_PATH 也当成 `--private-key-path` 的来源，
 * 而它和本脚本统一使用的 `--private-key`（**内容**）是 clap 互斥参数 —— 环境里只要残留这个
 * 变量，CLI 就会直接报 `cannot be used with` 并退出（连构建都进不去）。
 * 所以凡是要自己传私钥的地方，都要显式把它从子进程环境里剔掉。
 */
const DROP_KEY_PATH_ENV = { TAURI_SIGNING_PRIVATE_KEY_PATH: undefined }

function capture(cmd, args, { cwd = ROOT, env } = {}) {
  const r = spawnSync(cmd, args, {
    cwd,
    env: env ? { ...process.env, ...env } : process.env,
    stdio: ['ignore', 'pipe', 'pipe']
  })
  return { status: r.status, stdout: (r.stdout || '').toString().trim(), stderr: (r.stderr || '').toString().trim() }
}

function git(args) {
  return capture('git', args)
}

/**
 * 本地 tauri CLI 的 js 入口。
 * 不走 `node_modules/.bin/tauri(.cmd)`：那是 Windows 的批处理垫片，spawn 时要么依赖 shell 拼字符串
 * （带空格路径会散架）、要么直接 ENOENT。用当前 node 执行真实 js 入口最稳，也顺带锁死版本来源。
 */
function tauriEntry() {
  const js = path.join(ROOT, 'node_modules', '@tauri-apps', 'cli', 'tauri.js')
  if (!existsSync(js)) fail('未找到 tauri CLI（node_modules/@tauri-apps/cli/tauri.js），请先 pnpm install')
  return js
}

function runTauri(args, env) {
  return run(process.execPath, [tauriEntry(), ...args], { env })
}

// ==================== minisign 签名校验（纯 Node，无第三方依赖） ====================
//
// tauri 的更新签名就是 minisign 格式，只是被 base64 包了一层：
//   <文件>.sig  = base64( "untrusted comment: ...\n" + base64(alg|keyid|sig) + "\ntrusted comment: ...\n" + base64(全局签名)\n )
//   pubkey      = base64( "untrusted comment: minisign public key: <KEYID>\n" + base64(alg|keyid|pubkey) + "\n" )
// 校验 = ed25519 验签(公钥, 待签内容, 签名)。
//
// 待签内容由**签名**里的 alg 决定（公钥里的 alg 恒为 "Ed"，只是算法标识，别拿它去比对）：
//   "ED"（大写）= 对 BLAKE2b-512 摘要验签（minisign 默认走这个预哈希）
//   "Ed"（小写）= 直接对原文验签
// 这两个是反直觉的一对，写反了会稳定报「算法不匹配」——本地校验过的价值就在这儿。
//
// 这一步的意义：签名错了在本地是**完全静默**的——构建成功、上传成功，只有换机器的用户点更新时才报
// 「签名校验失败」。所以每次发版前必须先在本地把签名对一遍公钥。

/** Ed25519 公钥的 SPKI DER 前缀（后面接 32 字节裸公钥） */
const ED25519_SPKI_PREFIX = Buffer.from('302a300506032b6570032100', 'hex')
/** minisign 公钥里恒定的算法标识 */
const PUBKEY_ALG = 'Ed'
/** 签名算法标识：大写 ED = 预哈希（BLAKE2b-512），小写 Ed = 原文 */
const PREHASHED_ALG = 'ED'
const RAW_ALG = 'Ed'

function ed25519KeyObject(raw32) {
  return createPublicKey({
    key: Buffer.concat([ED25519_SPKI_PREFIX, raw32]),
    format: 'der',
    type: 'spki'
  })
}

function unwrap(raw) {
  // 配置里的 pubkey / .sig 文件内容都是 base64(文本)
  const text = Buffer.from(String(raw).trim(), 'base64').toString('utf8')
  return text.split(/\r?\n/).filter((l) => l.length > 0)
}

function parsePublicKey(raw) {
  const lines = unwrap(raw)
  const blobLine = lines.find((l) => !l.startsWith('untrusted comment') && !l.startsWith('trusted comment'))
  if (!blobLine) fail('公钥格式不对：找不到 base64 主体')
  const blob = Buffer.from(blobLine, 'base64')
  if (blob.length !== 42) fail(`公钥长度不对：期望 42 字节，实际 ${blob.length}`)
  return { alg: blob.subarray(0, 2).toString('latin1'), keyId: blob.subarray(2, 10), key: blob.subarray(10) }
}

function parseSignature(raw) {
  const lines = unwrap(raw)
  const blobLine = lines.find((l) => !l.startsWith('untrusted comment') && !l.startsWith('trusted comment'))
  if (!blobLine) fail('签名格式不对：找不到 base64 主体')
  const blob = Buffer.from(blobLine, 'base64')
  if (blob.length !== 74) fail(`签名长度不对：期望 74 字节，实际 ${blob.length}`)
  return { alg: blob.subarray(0, 2).toString('latin1'), keyId: blob.subarray(2, 10), sig: blob.subarray(10) }
}

const hex = (buf) => buf.toString('hex').toUpperCase()

/** 校验单个文件；返回 { ok, keyId?, reason? }，不抛异常（便于批量报告） */
function verifySignature(content, sigRaw, pubRaw) {
  try {
    const pub = parsePublicKey(pubRaw)
    const sig = parseSignature(sigRaw)
    if (hex(pub.keyId) !== hex(sig.keyId)) {
      return { ok: false, reason: `签名用的密钥（keyid ${hex(sig.keyId)}）与配置里的公钥（keyid ${hex(pub.keyId)}）不是同一把` }
    }
    if (pub.alg !== PUBKEY_ALG) return { ok: false, reason: `公钥算法标识异常：${pub.alg}` }
    if (sig.alg === PREHASHED_ALG) {
      // 预哈希：先对文件做 BLAKE2b-512，再验签
      const digest = createHash('blake2b512').update(content).digest()
      return edVerify(null, digest, ed25519KeyObject(pub.key), sig.sig)
        ? { ok: true, keyId: hex(pub.keyId) }
        : { ok: false, reason: 'ed25519 验签不通过（文件与签名不匹配）' }
    }
    if (sig.alg === RAW_ALG) {
      return edVerify(null, content, ed25519KeyObject(pub.key), sig.sig)
        ? { ok: true, keyId: hex(pub.keyId) }
        : { ok: false, reason: 'ed25519 验签不通过（文件与签名不匹配）' }
    }
    return { ok: false, reason: `未知的签名算法标识：${sig.alg}` }
  } catch (e) {
    return { ok: false, reason: e.message }
  }
}

// ==================== 版本号 ====================

function bumpVersion(v, kind) {
  const parts = String(v).split('.').map((n) => parseInt(n, 10))
  if (parts.length !== 3 || parts.some(Number.isNaN)) fail(`版本号不是 x.y.z 形式：${v}`)
  const [maj, min, pat] = parts
  if (kind === 'major') return `${maj + 1}.0.0`
  if (kind === 'minor') return `${maj}.${min + 1}.0`
  if (kind === 'patch') return `${maj}.${min}.${pat + 1}`
  fail(`--bump 只支持 patch / minor / major，收到：${kind}`)
}

async function currentVersion() {
  return (await readJson(CONF_FILE)).version
}

/** 三处版本号一起改：版本号漂了会导致「构建出来的包名/清单版本/应用自报版本」互相对不上 */
async function setVersion(next) {
  const targets = [
    { file: CONF_FILE, re: /("version"\s*:\s*")[^"]+(")/, label: 'src-tauri/tauri.conf.json' },
    { file: CARGO_FILE, re: /(^version\s*=\s*")[^"]+(")/m, label: 'src-tauri/Cargo.toml' },
    { file: NPM_FILE, re: /("version"\s*:\s*")[^"]+(")/, label: 'package.json' }
  ]
  for (const t of targets) {
    const before = await readFile(t.file, 'utf8')
    if (!t.re.test(before)) fail(`${t.label} 里找不到 version 字段`)
    const after = before.replace(t.re, `$1${next}$2`)
    if (after !== before) await writeTextKeepingEol(t.file, after)
    ok(`${t.label} → ${next}`)
  }
}

// ==================== 构建产物 ====================

async function findArtifact() {
  if (!existsSync(BUNDLE_NSIS)) return null
  const files = await readdir(BUNDLE_NSIS)
  const exe = files.find((f) => f.endsWith('-setup.exe'))
  if (!exe) return null
  const file = path.join(BUNDLE_NSIS, exe)
  const sigFile = `${file}.sig`
  if (!existsSync(sigFile)) fail(`找到安装包 ${exe}，但缺少签名文件 ${exe}.sig —— 检查 createUpdaterArtifacts 是否开启、签名密钥是否设置`)
  return { file, sigFile, name: exe }
}

// ==================== Gitee API ====================

async function gitee(method, apiPath, { token, query = {}, body, form, raw = false } = {}) {
  const url = new URL(API + apiPath)
  for (const [k, v] of Object.entries(query)) if (v !== undefined) url.searchParams.set(k, String(v))
  if (token) url.searchParams.set('access_token', token)

  const init = { method, headers: {} }
  if (form) {
    init.body = form // multipart：不要手写 Content-Type，交给 fetch 带 boundary
  } else if (body !== undefined) {
    init.headers['Content-Type'] = 'application/json;charset=UTF-8'
    init.body = JSON.stringify(body)
  }

  const res = await fetch(url, init)
  const text = await res.text()
  if (!res.ok) {
    let hint = ''
    if (res.status === 401) hint = '（令牌无效或没有 projects 权限）'
    if (res.status === 403) hint = '（令牌权限不足，或该操作需要写权限）'
    if (res.status === 404) hint = '（仓库/资源不存在，或令牌看不到它）'
    fail(`Gitee ${method} ${apiPath} → HTTP ${res.status}${hint}\n${text.slice(0, 500)}`)
  }
  if (raw) return text
  try {
    return text ? JSON.parse(text) : null
  } catch {
    return text
  }
}

async function listReleases(token) {
  return (await gitee('GET', `/repos/${OWNER}/${REPO}/releases`, { token, query: { per_page: 100 } })) || []
}

async function ensureRelease({ tag, name, body, token, force, prerelease, dryRun }) {
  const releases = await listReleases(token)
  const existing = releases.find((r) => r.tag_name === tag)
  if (existing) {
    if (!force) {
      fail(
        `release ${tag} 已存在（id=${existing.id}）。\n` +
          `  想覆盖请加 --force（会先删除该 release 再重建）`
      )
    }
    if (dryRun) {
      log(`  ${C.dim('[dry-run]')} 将删除已有 release ${tag}（id=${existing.id}）并重建`)
      return { id: existing.id, reused: false, dryRun: true }
    }
    await gitee('DELETE', `/repos/${OWNER}/${REPO}/releases/${existing.id}`, { token })
    ok(`已删除旧 release ${tag}（id=${existing.id}）`)
  }
  if (dryRun) {
    log(`  ${C.dim('[dry-run]')} 将创建 release ${tag}（target=${BRANCH}）`)
    return { id: 0, reused: false, dryRun: true }
  }
  const created = await gitee('POST', `/repos/${OWNER}/${REPO}/releases`, {
    token,
    body: { tag_name: tag, name, body, target_commitish: BRANCH, prerelease: !!prerelease }
  })
  ok(`已创建 release ${tag}（id=${created.id}）`)
  return { id: created.id, reused: false, dryRun: false }
}

async function uploadAsset(releaseId, file, token) {
  const buf = await readFile(file)
  const form = new FormData()
  form.append('file', new Blob([buf]), path.basename(file))
  await gitee('POST', `/repos/${OWNER}/${REPO}/releases/${releaseId}/attach_files`, { token, form })
  // 上传响应结构不稳定，统一回查一次附件列表拿 download url（唯一可信来源）
  const assets = await gitee('GET', `/repos/${OWNER}/${REPO}/releases/${releaseId}/attach_files`, { token })
  const hit = (assets || []).find((a) => a.name === path.basename(file))
  if (!hit) fail(`上传后回查不到附件 ${path.basename(file)}`)
  return hit
}

async function readRepoFile(relPath, token) {
  try {
    const r = await gitee('GET', `/repos/${OWNER}/${REPO}/contents/${relPath}`, { token, query: { ref: BRANCH } })
    if (r && r.sha) return { sha: r.sha, content: Buffer.from(r.content || '', 'base64').toString('utf8') }
  } catch {
    /* 文件不存在 → 新建 */
  }
  return null
}

async function commitManifest(text, token, message) {
  const cur = await readRepoFile(MANIFEST_PATH_IN_REPO, token)
  const payload = {
    content: Buffer.from(text, 'utf8').toString('base64'),
    message,
    branch: BRANCH
  }
  if (cur?.sha) payload.sha = cur.sha
  await gitee('PUT', `/repos/${OWNER}/${REPO}/contents/${MANIFEST_PATH_IN_REPO}`, {
    token,
    body: { ...payload, access_token: token }
  })
  return { created: !cur, sha: cur?.sha || null }
}

// ==================== 前置检查 ====================

function checkNotRunning() {
  if (process.platform !== 'win32') return
  const r = capture('tasklist', ['/FI', 'IMAGENAME eq backend_dev_tools.exe', '/NH'])
  if (/backend_dev_tools\.exe/i.test(r.stdout)) {
    fail(
      '检测到应用正在运行（backend_dev_tools.exe）。\n' +
        '  构建最后一步会因文件被占用而失败（拒绝访问 os error 5），请先关闭应用再重试。'
    )
  }
  ok('应用未在运行')
}

/**
 * 确保 NSIS 工具链已就位。
 *
 * `tauri build --bundles nsis` 首次会从 GitHub 下载 nsis-3.13.zip 与 nsis_tauri_utils.dll，
 * 国内直连常超时 → `timeout: global` → 整个发版在最后一步才中止（白等一轮 vite build + Rust 编译）。
 * 这里在构建**之前**先探测缓存，缺了就调用 scripts/fetch-nsis.ps1 走镜像预置好。
 *
 * 只在 Windows 上做（NSIS 仅 Windows 打包需要）；探测到已存在则直接跳过，幂等。
 */
function ensureNsisToolchain() {
  if (process.platform !== 'win32' || !NSIS_CACHE_DIR) return
  const makensis = path.join(NSIS_CACHE_DIR, 'makensis.exe')
  const utilsDll = path.join(NSIS_CACHE_DIR, 'Plugins', 'x86-unicode', 'nsis_tauri_utils.dll')
  if (existsSync(makensis) && existsSync(utilsDll)) {
    ok('NSIS 工具链已就位（跳过预置）')
    return
  }
  if (!existsSync(NSIS_FETCH_SCRIPT)) {
    warn(`未找到 NSIS 缓存（${NSIS_CACHE_DIR}），且缺少预置脚本 ${path.relative(ROOT, NSIS_FETCH_SCRIPT)}`)
    warn('tauri 将尝试自行从 GitHub 下载，若网络受限可能在打包最后一步超时失败')
    return
  }
  warn('NSIS 工具链缺失，先用镜像预置（否则 tauri 会在打包最后一步直连 GitHub 下载并可能超时）')
  run('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', NSIS_FETCH_SCRIPT])
  // 预置后再校验一次：缺文件就早失败，别浪费一轮编译
  if (!existsSync(makensis) || !existsSync(utilsDll)) {
    fail(
      `NSIS 工具链预置后仍不完整，检查 ${NSIS_CACHE_DIR}\n` +
        '  需要：makensis.exe 与 Plugins\\x86-unicode\\nsis_tauri_utils.dll\n' +
        `  可手动重跑：powershell -ExecutionPolicy Bypass -File ${path.relative(ROOT, NSIS_FETCH_SCRIPT)}`
    )
  }
  ok('NSIS 工具链预置完成')
}

function checkSync() {
  const head = git(['rev-parse', 'HEAD'])
  if (head.status !== 0) {
    warn('不是 git 仓库，跳过同步检查')
    return
  }
  const remote = git(['ls-remote', REMOTE, `refs/heads/${BRANCH}`])
  if (remote.status !== 0 || !remote.stdout) {
    warn(`查不到 ${REMOTE}/${BRANCH}，跳过同步检查（离线或分支不存在？）`)
    return
  }
  const remoteSha = remote.stdout.split(/\s+/)[0]
  if (remoteSha !== head.stdout.trim()) {
    const counts = git(['rev-list', '--left-right', '--count', `HEAD...${remoteSha}`])
    const [ahead, behind] = (counts.stdout || '').split(/\s+/).map((n) => parseInt(n, 10))
    fail(
      `本地 HEAD 与远端 ${REMOTE}/${BRANCH} 不一致，直接发版会让 release 指向旧代码。\n` +
        `  本地 ${head.stdout.trim().slice(0, 8)}（领先 ${ahead || '?'}） / 远端 ${remoteSha.slice(0, 8)}（领先 ${behind || '?'}）\n` +
        `  先把代码推上去（或先 pull 合并），确认无误后用 --skip-sync-check 强行跳过。`
    )
  }
  ok(`本地 HEAD 与远端一致（${head.stdout.trim().slice(0, 8)}）`)
}

// ==================== 各命令 ====================

/**
 * 私钥口令来源（按优先级）：
 *   1. 环境变量 TAURI_SIGNING_PRIVATE_KEY_PASSWORD —— 显式设置即生效，`= ''`（空串）也算「已指定」
 *   2. 私钥同路径 + `.password` 的文件（如 ~/.tauri/backend_dev_tools.key.password），取首行并 trim
 *   3. 空串
 * 密钥可以完全不带口令，那时第 3 条自然命中，一切照旧。
 */
async function resolveKeyPassword(keyPath) {
  const env = process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD
  if (env !== undefined) {
    return { value: env, from: '环境变量 TAURI_SIGNING_PRIVATE_KEY_PASSWORD' }
  }
  const file = `${keyPath}.password`
  if (existsSync(file)) {
    const first = (await readFile(file, 'utf8')).split(/\r?\n/)[0].trim()
    return { value: first, from: `口令文件 ${file}` }
  }
  return { value: '', from: '未配置口令（按空口令处理）' }
}

/** `~/.tauri` 下现成的私钥 —— 找不到默认钥匙时列出来提示，避免「钥匙明明在却只报找不到」 */
async function listKeyCandidates() {
  const dir = path.join(homedir(), '.tauri')
  if (!existsSync(dir)) return []
  try {
    return (await readdir(dir))
      .filter((f) => f.endsWith('.key'))
      .map((f) => path.join(dir, f))
  } catch {
    return []
  }
}

/**
 * 签名自检：用私钥签一个临时文件，再拿 tauri.conf.json 里的公钥验签。
 * 把「口令不对」「私钥与配置里公钥不是一对」这两类错误提前到构建**之前**暴露 —— 否则要等
 * vite build + Rust 编译 + 打包（几分钟）跑完才在签名步骤报错，白等一轮。
 * 返回 { ok, keyId?, message? }；临时文件在 finally 里清掉（fail() 会直接退出进程，
 * 所以函数内部不调 fail，由调用方决定）。
 */
async function selfCheckSigningKey(ctx) {
  const probe = path.join(tmpdir(), `bdt-signing-probe-${Date.now()}.txt`)
  await writeFile(probe, 'bdt-updater-signing-selfcheck\n', 'utf8')
  try {
    const r = capture(
      process.execPath,
      [
        tauriEntry(),
        'signer',
        'sign',
        '-k',
        await readFile(ctx.keyPath, 'utf8'),
        '-p',
        ctx.keyPassword,
        probe
      ],
      {
        // 私钥与口令都从命令行显式给，环境里同名的变量要剔掉，否则撞上 clap 互斥
        env: {
          ...DROP_KEY_PATH_ENV,
          TAURI_SIGNING_PRIVATE_KEY: undefined,
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: undefined
        }
      }
    )
    if (r.status !== 0) {
      const detail = `${r.stdout}\n${r.stderr}`.trim()
      if (/password/i.test(detail)) {
        return {
          ok: false,
          message:
            `私钥口令不正确（当前来源：${ctx.passwordFrom}）\n` +
            `  该私钥设了口令，发版脚本必须能拿到它，三选一：\n` +
            `    1) 写进文件（一次生效）：把口令写到 ${ctx.keyPath}.password 的首行\n` +
            `    2) 临时给：TAURI_SIGNING_PRIVATE_KEY_PASSWORD=xxx pnpm release --bump patch\n` +
            `    3) 换成不带口令的密钥：pnpm exec tauri signer generate -w <路径> -p ""\n` +
            `  原始错误：${detail}`
        }
      }
      return { ok: false, message: `私钥不可用：${detail}` }
    }
    const check = verifySignature(
      await readFile(probe),
      await readFile(`${probe}.sig`, 'utf8'),
      ctx.pubkey
    )
    if (!check.ok) {
      return {
        ok: false,
        message:
          `私钥与 tauri.conf.json 里的公钥不配对：${check.reason}\n` +
          `  这把私钥签出来的包，到了用户机器上会被更新器拒绝安装。\n` +
          `  请确认 plugins.updater.pubkey 是「${ctx.keyPath}」对应的公钥。`
      }
    }
    return { ok: true, keyId: check.keyId }
  } finally {
    await rm(probe, { force: true })
    await rm(`${probe}.sig`, { force: true })
  }
}

async function loadUpdaterContext() {
  const conf = await readJson(CONF_FILE)
  const pubkey = conf?.plugins?.updater?.pubkey
  if (!pubkey) fail('tauri.conf.json 里没有 plugins.updater.pubkey')
  const endpoints = conf?.plugins?.updater?.endpoints || []
  const keyPath =
    process.env.TAURI_SIGNING_PRIVATE_KEY_PATH ||
    path.join(homedir(), '.tauri', 'backend_dev_tools.key')
  const { value: keyPassword, from: passwordFrom } = await resolveKeyPassword(keyPath)
  return { conf, pubkey, endpoints, keyPath, keyPassword, passwordFrom }
}

/** 构建 + 签名 + 校验 */
async function cmdBuild(opts) {
  step(1, '前置检查')
  const ctx = await loadUpdaterContext()
  if (!existsSync(ctx.keyPath)) {
    const cands = await listKeyCandidates()
    fail(
      `找不到签名私钥：${ctx.keyPath}\n` +
        (cands.length
          ? `  ~/.tauri 下现成的私钥（用 TAURI_SIGNING_PRIVATE_KEY_PATH=<路径> 指定）：\n` +
            cands.map((c) => `    ${c}\n`).join('')
          : '') +
        `  生成新密钥：pnpm exec tauri signer generate -w "${ctx.keyPath}" -p ""\n` +
        `  生成后把公钥写进 src-tauri/tauri.conf.json 的 plugins.updater.pubkey`
    )
  }
  ok(`签名私钥：${ctx.keyPath}`)
  ok(`私钥口令来源：${ctx.passwordFrom}`)
  // 口令错、或钥匙与配置里的公钥不配对 —— 都别等编译打包完才报，先自检
  const keyCheck = await selfCheckSigningKey(ctx)
  if (!keyCheck.ok) fail(keyCheck.message)
  ok(`签名自检通过（keyid ${keyCheck.keyId}）：私钥可解开，且与配置里的公钥配对`)
  checkNotRunning()
  ensureNsisToolchain()

  // 不升版本号就构建，会沿用当前版本重发同一个版本（Release/tag 已存在 → 报错或需要 --force）。
  // 这是最容易不小心踩的一脚，所以显式提示。
  if (!opts.version && !opts.bump) {
    warn(
      `未指定 --bump / --version，将沿用当前版本 ${await currentVersion()} 构建和发布；` +
        '要发新版本请加 --bump patch（或 --version x.y.z）'
    )
  }

  if (opts.version) {
    step(2, `设置版本号 → ${opts.version}`)
    await setVersion(opts.version)
    opts.bump = null
  } else if (opts.bump) {
    const next = bumpVersion(await currentVersion(), opts.bump)
    step(2, `升级版本号 → ${next}`)
    await setVersion(next)
  }

  const version = await currentVersion()
  step(3, `构建并签名（版本 ${version}）`)
  log(C.dim('  tauri build 会先 vite build 再编译 Rust 并打包 NSIS，首次/大改后耗时较长'))
  runTauri(['build', '--bundles', 'nsis'], {
    // 私钥只传**内容**。TAURI_SIGNING_PRIVATE_KEY 与 TAURI_SIGNING_PRIVATE_KEY_PATH 在 CLI 里是
    // 互斥参数（clap conflicts_with），两个都设会直接报错退出。
    TAURI_SIGNING_PRIVATE_KEY: await readFile(ctx.keyPath, 'utf8'),
    // 口令必须显式设置（空口令也要设成空串）：不设时 CLI 会转交互式密码提示，非交互环境下会挂住
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ctx.keyPassword,
    // 见 DROP_KEY_PATH_ENV：环境里残留这个变量会和上面的「内容」型私钥互斥
    ...DROP_KEY_PATH_ENV
  })

  step(4, '校验签名与公钥是否匹配')
  const art = await findArtifact()
  if (!art) fail(`构建结束但没找到 NSIS 安装包，检查 ${BUNDLE_NSIS}`)
  const result = await verifyArtifact(art, ctx.pubkey)
  if (!result.ok) fail(`签名校验失败：${result.reason}\n  这意味着用户点「立即更新」会报签名错误，必须先修好再发布。`)
  ok(`${art.name} 签名有效（keyid ${result.keyId}）`)
  return { version, artifact: art }
}

async function verifyArtifact(art, pubkey) {
  const content = await readFile(art.file)
  const sigRaw = await readFile(art.sigFile, 'utf8')
  return verifySignature(content, sigRaw, pubkey)
}

async function cmdVerify(opts) {
  const ctx = await loadUpdaterContext()
  const files = opts.files.length ? opts.files : [(await findArtifact())?.file].filter(Boolean)
  if (!files.length) {
    fail(`没有可校验的文件：未指定参数，且 ${BUNDLE_NSIS} 下没有构建产物。先跑 build。`)
  }
  let bad = 0
  for (const f of files) {
    const abs = path.resolve(ROOT, f)
    const sigFile = abs.endsWith('.sig') ? abs : `${abs}.sig`
    if (!existsSync(sigFile)) {
      warn(`${path.basename(f)}：缺少 ${path.basename(sigFile)}`)
      bad++
      continue
    }
    const r = verifySignature(await readFile(abs), await readFile(sigFile, 'utf8'), ctx.pubkey)
    if (r.ok) ok(`${path.basename(abs)} → 签名有效（keyid ${r.keyId}）`)
    else {
      warn(`${path.basename(abs)} → ${r.reason}`)
      bad++
    }
  }
  if (bad) fail(`${bad} 个文件的签名校验未通过`)
}

/**
 * 解析 Gitee 令牌。三个来源，按顺序取第一个命中的：
 *   1. 环境变量 `GITEE_TOKEN`
 *   2. 仓库内 `.workbuddy/release.env`（该文件已在 .gitignore 里，写 `GITEE_TOKEN=xxx` 即可）
 *   3. `~/.gitee_token`（纯令牌文本）
 * 之所以支持文件：令牌是密钥，走文件可以避免它出现在命令行历史、shell 记录或聊天记录里。
 */
async function resolveToken() {
  if (process.env.GITEE_TOKEN) return { token: process.env.GITEE_TOKEN.trim(), from: '环境变量 GITEE_TOKEN' }

  const envFile = path.join(ROOT, '.workbuddy', 'release.env')
  if (existsSync(envFile)) {
    const m = (await readFile(envFile, 'utf8')).match(/^\s*GITEE_TOKEN\s*=\s*(.+)$/m)
    if (m) return { token: m[1].trim().replace(/^["']|["']$/g, ''), from: path.relative(ROOT, envFile) }
  }

  const homeFile = path.join(homedir(), '.gitee_token')
  if (existsSync(homeFile)) return { token: (await readFile(homeFile, 'utf8')).trim(), from: '~/.gitee_token' }

  return { token: '', from: '' }
}

/** 上传 + 更新清单 */
async function cmdPublish(opts) {
  const ctx = await loadUpdaterContext()
  const { token, from } = await resolveToken()
  if (!token && !opts.dryRun) {
    fail(
      '缺少 Gitee 访问令牌。任选一种方式（不要写进仓库）：\n' +
        '  1) 环境变量：export GITEE_TOKEN=xxxx（PowerShell：$env:GITEE_TOKEN="xxxx"）\n' +
        `  2) 写进 ${path.join('.workbuddy', 'release.env')}：GITEE_TOKEN=xxxx\n` +
        '  3) 写进 ~/.gitee_token（纯令牌文本）\n' +
        '  令牌在 https://gitee.com/profile/personal_access_tokens 新建，勾 projects 权限即可。'
    )
  }
  if (token) ok(`令牌来源：${from}`)

  step(1, '前置检查')
  const version = await currentVersion()
  const art = await findArtifact()
  if (!art) fail(`没有构建产物（${BUNDLE_NSIS}）。先跑 node scripts/release.mjs build`)
  if (!art.name.includes(version)) {
    fail(`产物名 ${art.name} 里没有版本号 ${version}，可能产物过期了，请重新构建`)
  }
  const vr = await verifyArtifact(art, ctx.pubkey)
  if (!vr.ok) fail(`签名校验失败：${vr.reason}`)
  ok(`产物 ${art.name}，签名有效`)
  if (!opts.skipSyncCheck) checkSync()

  const tag = `v${version}`
  const notes = await resolveNotes(opts, version)

  step(2, `发布到 Gitee（${OWNER}/${REPO} @ ${tag}）`)
  const { id: releaseId, dryRun } = await ensureRelease({
    tag,
    name: `${version} 发布`,
    body: notes,
    token,
    force: opts.force,
    prerelease: opts.prerelease,
    dryRun: opts.dryRun
  })

  let downloadUrl
  if (dryRun) {
    downloadUrl = `https://gitee.com/${OWNER}/${REPO}/releases/download/${tag}/${art.name}`
    log(`  ${C.dim('[dry-run]')} 将上传 ${art.name} → ${downloadUrl}`)
  } else {
    const asset = await uploadAsset(releaseId, art.file, token)
    downloadUrl = asset.browser_download_url
    ok(`已上传附件：${asset.name}`)
  }

  step(3, '生成 latest.json')
  const manifest = {
    version,
    notes,
    pub_date: new Date().toISOString(),
    platforms: {
      [PLATFORM_KEY]: {
        signature: (await readFile(art.sigFile, 'utf8')).trim(),
        url: downloadUrl
      }
    }
  }
  const text = JSON.stringify(manifest, null, 2) + '\n'
  await writeTextKeepingEol(MANIFEST_FILE, text)
  ok(`已写入 ${path.relative(ROOT, MANIFEST_FILE)}`)

  if (opts.noCommitManifest) {
    warn('按 --no-commit-manifest，未提交到仓库（需你手动 push 才会生效）')
  } else if (dryRun) {
    log(`  ${C.dim('[dry-run]')} 将把 latest.json 提交到 ${BRANCH}`)
  } else {
    step(4, `提交 latest.json 到 ${BRANCH}`)
    const r = await commitManifest(text, token, `chore(release): ${tag} 更新清单`)
    ok(`${r.created ? '新建' : '更新'}了仓库里的 latest.json`)
    // 本地那份 latest.json 与远端刚提交的内容一致，但它是**未提交的脏文件**——
    // 直接 pull/--rebase 会被 git 拒绝（unstaged changes）。先丢弃再拉最省事。
    warn(
      `远端 ${BRANCH} 因此多了一个提交，本地要同步：\n` +
        `    git restore latest.json && git pull --rebase ${REMOTE} ${BRANCH}\n` +
        `  另外版本号变更（tauri.conf.json / Cargo.toml / package.json）是脚本改的本地文件，记得 commit + push。`
    )
  }

  step(5, '结果')
  log(`  版本        ${version}`)
  log(`  安装包      ${downloadUrl}`)
  log(`  清单端点    ${ctx.endpoints[0] || '(未配置)'}`)
  log(`  平台条目    ${PLATFORM_KEY}`)
  if (!dryRun) {
    const raw = ctx.endpoints[0]
    if (raw) {
      log(`\n  验证清单已生效（Gitee raw 有缓存，可能需等几十秒）：`)
      log(C.dim(`  curl -s "${raw}" | head -30`))
    }
  }
}

/** 发布说明最多保留多少条提交（超出的折叠成一行）—— 它会原样显示在应用的「更新内容」里 */
const MAX_NOTE_LINES = 15

async function resolveNotes(opts, version) {
  if (opts.notesFile) return (await readFile(path.resolve(ROOT, opts.notesFile), 'utf8')).trim()
  if (opts.notes) return opts.notes

  // 从上一个 v* 标签起算；没有标签则退回最近 20 条提交
  const lastTag = git(['describe', '--tags', '--abbrev=0', '--match', 'v*'])
  const hasTag = lastTag.status === 0 && !!lastTag.stdout
  const rangeArgs = hasTag ? [`${lastTag.stdout}..HEAD`] : ['-20']
  const r = git(['log', '--no-merges', '--pretty=- %s', ...rangeArgs])

  // 保序去重：仓库经历过 rebase/合并，同一条提交可能被重复引入（实测 v1.3.0..HEAD 有 4 组重复 subject）。
  // 这些行会**原样**显示在应用的「更新内容」里，重复条目观感很差，所以在切片前先按 subject 去重。
  const seen = new Set()
  const lines = []
  let dupes = 0
  for (const raw of (r.stdout || '').split(/\r?\n/)) {
    const subject = raw.replace(/^- /, '').trim()
    if (!subject) continue
    if (seen.has(subject)) {
      dupes++
      continue
    }
    seen.add(subject)
    lines.push(`- ${subject}`)
  }
  if (dupes) warn(`发布说明已合并 ${dupes} 条重复的提交记录`)
  if (!lines.length) return `${version} 发布`

  const head = hasTag ? [`本版更新（自 ${lastTag.stdout}）`] : []
  const kept = lines.slice(0, MAX_NOTE_LINES)
  if (lines.length > MAX_NOTE_LINES) kept.push(`- …另有 ${lines.length - MAX_NOTE_LINES} 条提交`)
  return [...head, ...kept].join('\n')
}

// ==================== 入口 ====================

async function main() {
  const opts = parseArgs(process.argv.slice(2))
  log(C.bold(`\n发版流程：${opts.command}${opts.dryRun ? '（dry-run：不发写请求）' : ''}`))

  if (opts.command === 'verify') return cmdVerify(opts)
  if (opts.command === 'build') return cmdBuild(opts)
  if (opts.command === 'publish') return cmdPublish(opts)
  if (opts.command === 'all') {
    if (!opts.skipBuild) {
      const r = await cmdBuild(opts)
      if (r) ok(`构建完成：${r.artifact.name}`)
    } else {
      ok('按 --skip-build 跳过构建，复用已有产物')
    }
    // build 已经做过版本号变更与签名校验，publish 内部会再校验一次（幂等）
    return cmdPublish({ ...opts, skipBuild: true, version: null, bump: null })
  }
}

main().catch((e) => {
  log(`\n${C.red('✗ 发版中止')}\n${e.message}\n`)
  process.exit(1)
})
