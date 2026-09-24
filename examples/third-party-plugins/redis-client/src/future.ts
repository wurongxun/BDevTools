// 后期阶段（P1~P3）的预留接口与路线图。
// P1 已在 client.ts 中实现大部分；这里保留「路线图」与「尚未实现」的接缝说明，方便检索与接续。

export const ROADMAP = {
  P0: {
    status: 'done',
    scope: '连接管理 + Key 浏览 + 6 种基础类型读写 + 命令行',
    notes: '纯前端 + 内存引擎（mock 模式完整演示）；tcp 模式已接真实后端（HostRedisTransport → 宿主 redis_command → src-tauri/src/command/redis_backend 的 RESP 客户端）。'
  },
  P1: {
    status: 'done',
    scope: '值编辑器增强 + 批量操作 + TTL 管理 + 基本监控',
    implemented: [
      '值多格式视图（Text/JSON/XML/YAML/Hex）+ JSON 树视图 + 大值分页',
      '值对比（文本行 diff / JSON 结构 diff）',
      '批量删除 / 设过期 / 取消过期 / 导出（JSON·CSV·Redis 脚本）',
      'Key 重命名 / 复制（同实例）/ 导入',
      'TTL 可视化（倒计时 + 进度条 + 即将过期标记）',
      '基本监控（概览 / INFO 分区 / 慢查询 / 客户端列表）'
    ],
    seams: {
      '变更监听': 'client.watchKeys — 基于 Keyspace Notifications，需真实连接（P2）',
      'SSH/SSL/UnixSocket 连接': 'connection 配置里的 ssh/tls/unixSocket 字段（UI 已留位，待真实传输启用）'
    }
  },
  P2: {
    status: 'done',
    scope: '内存分析 + 慢查询 + 集群支持 + 操作审计',
    implemented: [
      '内存分析：大 Key 排名 / 按前缀内存占比 / 按类型分布饼图（MemoryAnalysisPanel + engine.analyzeMemory + client.memoryAnalyze）',
      '慢查询分析：统计卡片(数/最大/平均/P95) + 命令热点 + 归一化明细（SlowQueryPanel + engine.analyzeSlow + client.slowQueryAnalysis）',
      '集群/复制：主从复制信息、6 节点集群拓扑+槽位分布条、Sentinel 主节点（ClusterPanel + engine.clusterTopology/replicationInfo/sentinelMasters）',
      '操作审计：每次命令行/批量/导入导出/键变更写入审计日志，敏感参数脱敏，可筛选/导出/清空（store.recordAudit + AuditPanel）'
    ],
    seams: {
      '真实传输接入': '已落地：tcp 模式经 WasmRedisTransport → bridge.call(\'handle\', {action}) 打给插件自带的 plugin.wasm（源码 ../_guest/redis-client，Rust + extism-pdk）。后端自己实现 RESP 编解码，字节收发走宿主 net: 授权下的原始 socket（host_tcp_connect/send/recv/close），连接按「键」持久在宿主侧；P2 分析动作 analyze_memory/slowlog/cluster/replication/sentinel 用 SCAN+MEMORY USAGE/SLOWLOG GET/CLUSTER INFO·NODES/INFO replication/SENTINEL MASTERS 聚合',
      '审计持久化': '当前走 MemoryPersistence（不透明源无 localStorage）；KvPersistence 预留，接入后审计跨会话保留'
    }
  },
  P3: {
    status: 'reserved',
    scope: '数据迁移 + Lua 脚本 + Pub/Sub + 插件系统',
    seams: {
      'Lua 脚本': 'client.eval(script, keys, args) / evalsha(sha) — 脚本管理',
      'Pub/Sub': 'client.publish(channel, msg) / subscribe(channel) — 需常驻通道',
      '数据迁移': 'client.migrateInstance(src, dst, opts) — SCAN+DUMP 增量同步 / RDB 导入'
    }
  }
}
