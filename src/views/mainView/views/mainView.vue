<template>
  <div class="excel-container">
    <div class="cards-grid">
      <el-card class="upload-card">
        <template #header>
          <div class="card-header">
            <span>Excel文件处理</span>
          </div>
        </template>
        <el-upload class="upload-demo" drag action="#" multiple ref="upload" :http-request="uploads" :limit="1"
          :on-exceed="handleExceed" accept=".xlsx">
          <el-icon class="el-icon--upload"><upload-filled /></el-icon>
          <div class="el-upload__text">
            将文件拖到此处，或<em>点击选择文件</em>
          </div>
        </el-upload>
      </el-card>

      <el-card class="upload-card">
        <template #header>
          <div class="card-header">
            <span>其他文件处理</span>
          </div>
        </template>
        <el-upload class="upload-demo" drag action="#" multiple ref="upload2" :http-request="uploads" :limit="1"
          :on-exceed="handleExceed" accept=".xlsx">
          <el-icon class="el-icon--upload"><upload-filled /></el-icon>
          <div class="el-upload__text">
            将文件拖到此处，或<em>点击选择文件</em>
          </div>
        </el-upload>
      </el-card>

      <el-card class="upload-card">
        <template #header>
          <div class="card-header">
            <span>数据导入</span>
          </div>
        </template>
        <el-upload class="upload-demo" drag action="#" multiple ref="upload3" :http-request="uploads" :limit="1"
          :on-exceed="handleExceed" accept=".xlsx">
          <el-icon class="el-icon--upload"><upload-filled /></el-icon>
          <div class="el-upload__text">
            将文件拖到此处，或<em>点击选择文件</em>
          </div>
        </el-upload>
      </el-card>

      <el-card class="upload-card">
        <template #header>
          <div class="card-header">
            <span>数据导出</span>
          </div>
        </template>
        <el-upload class="upload-demo" drag action="#" multiple ref="upload4" :http-request="uploads" :limit="1"
          :on-exceed="handleExceed" accept=".xlsx">
          <el-icon class="el-icon--upload"><upload-filled /></el-icon>
          <div class="el-upload__text">
            将文件拖到此处，或<em>点击选择文件</em>
          </div>
        </el-upload>
      </el-card>
    </div>

    <el-card class="instructions-card">
      <template #header>
        <div class="card-header">
          <span>使用说明</span>
        </div>
      </template>
      <ul>
        <li>支持.xlsx格式的Excel文件</li>
        <li>文件需包含4个工作表(sheet)</li>
        <li>工作表1必须包含"订单号"、"订单项目"、"计划行"列</li>
        <li>工作表2、3、4必须包含"订单号"、"订单项目"、"计划行"、"核实备注"列</li>
        <li>处理完成后会自动新生成的Excel文件</li>
      </ul>
    </el-card>
  </div>
</template>

<script lang="ts" setup>
import { ref } from 'vue'
import type { UploadInstance, UploadProps, UploadRawFile } from 'element-plus'
import { genFileId, ElNotification, ElMessage } from 'element-plus'
import { save } from '@tauri-apps/plugin-dialog'
import ExcelJS from "exceljs"
import { writeFile, BaseDirectory } from '@tauri-apps/plugin-fs'

const upload = ref<UploadInstance>()
const upload2 = ref<UploadInstance>()
const upload3 = ref<UploadInstance>()
const upload4 = ref<UploadInstance>()

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
  padding: 2px;
  box-sizing: border-box;
}

.cards-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
  gap: 20px;
  margin-bottom: 20px;
}

.upload-card {
  border-radius: 8px;
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
}

.card-header {
  font-weight: bold;
  font-size: 16px;
  color: #303133;
}

:deep(.upload-demo) {
  display: flex;
  justify-content: center;
}

:deep(.el-upload-dragger) {
  width: 100%;
  padding: 30px 20px;
}

:deep(.el-icon--upload) {
  font-size: 48px;
  color: #c0c4cc;
  margin-bottom: 16px;
}

:deep(.el-upload__text) {
  color: #606266;
  font-size: 14px;
  margin-bottom: 16px;
}

:deep(.el-upload__text em) {
  color: #409eff;
  font-style: normal;
}

.instructions-card {
  border-radius: 8px;
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
}

.instructions-card ul {
  padding-left: 20px;
  margin: 0;
}

.instructions-card li {
  color: #606266;
  margin-bottom: 8px;
  line-height: 1.5;
}

@media (max-width: 768px) {
  .excel-container {
    padding: 2px;
  }

  .cards-grid {
    grid-template-columns: 1fr;
    gap: 15px;
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
    grid-template-columns: repeat(3, 1fr);
  }
}
</style>