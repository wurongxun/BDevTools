/*
 * 插件页脚本（外链，因为插件页 CSP 是 `script-src 'self'`，内联脚本会被直接拒绝）。
 *
 * 这里没有任何业务逻辑，也拿不到任何宿主对象：整页只有三件事
 *   1. 发 `ready` 握手，让宿主把主题与 WASM 运行时状态推过来
 *   2. 把按钮点击变成 `call` 消息 —— 宿主转发进 Extism WASM 沙箱
 *   3. 把 `{ reqId, ok, data }` 的回复渲染到结果区
 *
 * 页面与宿主之间唯一的通信方式是 `parent.postMessage`，
 * 因为宿主给 iframe 的 sandbox 只有 `allow-scripts`（不透明源）。
 */
(function () {
  'use strict'

  // 必须与宿主 `src/core/plugin/types.ts` 的 PLUGIN_BRIDGE_VERSION 一致，否则消息会被丢弃
  var BRIDGE_VERSION = 1
  // WASM 后端唯一入口函数名，对应 guest 侧 `#[plugin_fn] pub fn handle`
  var ENTRY_FUNC = 'handle'

  var badge = document.getElementById('wasm-badge')
  var out = document.getElementById('out')
  var textEl = document.getElementById('text')
  var keyEl = document.getElementById('key')
  var pathEl = document.getElementById('path')
  var urlEl = document.getElementById('url')
  var needleEl = document.getElementById('needle')
  var destEl = document.getElementById('dest')

  /** 已发出的请求：reqId → 按钮元素（用于禁用 + 复位） */
  var pending = {}
  var seq = 0
  /** 宿主推来的运行时状态；null = 还没收到 theme 消息 */
  var runtime = null

  // ---------- 渲染 ----------

  function setBadge(text, off) {
    if (!badge) return
    badge.textContent = text
    badge.className = off ? 'badge is-off' : 'badge'
  }

  function show(title, body) {
    if (!out) return
    out.textContent = body === undefined ? String(title) : title + '\n\n' + body
  }

  function showError(title, message) {
    show(title, message || '未知错误')
  }

  /** data 可能是对象（guest 的 data 字段）或字符串，统一美化 */
  function stringify(data) {
    if (data === null || data === undefined) return ''
    if (typeof data === 'string') {
      // guest 返回的 Resp 有时会被宿主作为字符串透传，尝试再解析一层
      try {
        return JSON.stringify(JSON.parse(data), null, 2)
      } catch (e) {
        return data
      }
    }
    try {
      return JSON.stringify(data, null, 2)
    } catch (e) {
      return String(data)
    }
  }

  /** 把 guest 的响应规范化成 { ok, message, data }；解析失败返回 null */
  function toBody(reply) {
    try {
      return typeof reply === 'string' ? JSON.parse(reply) : reply
    } catch (e) {
      return null
    }
  }

  /** guest 的响应体：{ ok, message, data } */
  function renderGuestReply(reply) {
    var body = toBody(reply)
    if (body === null) {
      show('WASM 返回了无法解析的内容', String(reply))
      return
    }
    if (typeof body !== 'object') {
      show('WASM 返回', stringify(body))
      return
    }
    var detail = stringify(body.data)
    if (body.ok === false) {
      showError('WASM 拒绝了这次请求', body.message + (detail ? '\n\n' + detail : ''))
      return
    }
    // 纯计算类动作把结果放在 data 里，挑出来单独展示更直观
    if (detail) {
      show(body.message || '完成', detail)
    } else {
      show(body.message || '完成')
    }
  }

  // ---------- 桥通信 ----------

  function send(payload) {
    parent.postMessage(payload, '*')
  }

  /**
   * 调一次 WASM 后端。
   * @param {string} action   guest 侧的动作分支
   * @param {HTMLElement} btn  触发按钮（调用期间禁用，防连点）
   * @param {object} [extra]   追加字段（流式动作用的 cursor / dest / offset 等）
   * @param {function} [onDone] 拿到回包时不直接渲染，交给调用方处理（流式循环用）
   */
  function invoke(action, btn, extra, onDone) {
    if (runtime === false) {
      showError('WASM 运行时未启用', '当前构建关闭了 wasm-runtime，插件后端无法执行。')
      return
    }
    var reqId = ++seq
    pending[reqId] = { btn: btn, onDone: onDone }
    if (btn) btn.disabled = true
    show('正在调用 WASM…', 'action = ' + action)

    var input = {
      action: action,
      text: textEl ? textEl.value : '',
      key: keyEl ? keyEl.value : '',
      path: pathEl ? pathEl.value : '',
      url: urlEl ? urlEl.value : '',
      needle: needleEl ? needleEl.value : '',
      dest: destEl ? destEl.value : '',
    }
    if (extra) {
      for (var k in extra) {
        if (Object.prototype.hasOwnProperty.call(extra, k)) input[k] = extra[k]
      }
    }
    send({ v: BRIDGE_VERSION, kind: 'call', reqId: reqId, func: ENTRY_FUNC, input: input })
  }

  /**
   * 流式动作：页面按游标**反复调用**，直到 guest 说 done。
   *
   * 这是长任务的唯一正确形态：单次调用有 15s / 10 亿 fuel 的上限，一次跑不完是常态；
   * 游标由 guest 在响应里回传，页面只负责推进 + 聚合进度。
   */
  function runStream(action, btn) {
    if (runtime === false) {
      showError('WASM 运行时未启用', '当前构建关闭了 wasm-runtime，插件后端无法执行。')
      return
    }
    var cursor = 0
    var rounds = 0
    var total = 0

    function step() {
      invoke(action, rounds === 0 ? btn : null, { cursor: cursor }, function (reply) {
        var body = toBody(reply)
        if (!body || body.ok === false) {
          renderGuestReply(reply)
          return
        }
        var d = body.data || {}
        rounds += 1
        if (typeof d.cursor === 'number') cursor = d.cursor
        if (typeof d.size === 'number') total = d.size
        var pct = total ? Math.round((cursor / total) * 100) : 100
        show(
          '第 ' + rounds + ' 轮 · ' + pct + '% · ' + cursor + ' / ' + total + ' 字节',
          stringify(d),
        )
        // done 之后还要再调一次？不用：guest 已经把最终游标和结果给全了
        if (d.done || rounds >= 64) {
          renderGuestReply(reply)
          return
        }
        step()
      })
    }

    step()
  }

  window.addEventListener('message', function (e) {
    var data = e.data
    if (!data || typeof data !== 'object' || data.v !== BRIDGE_VERSION) return

    // 宿主主动推送：主题 + WASM 运行时可用性
    if (data.kind === 'theme') {
      document.documentElement.setAttribute('data-theme', data.dark ? 'dark' : 'light')
      runtime = data.wasm === true
      setBadge(runtime ? 'WASM 运行时：已就绪' : 'WASM 运行时：未启用', !runtime)
      return
    }

    // 调用回复
    if (typeof data.reqId !== 'number') return
    var entry = pending[data.reqId]
    if (entry) {
      if (entry.btn) entry.btn.disabled = false
      delete pending[data.reqId]
    }
    if (data.ok) {
      // 流式循环自己处理回包（要拿 cursor / done 决定是否再调一次）
      if (entry && entry.onDone) entry.onDone(data.data)
      else renderGuestReply(data.data)
    } else {
      // 宿主已把 501 / 500 折叠成可读文案
      showError('调用失败', data.error)
    }
  })

  // ---------- 事件绑定（CSP 禁内联，只能 addEventListener）----------

  document.addEventListener('click', function (e) {
    var target = e.target
    if (!target || target.tagName !== 'BUTTON') return
    var action = target.getAttribute('data-action')
    if (action) {
      // 大文件：这两个动作由页面按游标循环调用（一次调用跑不完）
      if (action === 'scan' || action === 'copy') {
        runStream(action, target)
        return
      }
      // 就地改：单次调用即可（偏移 3 处写 3 字节，再截断到 7 字节）
      if (action === 'patch') {
        invoke('patch', target, { offset: 3, truncateTo: 7 })
        return
      }
      invoke(action, target)
      return
    }
    if (target.id === 'clear') {
      show('已清空。点击上面的按钮，结果会显示在这里。')
    }
  })

  // ---------- 握手 ----------

  setBadge('WASM 运行时：握手…', false)
  send({ v: BRIDGE_VERSION, kind: 'ready' })
})()
