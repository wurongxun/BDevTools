<template>
<pdfjs-viewer :pdf-src="pdfSrc" />
</template>

<script setup lang="ts">
import PdfjsViewer from "vue3-pdfjs";

export default {
components: { PdfjsViewer },
data() {
return {
pdfSrc: "/path/to/your/pdf/file.pdf", // PDF 文件路径
};
},
};
</script>