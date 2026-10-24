#!/usr/bin/env bash
#
# 第三方插件示例的完整自检 —— 一条命令跑完「编译 → 打包 → ABI → 端到端」。
#
# 用途：
#   * 本地改完示例后跑一遍，确认没把能跑的东西改坏
#   * 直接作为 CI 的一步（见 docs/plugin-dev/checklist.md）
#
# 用法：
#   bash examples/third-party-plugins/verify.sh
#
# 退出码非 0 即失败。

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"

# Windows 上的 node 不认 Git Bash 的 POSIX 路径：把 /d/xxx 直接喂给它会被解析成 D:\d\xxx，
# 报 ENOENT / MODULE_NOT_FOUND。凡是要传给 node 的路径都先过这里转成 Windows 形式。
NODE_BIN="${NODE_BIN:-node}"
HERE_WIN="$HERE"
if command -v cygpath >/dev/null 2>&1; then
  HERE_WIN="$(cygpath -m "$HERE")"
fi

say() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
ok()  { printf '    \033[32m✓\033[0m %s\n' "$*"; }

# ------------------------------------------------- 0. 链接器临时目录兜底（Windows）
# 为什么需要：MSVC 的 link.exe 必须有一个可写的临时目录。若进程环境里既没有 TMP/TEMP
# 也没有 USERPROFILE，Windows 的 GetTempPath 会兜底到「Windows 目录」，普通用户不可写，
# 于是链接第 5 步（乃至任何 cargo build/test）会报：
#   LNK1104: 无法打开文件 "C:\Windows\lnk{GUID}.tmp"
# 触发场景：某些终端/shell 垫片或 CI 包装层会把 TMP/TEMP 从「子进程环境」里抹掉
# （表现为外层 shell 里 `echo $TMP` 有值，但脚本内 `$TMP` 为空）。
# 这里只在这种情况下补一个工作区内、确定可写的目录；TMP 本来正常时不做任何改动。
LINK_TMP="$ROOT/.tmp-verify-link"
if [ -z "${TMP:-}" ] || [ ! -d "${TMP:-/nonexistent}" ]; then
  mkdir -p "$LINK_TMP"
  if command -v cygpath >/dev/null 2>&1; then
    TMP="$(cygpath -m "$LINK_TMP")"
  else
    TMP="$LINK_TMP"
  fi
  export TMP
  export TEMP="$TMP"
  ok "TMP 缺失或不可用，已为链接器指定 TMP=$TMP"
fi

# ---------------------------------------------------------------- 1. 静态检查
say "1/5 检查示例包结构"

PKGS="json-toolkit text-toolkit webgl-demo redis-client"

for pkg in $PKGS; do
  test -f "$HERE/$pkg/plugin.json" || { echo "缺少 $pkg/plugin.json"; exit 1; }
  test -f "$HERE/$pkg/page.html"   || { echo "缺少 $pkg/page.html"; exit 1; }
  # 插件页 CSP 是 script-src 'self'：内联脚本与内联事件属性会被**静默**拦掉（点了没反应）
  if grep -Eq '<script[^>]*>[^<]' "$HERE/$pkg/page.html" \
     || grep -Eq 'on(click|load|change|input|submit)=' "$HERE/$pkg/page.html"; then
    echo "$pkg/page.html 里有内联脚本或内联事件属性 —— 必须外链脚本 + addEventListener"
    exit 1
  fi
  ok "$pkg 清单与入口存在（无内联脚本）"
done

# pageCapabilities 必须在宿主白名单内（不在的话宿主会拒装，这里提前拦住）
# 白名单与 plugin_host::PAGE_CAPABILITIES 手工保持一致
"$NODE_BIN" -e '
const fs = require("fs"), path = require("path")
const HERE = process.argv[1]
const PKGS = ["json-toolkit", "text-toolkit", "webgl-demo", "redis-client"]
const WL = ["wasm", "assets", "media", "worker", "isolate"]
// 清单键名白名单：与 plugin_host::MANIFEST_KEYS 手工保持一致（改了那边要同步这里）
const KEYS = ["id", "name", "version", "description", "icon", "entry", "wasm", "init",
              "permissions", "pageCapabilities", "page_capabilities", "standaloneWindow"]
