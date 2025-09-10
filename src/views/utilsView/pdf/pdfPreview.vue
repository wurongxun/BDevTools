<template>
    <h1>PDF.js Previous/Next example</h1>
    <p>Please use <a href="https://mozilla.github.io/pdf.js/getting_started/#download"><i>official releases</i></a> in
        production environments.</p>
    <div>
        <button id="prev">Previous</button>
        <button id="next">Next</button>
        &nbsp; &nbsp;
        <span>Page: <span id="page_num"></span> / <span id="page_count"></span></span>
    </div>
    <canvas id="the-canvas"></canvas>
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core'
import { reactive, onMounted, ref, computed, watch } from "vue";
import * as pdfjsLib from "pdfjs-dist";  // 修改这一行
import { useRoute, useRouter } from 'vue-router'
// 引入本地 PDF.js worker
import pdfjsWorker from 'pdfjs-dist/build/pdf.worker.min?url';
import { ElMessage } from 'element-plus';

// 获取当前路由信息
const route = useRoute()
var pdfDoc = null,
    pageNum = 1,
    pageRendering = false,
    pageNumPending = null,
    scale = 0.8,
    canvas =null,
    ctx = null;
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
canvas = document.getElementById('the-canvas') as HTMLCanvasElement,
    ctx = canvas.getContext('2d');

    document.getElementById('prev').addEventListener('click', onPrevPage);
    document.getElementById('next').addEventListener('click', onNextPage);

    pdfjsLib.getDocument(url).promise.then(function (pdfDoc_) {
        pdfDoc = pdfDoc_;
        document.getElementById('page_count').textContent = pdfDoc.numPages;

        // Initial/first page rendering
        renderPage(pageNum);
    });

    // const loadingTask = createLoadingTask(url);
    // loadingTask.promise.then((pdf: { numPages: number }) => {
    //     state.numPages = pdf.numPages;
    // });
    //loadPdf();
}
// If absolute URL from the remote server is provided, configure the CORS
// header on that server.
//var url = 'https://raw.githubusercontent.com/mozilla/pdf.js/ba2edeae/web/compressed.tracemonkey-pldi-09.pdf';

// Loaded via <script> tag, create shortcut to access PDF.js exports.
//   var { pdfjsLib } = globalThis;

// The workerSrc property shall be specified.
//pdfjsLib.GlobalWorkerOptions.workerSrc = '//mozilla.github.io/pdf.js/build/pdf.worker.mjs';



/**
 * Get page info from document, resize canvas accordingly, and render page.
 * @param num Page number.
 */
function renderPage(num) {
    pageRendering = true;
    // Using promise to fetch the page
    pdfDoc.getPage(num).then(function (page) {
        var viewport = page.getViewport({ scale: scale });
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
    });

    // Update page counters
    document.getElementById('page_num').textContent = num;
}

/**
 * If another page rendering in progress, waits until the rendering is
 * finised. Otherwise, executes rendering immediately.
 */
function queueRenderPage(num) {
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
    if (pageNum <= 1) {
        return;
    }
    pageNum--;
    queueRenderPage(pageNum);
}

/**
 * Displays next page.
 */
function onNextPage() {
    if (pageNum >= pdfDoc.numPages) {
        return;
    }
    pageNum++;
    queueRenderPage(pageNum);
}

/**
 * Asynchronously downloads PDF.
 */

onMounted(() => {

    loadPdfFile();

});
</script>

<style scoped>
#the-canvas {
    border: 1px solid black;
    direction: ltr;
}
</style>