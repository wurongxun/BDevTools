<template>
  <Modal title="连接配置" @close="$emit('close')">
    <div class="field">
      <label>名称</label>
      <input class="input" v-model="form.name" placeholder="我的 Redis" />
    </div>
    <div class="field-row">
      <div class="field">
        <label>主机</label>
        <input class="input" v-model="form.host" placeholder="127.0.0.1" />
      </div>
      <div class="field" style="max-width: 110px">
        <label>端口</label>
        <input class="input" v-model.number="form.port" type="number" />
      </div>
    </div>
    <div class="field">
      <label>密码（可选）</label>
      <input class="input" v-model="form.password" type="password" placeholder="留空表示无密码" />
    </div>
    <div class="field-row">
      <div class="field" style="max-width: 110px">
        <label>默认库</label>
        <input class="input" v-model.number="form.db" type="number" min="0" max="15" />
      </div>
      <div class="field">
        <label>连接方式</label>
        <select class="select" v-model="form.mode">
          <option value="mock">本地 Mock（内置引擎，无需真实服务）</option>
          <option value="tcp">直连 TCP（真实后端）</option>
          <option value="proxy">伴随代理（预留）</option>
        </select>
      </div>
    </div>
    <div v-if="form.mode === 'mock'" class="field">
      <label class="chk"><input type="checkbox" v-model="form.seed" /> 注入演示数据</label>
    </div>

    <div class="field reserved">
      <label class="chk"><input type="checkbox" v-model="form.tls" /> 启用 SSL/TLS <span class="tag-reserved">预留</span></label>
    </div>
    <div class="field reserved">
      <label>Unix Socket 路径<span class="tag-reserved">预留</span></label>
      <input class="input" v-model="form.unixSocket" placeholder="/tmp/redis.sock" disabled />
    </div>
    <div class="field reserved">
      <label>SSH 隧道<span class="tag-reserved">预留</span></label>
      <div class="field-row">
        <input class="input" v-model="ssh.host" placeholder="跳板机 host" disabled />
        <input class="input" v-model="ssh.user" placeholder="user" disabled />
      </div>
    </div>

    <template #footer>
      <button class="btn" type="button" @click="$emit('close')">取消</button>
      <button class="btn primary" type="button" @click="save">保存</button>
    </template>
  </Modal>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import Modal from './Modal.vue'
import { addConnection, updateConnection, connectById, newConnId, toast } from '../store'
import type { ConnectionConfig } from '../types'

const props = defineProps<{ conn?: ConnectionConfig }>()
const emit = defineEmits<{ (e: 'close'): void }>()

const base: ConnectionConfig = {
  id: '',
  name: '',
  host: '127.0.0.1',
  port: 6379,
  password: '',
  db: 0,
  mode: 'mock',
  seed: true
}
const form = reactive<ConnectionConfig>({ ...base, ...(props.conn || {}) })
const ssh = reactive({ host: '', user: '' })

async function save() {
  if (!form.name.trim()) {
    toast('请填写名称', 'err')
    return
  }
  const isEdit = !!props.conn
  if (!isEdit) form.id = newConnId()
  if (isEdit) updateConnection({ ...form })
  else addConnection({ ...form })
  await connectById(form.id)
  toast('已保存并连接', 'ok')
  emit('close')
}
</script>