const bad = [], typeBad = [], keyBad = []
const manifests = {}
for (const pkg of PKGS) {
  const m = JSON.parse(fs.readFileSync(path.join(HERE, pkg, "plugin.json"), "utf8"))
  manifests[pkg] = m
  for (const c of m.pageCapabilities || []) if (!WL.includes(c)) bad.push(pkg + " -> " + c)
  // 写字符串 / 数字宿主会直接解析失败，这里给出人话
  if ("standaloneWindow" in m && typeof m.standaloneWindow !== "boolean") {
    typeBad.push(pkg + " -> " + JSON.stringify(m.standaloneWindow))
  }
  // 键名大小写写错 = 宿主拒装，提前拦
  for (const k of Object.keys(m)) {
    if (!KEYS.includes(k) && !/^[x_]/.test(k)) keyBad.push(pkg + " -> " + k)
  }
}
if (bad.length) { console.error("pageCapabilities 不在白名单内: " + bad.join(", ")); process.exit(1) }
if (typeBad.length) { console.error("standaloneWindow 必须是布尔值: " + typeBad.join(", ")); process.exit(1) }
if (keyBad.length) { console.error("清单含未识别/大小写错误的键名: " + keyBad.join(", ")); process.exit(1) }
// 示范包必须真的声明了独立窗口，否则示例没有示范作用
if (manifests["webgl-demo"].standaloneWindow !== true) {
  console.error("webgl-demo 应声明 standaloneWindow: true"); process.exit(1)
}
// 反例：带 WASM 后端（要调宿主 plugin_call）的包不能声明，否则会失去宿主桥
if (manifests["text-toolkit"].standaloneWindow) {
  console.error("text-toolkit 依赖宿主桥，不应声明 standaloneWindow"); process.exit(1)
}
// 示范包：自带 WASM 后端 + 网络访问授权（真实 Redis 连接全靠它），两者都不能少
const rc = manifests["redis-client"]
if (rc.wasm !== "plugin.wasm" || rc.init !== "init") {
  console.error("redis-client 应声明 wasm: plugin.wasm 与 init: init"); process.exit(1)
}
if (!(rc.permissions || []).some(p => p.startsWith("net:"))) {
  console.error("redis-client 应声明 net: 权限（真实 Redis 连接靠它）"); process.exit(1)
}
console.log("pageCapabilities / standaloneWindow / 键名 全部合规")
' "$HERE_WIN" || exit 1
ok "示例包结构检查通过"

# 包目录里绝不能出现构建产物目录（会被整个打进插件包）
PKG_DIRS=()
for pkg in $PKGS; do PKG_DIRS+=("$HERE/$pkg"); done
if find "${PKG_DIRS[@]}" -maxdepth 1 \( -name target -o -name node_modules \) | grep -q .; then
  echo "插件包目录里出现了 target/ 或 node_modules/ —— 应放在 _guest/ 下"
  exit 1
fi
ok "包目录里没有构建产物"

# ------------------------------------------------------------ 2. 编译 guest
say "2/5 编译示例的 WASM 后端"

if ! command -v cargo >/dev/null 2>&1; then
  echo "未找到 cargo —— 跳过 WASM 编译与端到端测试（仅做了静态检查）"
  exit 0
fi

if ! rustup target list --installed 2>/dev/null | grep -q '^wasm32-unknown-unknown$'; then
  echo "缺少 wasm32-unknown-unknown 目标，请先执行：rustup target add wasm32-unknown-unknown"
  exit 1
fi

# 自带 WASM 后端的示例包：<包名>:<guest crate 目录名（在 _guest/ 下）>:<crate 产物名>
GUESTS="text-toolkit:text-toolkit:text_toolkit_guest redis-client:redis-client:redis_client_guest"

for g in $GUESTS; do
  pkg="${g%%:*}"
  rest="${g#*:}"
  gdir="${rest%%:*}"
  crate="${rest#*:}"
  GUEST="$HERE/_guest/$gdir"
  WASM="$GUEST/target/wasm32-unknown-unknown/release/$crate.wasm"

  say "$pkg: 编译 guest（_guest/$gdir）"
  test -f "$GUEST/Cargo.toml" || { echo "缺少 guest crate: $GUEST"; exit 1; }
  (cd "$GUEST" && cargo build --release --target wasm32-unknown-unknown)
  test -f "$WASM" || { echo "构建产物不存在: $WASM"; exit 1; }
  ok "编译成功（$(du -h "$WASM" | cut -f1)）"

  # --------------------------------------------------------- 3. 产物同步进包
  say "$pkg: 同步产物并校验清单声明一致"
  cp "$WASM" "$HERE/$pkg/plugin.wasm"
  ok "plugin.wasm 已更新"

  # 清单里声明的 wasm 文件名必须真的在包里
  declared="$(sed -n 's/.*"wasm"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$HERE/$pkg/plugin.json")"
  test -n "$declared" || { echo "$pkg/plugin.json 未声明 wasm"; exit 1; }
  test -f "$HERE/$pkg/$declared" || { echo "清单声明的 $declared 不在包里"; exit 1; }
  ok "清单声明 $declared 且文件存在"

  # ---------------------------------------------------------------- 4. ABI 检查
  say "$pkg: 校验 WASM 的导入导出（宿主函数命名空间 + 导出名）"
  if ! command -v "$NODE_BIN" >/dev/null 2>&1; then
    echo "未找到 node —— 跳过 ABI 检查"
  else
    OUT="$("$NODE_BIN" "$HERE_WIN/inspect-wasm.js" "$HERE_WIN/$pkg/plugin.wasm")"
    echo "$OUT" | sed 's/^/    /'
    echo "$OUT" | grep -q 'handle, init' || {
      echo "$pkg 导出名不符：期望看到 handle 与 init"
      exit 1
    }
    echo "$OUT" | grep -q 'extism:host/user' || {
      echo "$pkg 没有 extism:host/user 导入：可能不是 extism-pdk 构建的模块"
      exit 1
    }
    ok "$pkg 导出 handle/init 齐备，宿主函数命名空间正确"
  fi
done

# ------------------------------------------------------------ 5. 端到端测试
say "5/5 跑宿主的端到端测试（真的把示例 WASM 加载进 Extism）"

(cd "$ROOT/src-tauri" && cargo test --lib plugin_wasm)
ok "端到端测试通过"

say "全部通过"
