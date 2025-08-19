<template>
  <div class="excel-container" ref="excelContainer">
    <div class="cards-grid">
      <el-card class="upload-card" shadow="hover">
        <template #header>
          <div class="card-header">
            <el-icon class="card-icon">
              <Document />
            </el-icon>
            <span>Excel文件处理</span>
          </div>
        </template>
        <el-upload class="upload-demo" drag action="#" multiple ref="upload" :http-request="uploads" :limit="1"
          :on-exceed="handleExceed" accept=".xlsx">
          <el-icon class="el-icon--upload">
            <UploadFilled />
          </el-icon>
          <div class="el-upload__text">
            将文件拖到此处，或<em>点击选择文件</em>
          </div>
          <div class="el-upload__tip">支持 .xlsx 格式文件</div>
        </el-upload>
      </el-card>

      <el-card class="upload-card" shadow="hover">
        <template #header>
          <div class="card-header">
            <el-icon class="card-icon">
              <Files />
            </el-icon>
            <span>其他文件处理</span>
          </div>
        </template>
        <el-upload class="upload-demo" drag action="#" multiple ref="upload2" :http-request="uploads" :limit="1"
          :on-exceed="handleExceed" accept=".xlsx">
          <el-icon class="el-icon--upload">
            <UploadFilled />
          </el-icon>
          <div class="el-upload__text">
            将文件拖到此处，或<em>点击选择文件</em>
          </div>
          <div class="el-upload__tip">支持 .xlsx 格式文件</div>
        </el-upload>
      </el-card>

      <el-card class="upload-card" shadow="hover">
        <template #header>
          <div class="card-header">
            <el-icon class="card-icon">
              <Upload />
            </el-icon>
            <span>数据导入</span>
          </div>
        </template>
        <el-upload class="upload-demo" drag action="#" multiple ref="upload3" :http-request="uploads" :limit="1"
          :on-exceed="handleExceed" accept=".xlsx">
          <el-icon class="el-icon--upload">
            <UploadFilled />
          </el-icon>
          <div class="el-upload__text">
            将文件拖到此处，或<em>点击选择文件</em>
          </div>
          <div class="el-upload__tip">支持 .xlsx 格式文件</div>
        </el-upload>
      </el-card>

      <el-card class="upload-card" shadow="hover">
        <template #header>
          <div class="card-header">
            <el-icon class="card-icon">
              <Download />
            </el-icon>
            <span>数据导出</span>
          </div>
        </template>
        <el-upload class="upload-demo" drag action="#" multiple ref="upload4" :http-request="uploads" :limit="1"
          :on-exceed="handleExceed" accept=".xlsx">
          <el-icon class="el-icon--upload">
            <UploadFilled />
          </el-icon>
          <div class="el-upload__text">
            将文件拖到此处，或<em>点击选择文件</em>
          </div>
          <div class="el-upload__tip">支持 .xlsx 格式文件</div>
        </el-upload>
      </el-card>
    </div>

    <el-card class="instructions-card" shadow="never">
      <template #header>
        <div class="card-header">
          <el-icon class="card-icon">
            <InfoFilled />
          </el-icon>
          <span>使用说明</span>
        </div>
      </template>
      <ul class="instructions-list">
        <li class="instruction-item">
          <el-icon class="instruction-icon">
            <Check />
          </el-icon>
          支持.xlsx格式的Excel文件
        </li>
        <li class="instruction-item">
          <el-icon class="instruction-icon">
            <Check />
          </el-icon>
          文件需包含4个工作表(sheet)
        </li>
        <li class="instruction-item">
          <el-icon class="instruction-icon">
            <Check />
          </el-icon>
          工作表1必须包含"订单号"、"订单项目"、"计划行"列
        </li>
        <li class="instruction-item">
          <el-icon class="instruction-icon">
            <Check />
          </el-icon>
          工作表2、3、4必须包含"订单号"、"订单项目"、"计划行"、"核实备注"列
        </li>
        <li class="instruction-item">
          <el-icon class="instruction-icon">
            <Check />
          </el-icon>
          处理完成后会自动新生成的Excel文件
        </li>
      </ul>
    </el-card>
  </div>
</template>

<script lang="ts" setup>
import { ref, onMounted, onBeforeUnmount } from 'vue'
import type { UploadInstance, UploadProps, UploadRawFile } from 'element-plus'
import { genFileId, ElNotification, ElMessage } from 'element-plus'
import { save } from '@tauri-apps/plugin-dialog'
import ExcelJS from "exceljs"
import { writeFile, BaseDirectory } from '@tauri-apps/plugin-fs'
// 导入图标组件
import { Document, Files, Upload, Download, InfoFilled, Check, UploadFilled } from '@element-plus/icons-vue'

const upload = ref<UploadInstance>()
const upload2 = ref<UploadInstance>()
const upload3 = ref<UploadInstance>()
const upload4 = ref<UploadInstance>()

// 添加滚动相关的响应式数据
const excelContainer = ref<HTMLElement | null>(null)

const uploads = async (val: { file: any; }) => {
  const file = val.file
  if (file) {
    if (!file.name.endsWith(".xlsx") && !file.name.endsWith(".XLSX")) {
      ElMessage.error("请上传xlsx或.XLSX文件")
      return
    }
    const arrayBuffer = await file.arrayBuffer()
    if (!arrayBuffer) return
    fileHandle(arrayBuffer)
  }
}

