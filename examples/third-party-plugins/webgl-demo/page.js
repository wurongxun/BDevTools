/* WebGL 自检示例：零依赖渲染 + 页面能力自检 */
'use strict'

// ==================== 页面能力自检 ====================
// 最小合法 WASM 模块（只有 magic + version，无 section）——用它探测「能不能编译 WASM」
const EMPTY_WASM = new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])

// 失败文案的统一后半句：这类「全红」最常见的根因**不是** CSP，而是宿主压根没读到清单声明
// （键名/大小写写错时 serde 会静默忽略）。别让排查从 CSP 开始。
const HINT_MANIFEST = '；若五项全红，先看「3D 与游戏」列表里有没有本插件——没有就是清单没读到'

async function probeWasm() {
  try {
    await WebAssembly.compile(EMPTY_WASM)
    return { ok: true, note: 'WASM 编译通过（可跑 Unity WebGL / Godot Web）' }
  } catch (e) {
    return { ok: false, note: "被阻止：script-src 缺 'wasm-unsafe-eval'" + HINT_MANIFEST }
  }
}

async function probeAssets() {
  try {
    const res = await fetch('assets/probe.json')
    if (!res.ok) return { ok: false, note: 'HTTP ' + res.status + '（扩展名或路径没放行）' }
    const data = await res.json()
    return { ok: true, note: 'fetch 包内资源成功：' + JSON.stringify(data) }
  } catch (e) {
    return { ok: false, note: "被阻止：connect-src 为 'none'" + HINT_MANIFEST }
  }
}

async function probeMedia() {
  return new Promise(function (resolve) {
    var settled = false
    var done = function (ok, note) {
      if (settled) return
      settled = true
      resolve({ ok: ok, note: note })
    }
    var el = document.createElement('audio')
    el.addEventListener('canplaythrough', function () { done(true, 'media-src 放行 blob: 音频') }, { once: true })
    el.addEventListener('error', function () { done(false, '被阻止：无 media-src' + HINT_MANIFEST) }, { once: true })
    try {
      el.src = URL.createObjectURL(buildWav())
    } catch (e) {
      done(false, '被阻止：无 media-src' + HINT_MANIFEST)
      return
    }
    el.load()
    setTimeout(function () { done(false, '超时（多半是被 CSP 静默阻止）' + HINT_MANIFEST) }, 2000)
  })
}

async function probeWorker() {
  return new Promise(function (resolve) {
    var settled = false
    var done = function (ok, note) {
      if (settled) return
      settled = true
      resolve({ ok: ok, note: note })
    }
    try {
      var url = URL.createObjectURL(new Blob(['self.postMessage(1)'], { type: 'text/javascript' }))
      var w = new Worker(url)
      w.onmessage = function () { w.terminate(); done(true, 'Blob Worker 可创建（引擎多线程的前提之一）') }
      w.onerror = function () { done(false, '被阻止：无 worker-src' + HINT_MANIFEST) }
      setTimeout(function () { done(false, '超时（多半是被 CSP 静默阻止）' + HINT_MANIFEST) }, 2000)
    } catch (e) {
      done(false, '被阻止：无 worker-src' + HINT_MANIFEST)
    }
  })
}

async function probeIsolate() {
  var sab = typeof SharedArrayBuffer !== 'undefined'
  var iso = window.crossOriginIsolated === true
  if (iso && sab) return { ok: true, note: '已跨源隔离，SharedArrayBuffer 可用（多线程 WASM 可行）' }
  return {
    ok: false,
    note: 'crossOriginIsolated=' + iso + ' / SharedArrayBuffer=' + sab + '（需独立窗口）'
  }
}

/** 生成一段 0.2 秒的 8kHz 单声道 WAV：用 blob: 探 media-src 最省事，不必内联 base64 */
function buildWav() {
  var rate = 8000
  var n = Math.floor(rate * 0.2)
  var buf = new ArrayBuffer(44 + n)
  var view = new DataView(buf)
  var str = function (off, s) {
    for (var i = 0; i < s.length; i++) view.setUint8(off + i, s.charCodeAt(i))
  }
  str(0, 'RIFF'); view.setUint32(4, 36 + n, true); str(8, 'WAVE')
  str(12, 'fmt '); view.setUint32(16, 16, true); view.setUint16(20, 1, true)
  view.setUint16(22, 1, true); view.setUint32(24, rate, true); view.setUint32(28, rate, true)
  view.setUint16(32, 1, true); view.setUint16(34, 8, true)
  str(36, 'data'); view.setUint32(40, n, true)
  for (var i = 0; i < n; i++) view.setUint8(44 + i, 128 + Math.round(60 * Math.sin(i / 12)))
  return new Blob([buf], { type: 'audio/wav' })
}

