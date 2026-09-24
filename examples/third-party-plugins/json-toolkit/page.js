/**
 * 插件页脚本 —— 只做渲染 + postMessage，绝不直接调用宿主接口。
 *
 * 这个 iframe 是不透明源（sandbox="allow-scripts"，没有 allow-same-origin）：
 * 访问不到 parent、拿不到任何 Tauri 对象，唯一的出口是把消息发给宿主。
 *
 * 桥协议（与宿主 PLUGIN_BRIDGE_VERSION 一致）：
 *   页面 → 宿主：{ v, kind: 'ready' }
 *                { v, kind: 'call', reqId, func, input }
 *   宿主 → 页面：{ v, kind: 'theme', dark }
 *                { v, reqId, ok: true, data } | { v, reqId, ok: false, error }
 *
 * 注意：宿主 CSP 为 script-src 'self'，内联脚本与内联事件处理器都会被拒绝。
 */
(function () {
  'use strict'

  var BRIDGE = 1
  var seq = 0
  var pending = new Map()

  var el = {
    input: document.getElementById('input'),
    output: document.getElementById('output'),
    theme: document.getElementById('theme'),
    bridge: document.getElementById('bridge'),
    format: document.getElementById('format'),
    minify: document.getElementById('minify'),
    copy: document.getElementById('copy'),
    probe: document.getElementById('probe'),
  }

  function post(msg) {
    msg.v = BRIDGE
    parent.postMessage(msg, '*')
  }

  /** 请求宿主：只有 call 一种动作，宿主再转给 Rust 侧做权限校验 */
  function hostCall(func, input) {
    return new Promise(function (resolve, reject) {
      var reqId = ++seq
      pending.set(reqId, { resolve: resolve, reject: reject })
      post({ kind: 'call', reqId: reqId, func: func, input: input })
    })
  }

  window.addEventListener('message', function (e) {
    var d = e.data
    if (!d || typeof d !== 'object' || d.v !== BRIDGE) return

    if (d.kind === 'theme') {
      applyTheme(!!d.dark)
      return
    }
    var p = pending.get(d.reqId)
    if (!p) return
    pending.delete(d.reqId)
    if (d.ok) p.resolve(d.data)
    else p.reject(new Error(d.error || '插件调用失败'))
  })

  function applyTheme(dark) {
    document.documentElement.setAttribute('data-theme', dark ? 'dark' : 'light')
    el.theme.textContent = dark ? 'dark' : 'light'
  }

  function show(text, isError) {
    el.output.textContent = text
    el.output.classList.toggle('is-error', !!isError)
  }

  function parseInput() {
    var raw = el.input.value.trim()
    if (!raw) throw new Error('请输入 JSON 内容')
    return JSON.parse(raw)
  }

  /** 递归统计结构：对象/数组/字符串/数字/布尔/null 数量与最大深度 */
  function analyze(value) {
    var stat = { object: 0, array: 0, string: 0, number: 0, boolean: 0, null: 0 }
    var maxDepth = 0

    function walk(node, depth) {
      if (depth > maxDepth) maxDepth = depth
      if (node === null) { stat.null++; return }
      if (Array.isArray(node)) {
        stat.array++
        node.forEach(function (item) { walk(item, depth + 1) })
        return
      }
      switch (typeof node) {
        case 'object':
          stat.object++
          Object.keys(node).forEach(function (k) { walk(node[k], depth + 1) })
          break
        case 'string': stat.string++; break
        case 'number': stat.number++; break
        case 'boolean': stat.boolean++; break
        default: stat.null++
      }
    }

    walk(value, 1)
    return { stat: stat, maxDepth: maxDepth }
  }

  function summary(parsed, text) {
    var info = analyze(parsed)
    return [
      '字节数：' + text.length,
      '最大深度：' + info.maxDepth,
      '对象 ' + info.stat.object + ' · 数组 ' + info.stat.array + ' · 字符串 ' + info.stat.string,
      '数字 ' + info.stat.number + ' · 布尔 ' + info.stat.boolean + ' · null ' + info.stat.null,
    ].join('\n')
  }

  el.format.addEventListener('click', function () {
    try {
      var parsed = parseInput()
      var text = JSON.stringify(parsed, null, 2)
      show(text + '\n\n———— 统计 ————\n' + summary(parsed, text))
    } catch (e) {
      show('解析失败：' + e.message, true)
    }
  })

  el.minify.addEventListener('click', function () {
    try {
      var parsed = parseInput()
      var text = JSON.stringify(parsed)
      show(text + '\n\n———— 统计 ————\n' + summary(parsed, text))
    } catch (e) {
      show('解析失败：' + e.message, true)
    }
  })

  el.copy.addEventListener('click', function () {
    var text = el.output.textContent || ''
    if (!text || text === '等待输入…') return
    // 沙箱内没有剪贴板权限，退化为选中文本让用户手动复制
    var range = document.createRange()
    range.selectNodeContents(el.output)
    var sel = window.getSelection()
    sel.removeAllRanges()
    sel.addRange(range)
    show('已选中结果文本，按 Ctrl/Cmd+C 复制。\n\n———— 原结果 ————\n' + text)
  })

  el.probe.addEventListener('click', function () {
    el.probe.disabled = true
    show('正在调用宿主 plugin_call…')
    hostCall('handle', { action: 'ping' })
      .then(function (data) {
        show('宿主返回：' + (typeof data === 'string' ? data : JSON.stringify(data)))
      })
      .catch(function (err) {
        // v1 未启用 WASM 运行时，这里会收到宿主的结构化错误——链路本身是通的
        show('宿主拒绝了本次调用（这是预期的）：\n' + err.message, true)
      })
      .then(function () { el.probe.disabled = false })
  })

  el.bridge.textContent = String(BRIDGE)
  // 握手：宿主收到 ready 后才会回灌主题
  post({ kind: 'ready' })
})()