const handleExceed: UploadProps['onExceed'] = async (files) => {
  upload.value!.clearFiles()
  const file = files[0] as UploadRawFile
  try {
    if (!file.name.endsWith(".xlsx") && !file.name.endsWith(".XLSX")) {
      ElMessage.error("请上传xlsx或.XLSX文件")
      return
    }
    const arrayBuffer = await file.arrayBuffer()
    fileHandle(arrayBuffer)
  } catch (error) {
    ElMessage.error('文件解析错误')
  }
  file.uid = genFileId()
  upload.value!.handleStart(file)
}

// 滚动事件处理函数
const handleScroll = () => {
  if (!excelContainer.value) return

  const container = excelContainer.value
  const { scrollTop, scrollHeight, clientHeight } = container
  const distanceToBottom = scrollHeight - scrollTop - clientHeight

  // 当距离底部小于等于50px时，添加类名
  if (distanceToBottom <= 50) {
    container.classList.add('near-bottom')
  } else {
    container.classList.remove('near-bottom')
  }
}

// 添加和移除滚动事件监听器
onMounted(() => {

})

onBeforeUnmount(() => {
  if (excelContainer.value) {
    excelContainer.value.removeEventListener('scroll', handleScroll)
  }
})

// 保留原有的 fileHandle 函数和其他辅助函数
const fileHandle = async (file: ExcelJS.Buffer) => {
  // ... 原有实现保持不变 ...
}

// ... 其他辅助函数保持不变 ...
</script>
<style scoped>
.excel-container {
  width: 100%;
  height: 100%;
  padding: 3px;
  box-sizing: border-box;
  overflow-y: auto;
  background-color: var(--el-bg-color-page);
}

/* 当接近底部时，为容器添加底部边距 */
.excel-container.near-bottom {
  padding-bottom: 80px;
}

.page-header {
  text-align: center;
  margin-bottom: 30px;
  animation: fadeInDown 0.5s ease;
}

.page-title {
  font-size: 28px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  margin-bottom: 10px;
}

.page-subtitle {
  font-size: 16px;
  color: var(--el-text-color-secondary);
}

.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
  gap: 24px;
  margin-bottom: 30px;
  animation: fadeInUp 0.6s ease;
}

.upload-card {
  border-radius: var(--border-radius);
  box-shadow: var(--el-box-shadow-light);
  transition: all 0.3s ease;
  border: 1px solid var(--el-border-color-lighter);
}

.upload-card:hover {
  transform: translateY(-5px);
  box-shadow: var(--el-box-shadow);
  border-color: var(--el-color-primary-light-5);
}

.card-header {
  display: flex;
  align-items: center;
  font-weight: 600;
  font-size: 18px;
  color: var(--el-text-color-primary);
}

.card-icon {
  margin-right: 8px;
  font-size: 20px;
  color: var(--el-color-primary);
}

:deep(.upload-demo) {
  display: flex;
  justify-content: center;
}

:deep(.el-upload-dragger) {
  width: 100%;
  padding: 30px 20px;
  border-radius: var(--border-radius);
  border: 2px dashed var(--el-border-color-lighter);
  transition: all 0.3s ease;
}

:deep(.el-upload-dragger:hover) {
  border-color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-9);
}

:deep(.el-upload-dragger.is-dragover) {
  border-color: var(--el-color-primary);
  background-color: var(--el-color-primary-light-9);
}

:deep(.el-icon--upload) {
  font-size: 48px;
  color: var(--el-color-primary-light-3);
  margin-bottom: 16px;
}

:deep(.el-upload__text) {
  color: var(--el-text-color-primary);
  font-size: 14px;
  margin-bottom: 10px;
}

:deep(.el-upload__text em) {
  color: var(--el-color-primary);
  font-style: normal;
  font-weight: 500;
}

:deep(.el-upload__tip) {
  color: var(--el-text-color-secondary);
  font-size: 12px;
  text-align: center;
}

.instructions-card {
  border-radius: var(--border-radius);
  box-shadow: var(--el-box-shadow-light);
  border: 1px solid var(--el-border-color-lighter);
  transition: all 0.3s ease;
  animation: fadeInUp 0.7s ease;
}

.instructions-card:hover {
  box-shadow: var(--el-box-shadow);
}

.instructions-card .card-header {
  font-size: 18px;
}

.instructions-list {
  padding-left: 0;
  margin: 0;
}

.instruction-item {
  display: flex;
  align-items: flex-start;
  color: var(--el-text-color-primary);
  margin-bottom: 12px;
  line-height: 1.6;
  font-size: 14px;
}

.instruction-icon {
  color: var(--el-color-success);
  font-size: 16px;
  margin-right: 8px;
  margin-top: 3px;
}

.excel-container.near-bottom .instructions-card {
  margin-bottom: 50px;
}

@media (max-width: 768px) {
  .excel-container {
    padding: 3px;
  }

  .page-title {
    font-size: 24px;
  }

  .cards-grid {
    grid-template-columns: 1fr;
    gap: 20px;
    margin-bottom: 20px;
  }

  :deep(.el-upload-dragger) {
    padding: 20px 10px;
  }
}

@media (min-width: 769px) and (max-width: 1200px) {
  .cards-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (min-width: 1201px) {
  .cards-grid {
    grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
  }
}

/* 动画效果 */
@keyframes fadeInDown {
  from {
    opacity: 0;
    transform: translateY(-20px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(20px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>