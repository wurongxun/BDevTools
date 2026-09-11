<template>
  <div class="overview">
    <!-- 元信息卡片 -->
    <div class="meta-card">
      <div class="meta-row">
        <span class="label">文件</span>
        <span class="value mono">{{ dump.name }}</span>
      </div>
      <div class="meta-row">
        <span class="label">路径</span>
        <span class="value mono path" :title="dump.path">{{ dump.path }}</span>
      </div>
      <div class="meta-row">
        <span class="label">格式</span>
        <span class="value">
          <el-tag size="small" :type="dump.supported ? 'primary' : 'warning'" effect="plain">
            {{ dump.format || '识别中…' }}
          </el-tag>
        </span>
      </div>
      <div class="meta-row">
        <span class="label">大小</span>
        <span class="value">{{ formatBytes(dump.size) }}（{{ formatInt(dump.size) }} 字节）</span>
      </div>
      <template v-if="dump.supported && dump.idSize">
        <div class="meta-row">
          <span class="label">标识符宽</span>
          <span class="value">{{ dump.idSize }} 字节（{{ dump.idSize === 8 ? '64 位 JVM' : '32 位 JVM' }}）</span>
        </div>
        <div class="meta-row">
          <span class="label">Dump 时间</span>
          <span class="value mono">{{ dump.timestampText || '未知' }}</span>
        </div>
      </template>
      <div v-if="dump.detail" class="meta-row detail">
        <span class="label">说明</span>
        <span class="value">{{ dump.detail }}</span>
      </div>
      <div v-if="dump.error" class="meta-row detail error">
        <span class="label">错误</span>
        <span class="value">{{ dump.error }}</span>
      </div>
    </div>

    <!-- 支持的格式提示 -->
    <div class="cap-card">
      <div class="cap-title">支持格式</div>
      <ul class="cap-list">
        <li><b>JAVA PROFILE 1.0.x</b> —— HotSpot / OpenJDK（jcmd GC.heap_dump、jmap、OOM 自动转储）</li>
        <li><b>Android Studio .hprof</b> —— 同族 JAVA PROFILE 变体（自动识别，个别扩展记录可能需后续适配）</li>
        <li><b>IBM PHD</b> —— 可识别（暂不支持解析，见右侧路线图）</li>
      </ul>
    </div>

    <!-- 能力路线图 -->
    <div class="cap-card">
      <div class="cap-title">能力路线图</div>
      <ul class="cap-list">
        <li>✅ <b>类</b>：按类统计对象数量与浅堆（Shallow Heap）</li>
        <li>⬜ <b>支配树</b>：独立视图（Retained Heap 已在 Leak Suspects 内使用）</li>
        <li>✅ <b>Leak Suspects</b>：支配树 Retained Heap + GC Roots 引用路径</li>
        <li>✅ <b>引用追踪</b>：Outgoing / Incoming / Path to GC Roots（类名搜索 → 实例）</li>
        <li>⬜ <b>OQL</b>：类 SQL 对象查询</li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { MatDump } from '../../types'
import { formatBytes, formatInt } from '../../api/matApi'

/** 概览页：文件元信息 + 支持格式 + 路线图 */
defineProps<{
  dump: MatDump
}>()
</script>

<style scoped lang="scss">
.overview {
  height: 100%;
  overflow: auto;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.meta-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 8px;
  padding: 12px 16px;
  background: var(--el-fill-color-light);
  border-radius: var(--radius-md);
  align-content: start;
}

.meta-row {
  display: flex;
  align-items: baseline;
  gap: 12px;
  min-height: 24px;

  &.detail .value {
    word-break: break-all;
  }

  &.error .value {
    color: var(--el-color-danger);
  }
}

.meta-row .label {
  flex-shrink: 0;
  width: 88px;
  font-size: var(--font-size-sm);
  color: var(--el-text-color-secondary);
}

.meta-row .value {
  font-size: var(--font-size-base);
  color: var(--el-text-color-primary);
  min-width: 0;
}

.mono {
  font-family: var(--font-family-mono);
}

.path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cap-card {
  padding: 12px 16px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: var(--radius-md);
}

.cap-title {
  font-size: var(--font-size-md);
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin-bottom: 8px;
}

.cap-list {
  margin: 0;
  padding-left: 20px;
  font-size: var(--font-size-base);
  line-height: 2;
  color: var(--el-text-color-secondary);
}
</style>