var CAP_ROWS = [
  { cap: 'wasm', run: probeWasm },
  { cap: 'assets', run: probeAssets },
  { cap: 'media', run: probeMedia },
  { cap: 'worker', run: probeWorker },
  { cap: 'isolate', run: probeIsolate }
]

function renderProbe(cap, result) {
  var li = document.createElement('li')
  li.className = 'probe-row ' + (result.ok ? 'is-ok' : 'is-bad')
  var dot = document.createElement('span')
  dot.className = 'probe-dot'
  var name = document.createElement('span')
  name.className = 'probe-cap'
  name.textContent = cap
  var note = document.createElement('span')
  note.className = 'probe-note'
  note.textContent = result.note
  li.appendChild(dot); li.appendChild(name); li.appendChild(note)
  document.getElementById('probe').appendChild(li)
}

async function runProbes() {
  for (var i = 0; i < CAP_ROWS.length; i++) {
    var row = CAP_ROWS[i]
    var li = document.createElement('li')
    li.className = 'probe-row is-pending'
    li.textContent = row.cap + '：检测中…'
    document.getElementById('probe').appendChild(li)
    var result
    try {
      result = await row.run()
    } catch (e) {
      result = { ok: false, note: '探测异常：' + (e && e.message ? e.message : String(e)) }
    }
    li.remove()
    renderProbe(row.cap, result)
  }
}

// ==================== 旋转立方体（裸 WebGL，无任何依赖） ====================
var VERT = [
  'attribute vec3 aPos;',
  'attribute vec3 aCol;',
  'uniform mat4 uMvp;',
  'varying vec3 vCol;',
  'void main() {',
  '  vCol = aCol;',
  '  gl_Position = uMvp * vec4(aPos, 1.0);',
  '}'
].join('\n')

var FRAG = [
  'precision mediump float;',
  'varying vec3 vCol;',
  'void main() { gl_FragColor = vec4(vCol, 1.0); }'
].join('\n')

// 每面 2 个三角形，逐顶点上色（面法线方向点的颜色）
var F = [
  { n: [0, 0, 1], c: [0.30, 0.62, 0.94] },
  { n: [0, 0, -1], c: [0.94, 0.42, 0.36] },
  { n: [1, 0, 0], c: [0.36, 0.78, 0.55] },
  { n: [-1, 0, 0], c: [0.95, 0.72, 0.30] },
  { n: [0, 1, 0], c: [0.68, 0.52, 0.92] },
  { n: [0, -1, 0], c: [0.55, 0.58, 0.62] }
]

function faceVerts(n) {
  // 由法线构造该面的 4 个角（顺序无所谓，索引按扇形切两三角）
  var ax = Math.abs(n[0]) === 1 ? [0, 1, 0] : [1, 0, 0]
  var u = cross(n, ax)
  var v = cross(n, u)
  var out = []
  for (var i = 0; i < 4; i++) {
    var su = i === 1 || i === 2 ? 1 : -1
    var sv = i >= 2 ? 1 : -1
    out.push([n[0] + u[0] * su + v[0] * sv, n[1] + u[1] * su + v[1] * sv, n[2] + u[2] * su + v[2] * sv])
  }
  return [out[0], out[1], out[2], out[0], out[2], out[3]]
}

