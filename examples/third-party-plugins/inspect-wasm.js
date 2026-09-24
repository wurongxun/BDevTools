// 解析 WASM 二进制，确认 guest 模块的导入命名空间与导出名，
// 与宿主 plugin_wasm.rs 的 EXTISM_USER_MODULE / plugin.json 的 init 声明对得上。
const fs = require('fs')

const path = process.argv[2]
const buf = fs.readFileSync(path)
if (buf.readUInt32LE(0) !== 0x6d736100) throw new Error('不是 wasm 二进制')
console.log('wasm 版本:', buf.readUInt32LE(4))

let off = 8
function u32() {
  let r = 0, s = 0, b
  do { b = buf[off++]; r |= (b & 0x7f) << s; s += 7 } while (b & 0x80)
  return r >>> 0
}
function name() {
  const n = u32()
  const s = buf.slice(off, off + n).toString('utf8')
  off += n
  return s
}

const imports = []
const exports_ = []

while (off < buf.length) {
  const id = buf[off++]
  const size = u32()
  const end = off + size
  if (id === 2) {
    const count = u32()
    for (let i = 0; i < count; i++) {
      const mod = name()
      const nm = name()
      const kind = buf[off++]
      if (kind === 0) u32()
      else if (kind === 1) { buf[off++]; const f = u32(); if (f & 1) u32(); if (f & 2) u32() }
      else if (kind === 2) { const f = u32(); if (f & 1) u32() }
      else if (kind === 3) { buf[off++] = buf[off]; buf[off++]; u32() }
      imports.push({ mod, nm, kind: ['func', 'table', 'mem', 'global'][kind] })
    }
  } else if (id === 7) {
    const count = u32()
    for (let i = 0; i < count; i++) {
      const nm = name()
      const kind = buf[off++]
      u32()
      exports_.push({ nm, kind: ['func', 'table', 'mem', 'global'][kind] })
    }
  }
  off = end
}

const EXPECT_MODULE = 'extism:host/user'
const hostUser = imports.filter((i) => i.mod === EXPECT_MODULE)
const other = imports.filter((i) => i.mod !== EXPECT_MODULE)

console.log('\n导入函数按模块分组：')
const byMod = {}
for (const i of imports) (byMod[i.mod] = byMod[i.mod] || []).push(i.nm)
for (const [m, ns] of Object.entries(byMod)) {
  console.log(`  ${m}  (${ns.length})`)
  if (m === EXPECT_MODULE || m === 'extism:host/env') console.log('    ' + ns.sort().join(', '))
}

console.log('\n导出函数：')
console.log('  ' + exports_.filter((e) => e.kind === 'func').map((e) => e.nm).sort().join(', '))

const want = ['init', 'handle']
let bad = 0
for (const w of want) {
  const ok = exports_.some((e) => e.nm === w && e.kind === 'func')
  console.log(`\n清单声明 init=${w}  ->  ${ok ? 'OK' : '缺失!'}`)
  if (!ok) bad++
}
console.log(`\n宿主函数命名空间 ${EXPECT_MODULE}: ${hostUser.length} 个`)
if (other.length) console.log(`非 extism:host/user 的导入（WASI 等）: ${other.length} 个`)

console.log('\n结论: ' + (bad === 0 ? '导出齐备' : `缺少 ${bad} 个导出`))
