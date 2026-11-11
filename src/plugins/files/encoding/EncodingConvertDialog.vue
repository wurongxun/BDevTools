<template>
  <GlobalDialog v-model="visible" title="转换文件编码 / 换行符" width="540px">
    <div class="conv-body">
      <div class="conv-file" :title="targetPath">{{ fileName || '未选择文件' }}</div>

      <p class="conv-warn">
        这会<b>重写磁盘上的文件字节</b>，不是只换一种显示方式。转换前会自动生成
        <code>.bdt-convert-*.bak</code> 备份，可在搜索面板的「撤销历史」里恢复。
      </p>

      <div v-if="dirtyBlocked" class="conv-block">
        当前文档有未保存的更改。转换以磁盘内容为准，会覆盖这些编辑 —— 请先保存（Ctrl+S）
        或撤销修改，再执行转换。
      </div>

      <div class="conv-current">
        当前：编码 <b>{{ current.encoding || '?' }}</b> · 换行
        <b>{{ current.eol || '?' }}</b> · BOM <b>{{ current.bom || 'none' }}</b>
      </div>

      <div class="conv-row">
        <span class="conv-label">目标编码</span>
        <el-select v-model="form.encoding" size="small" class="conv-select" @change="onEncodingPicked">
          <el-option v-for="enc in CONVERT_ENCODINGS" :key="enc" :label="enc" :value="enc" />
        </el-select>
      </div>
      <div class="conv-row">
        <span class="conv-label">目标换行符</span>
        <el-select v-model="form.eol" size="small" class="conv-select">
          <el-option v-for="eol in CONVERT_EOLS" :key="eol" :label="eol" :value="eol" />
        </el-select>
      </div>
      <div class="conv-row">
        <span class="conv-label">BOM</span>
        <el-select v-model="form.bom" size="small" class="conv-select" :disabled="utf16Target">
          <el-option v-if="utf16Target" :label="`${utf16Bom}（强制）`" :value="utf16Bom" />
          <el-option label="无" value="none" />
          <el-option label="UTF-8 BOM" value="utf8" />
        </el-select>
      </div>
      <p class="conv-tip">
        UTF-16LE / UTF-16BE 会强制写入对应 BOM（否则下次打开无法可靠识别编码）；
        其它编码不允许带 BOM。目标编码无法表示现有字符（如中文转 windows-1252）时会被拒绝，原文件不变。
      </p>
    </div>
    <template #footer>
      <app-button size="small" @click="visible = false">取消</app-button>
      <app-button
        size="small"
        type="primary"
        :loading="converting"
        :disabled="!canSubmit"
        @click="confirmConvert"
      >确认转换</app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import GlobalDialog from '@/components/GlobalDialog.vue'
import NotificationUtil from '@/utils/NotificationUtil'
import { useDocumentManagerStore } from '@/store/modules/documents'
import { pushUndoBatch, refreshOpenDocuments } from '../search/useUndoHistory'
import { convertEncodingEol } from './fileConvertApi'
import {
  closeConvertDialog,
  CONVERT_ENCODINGS,
  CONVERT_EOLS,
  convertDialogVisible,
  convertTargetPath,
  hasConvertChange
} from './useEncodingConvert'

const docStore = useDocumentManagerStore()
const converting = ref(false)

const visible = computed({
  get: () => convertDialogVisible.value,
  set: (v: boolean) => {
    if (v) convertDialogVisible.value = true
    else closeConvertDialog()
  }
})

const targetPath = computed(
  () => convertTargetPath.value || docStore.activeDocumentPath || ''
)
const doc = computed(() => docStore.documents.get(targetPath.value))
const fileName = computed(() => {
  const p = targetPath.value
  return p ? p.split(/[\\/]/).pop() : ''
})

const current = computed(() => ({
  encoding: doc.value?.encoding,
  eol: doc.value?.eol,
  bom: doc.value?.bom
}))

/** 大文件行补丁也算未保存内容（content === null），不能漏判 */
const dirtyBlocked = computed(
  () => !!doc.value?.isDirty || (doc.value?.largeEdits?.length ?? 0) > 0
)

const form = reactive({
  encoding: 'UTF-8',
  eol: 'LF',
  bom: 'none'
})

const utf16Target = computed(() => /^UTF-16(L|A|BE|LE)/i.test(form.encoding))
const utf16Bom = computed(() =>
  form.encoding.toUpperCase() === 'UTF-16BE' ? 'utf16be' : 'utf16le'
)

