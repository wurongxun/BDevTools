<template>
    <div>
        <el-button @click="loadPdf">加载PDF</el-button>
        <div v-loading="loading">
            <div v-for="page in visiblePages" :key="page">
                <canvas :id="`pdf-page-${page}`"></canvas>
            </div>
        </div>
        <el-pagination layout="prev, pager, next" :total="pageCount" @current-change="handlePageChange" />
    </div>
    <iframe v-if="state.source" :src="state.source" class="pdf-iframe"></iframe>
    {{ state.source }}
    =========================
    {{  pdfDoc.value }}

    uuu==========================
    {{  pageCount }}
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import { reactive, onMounted, ref, computed, watch } from "vue";
import getDocument  from "pdfjs-dist";
import { useRoute, useRouter } from 'vue-router'
// 引入本地 PDF.js worker
import pdfjsWorker from 'pdfjs-dist/build/pdf.worker.min?url';
import { ElMessage } from 'element-plus';

// 获取当前路由信息
const route = useRoute()

const state = reactive({
    source: '', //预览pdf文件地址
    pageNum: 1, //当前页面
    scale: 1, // 缩放比例
    numPages: 44, // 总页数
});
// 设置 Worker 路径（必须！）
// PDFJS.GlobalWorkerOptions.workerSrc =
//   "https://cdn.jsdelivr.net/npm/pdfjs-dist@3.4.120/build/pdf.worker.min.js";
pdfjsLib.GlobalWorkerOptions.workerSrc = pdfjsWorker;

//const pdfUrl = "https://example.com/large-file.pdf"; // 20MB PDF
const loading = ref(false);
const pdfDoc = ref<any>(null);
const pageCount = ref(0);
const visiblePages = ref<number[]>([]); // 当前可见的页（避免一次性渲染全部）

// 加载PDF文档（但不会立即渲染所有页）
const loadPdf = async () => {
    loading.value = true;
    try {
        const loadingTask = pdfjsLib.getDocument({
            url: "http://asset.localhost/C%3A%5CUsers%5Ckun15%5CDocuments%5Cnotebooks%5C%E4%BA%8C%E4%BA%8C%5CJavaEE%E5%B8%B8%E8%A7%81%E9%9D%A2%E8%AF%95%E9%A2%98-JavaWEB%E7%AF%87.pdf",
            rangeChunkSize: 65536, // 分块加载（优化大文件）
        });
        pdfDoc.value = await loadingTask.promise;
        pageCount.value = pdfDoc.value.numPages;
        // 默认只加载第一页
        visiblePages.value = [1];
        await renderPage(1);
    } catch (error) {
        console.error("PDF加载失败:", error);
    } finally {
        loading.value = false;
    }
};

// 渲染单页
const renderPage = async (pageNumber: number) => {
    if (!pdfDoc.value) return;
    try {
        const page = await pdfDoc.value.getPage(pageNumber);
        const canvas = document.getElementById(`pdf-page-${pageNumber}`) as HTMLCanvasElement;
        const viewport = page.getViewport({ scale: 1.5 });
        canvas.width = viewport.width;
        canvas.height = viewport.height;
        await page.render({
            canvasContext: canvas.getContext("2d"),
            viewport,
        }).promise;
    } catch (error) {
        console.error(`渲染第${pageNumber}页失败:`, error);
        ElMessage.error(`渲染第${pageNumber}页失败`);
    }
};

// 翻页时动态加载
const handlePageChange = (newPage: number) => {
    if (!visiblePages.value.includes(newPage)) {
        visiblePages.value.push(newPage);
        renderPage(newPage);
    }
};
const loadingTask = getDocument(url);
loadingTask.promise.then(pdf => {
console.log('PDF loaded');

// Fetch the first page
const pageNumber = 1;
pdf.getPage(pageNumber).then(page => {
console.log('Page loaded');

const scale = 1.5;
const viewport = page.getViewport({ scale });

// Prepare canvas using PDF page dimensions
const canvas = document.getElementById('pdf-canvas');
const context = canvas.getContext('2d');
canvas.height = viewport.height;
canvas.width = viewport.width;

// Render PDF page into canvas context
const renderContext = {
canvasContext: context,
viewport: viewport
};
const renderTask = page.render(renderContext);
renderTask.promise.then(() => {
console.log('Page rendered');
});
});
});
// 根据路由参数加载文件
const loadPdfFile = async () => {
    console.log('加载文件 loadPdfFile:', route)
    let filePath = ""
    // 检查是否有动态路由参数
    if (route.meta.filePath) {
        filePath = decodeURIComponent(route.meta.filePath as string)
    }
    // 如果没有参数，使用查询参数或meta信息
    else if (route.query.file || route.meta.filePath) {
        filePath = (route.query.filePath as string) || (route.meta.filePath as string)
    }
    console.log('加载文件 loadPdfFile state.source:', state.source)
    const url = convertFileSrc(filePath)
    state.source = url;
    console.log('加载文件 loadPdfFile url:', url)
    // const loadingTask = createLoadingTask(url);
    // loadingTask.promise.then((pdf: { numPages: number }) => {
    //     state.numPages = pdf.numPages;
    // });
    loadPdf();
}

onMounted(() => {
    loadPdfFile();

});
</script>