function cross(a, b) {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

function norm(a) {
  var l = Math.sqrt(a[0] * a[0] + a[1] * a[1] + a[2] * a[2]) || 1
  return [a[0] / l, a[1] / l, a[2] / l]
}

function buildGeometry() {
  var data = []
  F.forEach(function (face) {
    var n = norm(face.n)
    faceVerts(n).forEach(function (p) {
      data.push(p[0], p[1], p[2], face.c[0], face.c[1], face.c[2])
    })
  })
  return new Float32Array(data)
}

function perspective(fovy, aspect, near, far) {
  var f = 1 / Math.tan(fovy / 2)
  var nf = 1 / (near - far)
  return [f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, (far + near) * nf, -1, 0, 0, 2 * far * near * nf, 0]
}

function multiply(a, b) {
  var out = new Array(16)
  for (var i = 0; i < 4; i++) {
    for (var j = 0; j < 4; j++) {
      var s = 0
      for (var k = 0; k < 4; k++) s += a[k * 4 + j] * b[i * 4 + k]
      out[i * 4 + j] = s
    }
  }
  return out
}

function rotX(t) {
  var c = Math.cos(t), s = Math.sin(t)
  return [1, 0, 0, 0, 0, c, s, 0, 0, -s, c, 0, 0, 0, 0, 1]
}

function rotY(t) {
  var c = Math.cos(t), s = Math.sin(t)
  return [c, 0, -s, 0, 0, 1, 0, 0, s, 0, c, 0, 0, 0, 0, 1]
}

function translateZ(z) {
  return [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, z, 1]
}

function compile(gl, type, src) {
  var sh = gl.createShader(type)
  gl.shaderSource(sh, src)
  gl.compileShader(sh)
  if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
    throw new Error('shader: ' + gl.getShaderInfoLog(sh))
  }
  return sh
}

function initGL() {
  var canvas = document.getElementById('gl')
  var gl = canvas.getContext('webgl', { antialias: true }) || canvas.getContext('experimental-webgl')
  if (!gl) {
    document.getElementById('gl-badge').textContent = 'WebGL 不可用'
    document.getElementById('gl-note').textContent =
      '当前 webview 没有可用的 WebGL 上下文。多数情况是显卡驱动 / 远程桌面 / 软件渲染限制，' +
      '与应用插件体系无关；纯 2D 或 WebGPU 方案可另行评估。'
    return
  }
  document.getElementById('gl-badge').textContent = 'WebGL 已就绪'

  var prog = gl.createProgram()
  gl.attachShader(prog, compile(gl, gl.VERTEX_SHADER, VERT))
  gl.attachShader(prog, compile(gl, gl.FRAGMENT_SHADER, FRAG))
  gl.linkProgram(prog)
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    throw new Error('link: ' + gl.getProgramInfoLog(prog))
  }
  gl.useProgram(prog)

  var buf = gl.createBuffer()
  gl.bindBuffer(gl.ARRAY_BUFFER, buf)
  gl.bufferData(gl.ARRAY_BUFFER, buildGeometry(), gl.STATIC_DRAW)

  var aPos = gl.getAttribLocation(prog, 'aPos')
  var aCol = gl.getAttribLocation(prog, 'aCol')
  gl.enableVertexAttribArray(aPos)
  gl.vertexAttribPointer(aPos, 3, gl.FLOAT, false, 24, 0)
  gl.enableVertexAttribArray(aCol)
  gl.vertexAttribPointer(aCol, 3, gl.FLOAT, false, 24, 12)

  var uMvp = gl.getUniformLocation(prog, 'uMvp')
  gl.enable(gl.DEPTH_TEST)

  var t0 = performance.now()
  function frame(now) {
    var t = (now - t0) / 1000
    gl.viewport(0, 0, canvas.width, canvas.height)
    gl.clearColor(0.09, 0.10, 0.12, 1)
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT)
    var mvp = multiply(perspective(50 * Math.PI / 180, canvas.width / canvas.height, 0.1, 100),
      multiply(translateZ(-4.2), multiply(rotY(t * 0.8), rotX(t * 0.45))))
    gl.uniformMatrix4fv(uMvp, false, new Float32Array(mvp))
    gl.drawArrays(gl.TRIANGLES, 0, 36)
    requestAnimationFrame(frame)
  }
  requestAnimationFrame(frame)
}

try {
  initGL()
} catch (e) {
  document.getElementById('gl-badge').textContent = 'WebGL 初始化失败'
  document.getElementById('gl-note').textContent = '渲染初始化异常：' + (e && e.message ? e.message : String(e))
}

runProbes()