/** 打开对话框时用当前文件元信息做初值，避免用户一进来就看到「改成 UTF-8/LF」的误操作默认值。 */
watch(visible, (v) => {
  if (!v) return
  const d = doc.value
  // encoding_rs 的 name() 大小写不统一（GBK 大写、gb18030 小写），必须忽略大小写匹配，
  // 否则「当前是 GB18030」会被误判成不在列表里而回落成 UTF-8，一打开就是危险默认值
  const currentEnc = d?.encoding || ''
  const matched = (CONVERT_ENCODINGS as readonly string[]).find(
    (e) => e.toLowerCase() === currentEnc.toLowerCase()
  )
  form.encoding = matched || 'UTF-8'
  form.eol = d?.eol && (CONVERT_EOLS as readonly string[]).includes(d.eol) ? d.eol : 'LF'
  form.bom = utf16Target.value
    ? utf16Bom.value
    : d?.bom === 'utf8' ? 'utf8' : 'none'
})

// UTF-16 目标下 BOM 由后端强制；切回其它编码时必须复位，否则会把 utf16le 这个
// 只对该编码合法的 BOM 提交给 GBK/UTF-8，后端按「BOM 与目标编码不匹配」直接拒绝。
/**
 * 只在用户主动改选编码时联动 BOM。不能用 watch(form.encoding)：打开对话框回填
 * 当前编码也会触发，会把「当前是 UTF-8 BOM」的初值清成 none，制造一次用户没要求的
 * 「去 BOM」转换。LE ↔ BE 互切同样走这里，避免 utf16le BOM 留在 UTF-16BE 上。
 */
function onEncodingPicked() {
  form.bom = utf16Target.value ? utf16Bom.value : 'none'
}

const canSubmit = computed(
  () =>
    !!targetPath.value &&
    !dirtyBlocked.value &&
    !converting.value &&
    hasConvertChange(form, current.value)
)

async function confirmConvert() {
  if (!canSubmit.value) return
  const path = targetPath.value
  const d = doc.value
  converting.value = true
  try {
    const res = await convertEncodingEol({
      path,
      targetEncoding: form.encoding,
      targetEol: form.eol,
      targetBom: form.bom,
      expectedMtimeMs: d?.diskMtimeMs,
      expectedSize: d?.diskSize
    })
    if (res?.code !== 200) {
      // 409 / 415 / 400 / 413 都保持原文件不变（后端已清掉本次备份），文案直接透传
      const hint = res?.code === 409
        ? '文件在被外部程序改动；请重新加载后再试'
        : res?.code === 415
          ? '当前内容含目标编码无法表示的字符，建议选 UTF-8'
          : ''
      NotificationUtil.error(`${res?.msg || '转换失败'}${hint ? `（${hint}）` : ''}`)
      return
    }
    if (res.data.backup_path) {
      await pushUndoBatch('编码 / 换行符转换', [
        { path, backup_path: res.data.backup_path }
      ])
    }
    // 磁盘字节已变：必须刷新文档的编码/EOL/BOM/mtime/size 基线，
    // 否则下一次保存仍按旧编码回写，等于把转换结果又改回去
    await refreshOpenDocuments([path])
    NotificationUtil.success(
      res.data.backup_path
        ? `已转为 ${res.data.encoding} / ${res.data.eol}，原文件已备份，可在撤销历史中恢复`
        : res.msg || '目标与当前一致，未写盘'
    )
    visible.value = false
  } catch (e: any) {
    NotificationUtil.error(e?.message || '转换异常')
  } finally {
    converting.value = false
  }
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as *;

.conv-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
  font-size: var(--fontSizeSmall);
  color: var(--textColor);
}
.conv-file {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.conv-warn {
  margin: 0;
  padding: 8px 10px;
  border-radius: 4px;
  line-height: 1.6;
  background: var(--stateWarningFill);
  color: var(--stateWarningFg);
  code { font-family: monospace; }
}
.conv-block {
  padding: 8px 10px;
  border-radius: 4px;
  line-height: 1.6;
  background: var(--stateErrorFill);
  color: var(--stateErrorFg);
}
.conv-current {
  color: var(--textSecondary);
  b { color: var(--textColor); font-weight: 700; }
}
.conv-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.conv-label {
  flex: 0 0 92px;
  text-align: right;
  color: var(--textSecondary);
}
.conv-select { flex: 1 1 auto; }
.conv-tip {
  margin: 0;
  line-height: 1.6;
  color: var(--textSecondary);
}
</style>
