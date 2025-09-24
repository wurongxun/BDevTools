<template>
    <div class="pdf-container">
        <canvas id="the-canvas"></canvas>
        <div class="pdf-controls">
            <button @click="onMagnify">+</button>
            <button @click="onReduce">-</button>
            <button id="prev">Previous</button>
            <button id="next">Next</button>
            <span>Page: <span>{{ page_num }}</span> / <span>{{ page_count }}</span></span>
        </div>
    </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref, watch } from "vue";
import * as pdfjsLib from "pdfjs-dist";
import { useRoute } from 'vue-router'
// 引入本地 PDF.js worker
import pdfjsWorker from 'pdfjs-dist/build/pdf.worker.min?url';
import { ElMessage } from 'element-plus';
import { convertFileSrc } from '@tauri-apps/api/core'


// 获取当前路由信息
const route = useRoute()
let scale = ref<number>(1);
let page_num = ref<string>("");
let page_count = ref<string>("");
var pdfDoc: any = null,
    pageNum = 1,
    pageRendering = false,
    pageNumPending: number | null = null,
    canvas: HTMLCanvasElement | null = null,
    ctx: CanvasRenderingContext2D | null = null;

// 设置 Worker 路径（必须！）
pdfjsLib.GlobalWorkerOptions.workerSrc = pdfjsWorker;

const onMagnify = () => {
    if (!pdfDoc || !ctx) return;
    scale.value += 0.1;
    renderPage(pageNum);
    scrollToTop(); // 添加这行
}
const onReduce = () => {
    if (!pdfDoc || !ctx) return;
    if (scale.value > 0.2) {
        scale.value -= 0.1;
        renderPage(pageNum);
        scrollToTop(); // 添加这行
    }

}
// 添加新的滚动函数
function scrollToTop() {
    // 方法1: 滚动到指定元素顶部
    //document.querySelector('.pdf-container')?.scrollIntoView();

    // 或者方法2: 直接滚动到页面顶部
    // window.scrollTo({ top: 0, behavior: 'smooth' });

    // 或者方法3: 滚动容器内部滚动到顶部(如果容器有滚动条)
    document.querySelector('.pdf-container')?.scrollTo({ top: 0, behavior: 'smooth' });
}
// 根据路由参数加载文件
const loadPdfFile = async () => {
    try {
        let filePath = ""
        // 检查是否有动态路由参数
        if (route.meta.filePath) {
            filePath = decodeURIComponent(route.meta.filePath as string)
        }
        // 如果没有参数，使用查询参数
        else if (route.query.file) {
            filePath = route.query.file as string;
        }

        const url = convertFileSrc(filePath)
        canvas = document.getElementById('the-canvas') as HTMLCanvasElement;
        ctx = canvas.getContext('2d');

        document.getElementById('prev')?.addEventListener('click', onPrevPage);
        document.getElementById('next')?.addEventListener('click', onNextPage);

        const loadingTask = pdfjsLib.getDocument(url);
        loadingTask.promise.then(function (pdfDoc_) {
            pdfDoc = pdfDoc_;

            page_count.value = pdfDoc_.numPages.toString();

            // Initial/first page rendering
            renderPage(pageNum);
        });
    } catch (error: any) {
        ElMessage.error(error.message)
    }
}

/**
 * Get page info from document, resize canvas accordingly, and render page.
 * @param num Page number.
 */
function renderPage(num: number) {
    if (!pdfDoc || !ctx || !canvas) return;

    pageRendering = true;
    // Using promise to fetch the page
    pdfDoc.getPage(num).then(function (page: any) {
        var viewport = page.getViewport({ scale: scale.value });
        if (canvas && ctx) {
            canvas.height = viewport.height;
            canvas.width = viewport.width;

            // Render PDF page into canvas context
            var renderContext = {
                canvasContext: ctx,
                viewport: viewport
            };
            var renderTask = page.render(renderContext);

            // Wait for rendering to finish
            renderTask.promise.then(function () {
                pageRendering = false;
                if (pageNumPending !== null) {
                    // New page rendering is pending
                    renderPage(pageNumPending);
                    pageNumPending = null;
                }
            });
        }
    });

    // Update page counters
    page_num.value = num.toString();

}

/**
 * If another page rendering in progress, waits until the rendering is
 * finished. Otherwise, executes rendering immediately.
 */
function queueRenderPage(num: number) {
    if (pageRendering) {
        pageNumPending = num;
    } else {
        renderPage(num);
    }
}

/**
 * Displays previous page.
 */
function onPrevPage() {
    if (pageNum <= 1 || !pdfDoc) {
        return;
    }
    pageNum--;
    queueRenderPage(pageNum);
}

/**
 * Displays next page.
 */
function onNextPage() {
    if (!pdfDoc || pageNum >= pdfDoc.numPages) {
        return;
    }
    pageNum++;
    queueRenderPage(pageNum);
}
// 监听路由参数变化
watch(
    () => route.path,
    () => {
       loadPdfFile();
    },
    { immediate: true }
)
onMounted(() => {
    loadPdfFile();
});
</script>
<style scoped>
.pdf-container {
    width: 100%;
    height: 100%;
    position: relative;
    min-height: 100%;
    /* background-color: #f5f5f5; */
    padding: 10px 0;
    box-sizing: border-box;
    /* overflow: scroll; */
}

#the-canvas {
    width: 100%;
    direction: ltr;
    max-width: 100%;
    display: block;
    margin: 0 auto;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    border-radius: 4px;
    /* background-color: white; */
}

.pdf-controls {
    position: fixed;
    bottom: 20px;
    left: 50%;
    /* transform: translateX(-50%); */
    display: flex;
    align-items: center;
    gap: 15px;
    /* background-color: rgba(255, 255, 255, 0.95); */
    padding: 6px 6px;
    border-radius: 30px;
    box-shadow: 0 4px 15px rgba(0, 0, 0, 0.15);
    z-index: 100;
    backdrop-filter: blur(5px);
    border: 1px solid rgba(0, 0, 0, 0.05);
}

.pdf-controls button {
    padding: 8px 16px;
    border: 1px solid #ddd;
    /* background-color: #fff; */
    border-radius: 20px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.2s ease;
    min-width: 80px;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.05);
}

.pdf-controls button:hover:not(:disabled) {
    /* background-color: #409eff;
    color: white;
    border-color: #409eff; */
    transform: translateY(-2px);
    box-shadow: 0 4px 10px rgba(0, 0, 0, 0.1);
}

.pdf-controls button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.pdf-controls span {
    font-weight: 500;
    color: #333;
    font-size: 14px;
    min-width: 80px;
    text-align: center;
}
</style>