<template>
  <div class="box" ref="box">
    <div class="mid">
      <div v-if="data.showMdEditor" class="editor-wrapper">
        <MdEditor
          ref="editorRef"
          :previewOnly=true
          v-model="data.editedMdtext_md"
          @onSave="saveMdText"
          :theme="editorTheme"
          class="md-editor"
          @onError="onError"
          @change="onContentChange"
        />
      </div>
      <div v-else class="editor-placeholder">
        <div class="placeholder-content">
          <el-icon :size="48">
            <Document />
          </el-icon>
          <p class="placeholder-text">选择或创建一个笔记开始编辑</p>
          <el-button v-if="notebooks_data && notebooks_data.children && notebooks_data.children.length > 0"
                     class="create-note-button">
            创建新笔记
          </el-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts" name="MdEditor">
import {
  computed,
  h,
  defineComponent,
  onMounted,
  reactive,
  ref,
  toRefs,
  createVNode,
  toRaw,
  onActivated,
  onDeactivated,
  watch,
  onUnmounted,
  nextTick,
} from "vue";
import { ElMessage } from "element-plus";
import NotificationUtil from "@/utils/NotificationUtil";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { MdEditor } from "md-editor-v3";
import type { ExposeParam } from 'md-editor-v3';
import "md-editor-v3/lib/style.css";
import { ResponseData } from "@/entity/entitys";
import { load, Store } from '@tauri-apps/plugin-store';
import { tauri_request } from "@/utils/tauri_request";
import { Document } from '@element-plus/icons-vue';
import { useDark } from '@vueuse/core'
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import useStore from '@/store'
import { pdf_store_tem_key_prefix, pdf_store_tem_key, pdf_store_conten_key_prefix } from '@/setting'
import { FileType } from '@/store/modules/documents'

// --- 状态与 Store 初始化 ---
let store: Store | undefined = undefined;
const { noteTem, useDocumentManagerStore } = useStore()
const docStore = useDocumentManagerStore

// 暗黑模式
const isDark = useDark({
  storageKey: 'theme-appearance',
  valueDark: 'dark',
  valueLight: 'light'
})

const editorTheme = computed(() => {
  return isDark.value ? 'dark' : 'light';
});

// 响应式数据
const data = reactive({
  folders: [],
  files: [] as { title: string }[],
  selectedKeys: [],
  currentChooseFolderPath_md: "",
  currentChooseNoteAllPath_md: "",
  notEditedMdtext_md: "",
  editedMdtext_md: "",
  title_md: "",
  showMdEditor: false,
});

// 原代码中定义了但未使用的变量，保留以维持结构兼容性，但标记为可能无用
let notebooks_data = ref<any>(undefined);

// 编辑器引用
const editorRef = ref<ExposeParam>();
// 监听窗口 resize 事件，强制刷新编辑器
let resizeTimer: ReturnType<typeof setTimeout> | null = null;

const handleWindowResize = async () => {
  if (resizeTimer) {
    clearTimeout(resizeTimer);
  }
  resizeTimer = setTimeout(async () => {
    await nextTick();

    // 如果编辑器实例存在，尝试触发其内部的重绘
    if (editorRef.value) {
      console.log('[mdPreViewAndEdit] Window resized, refreshing editor');
    }

    // 触发编辑器的滚动同步（如果有）
    const scrollEvent = new CustomEvent('editor-scroll-sync');
    window.dispatchEvent(scrollEvent);
  }, 250);
};

// 当前激活的文档路径
const activeDocId = computed(() => docStore.activeDocumentId)

// 根据文档ID加载文件
// 根据文档ID加载文件
const loadFile = async (docId: string) => {
  if (!docId) {
    data.showMdEditor = false;
    return;
  }
  // 从 document store 获取文档信息
  const doc = docStore.documents.get(docId)
  console.log("docdocdocdocdoc",doc)
  if (!doc) {
    console.warn(`Document not found: ${docId}`)
    data.showMdEditor = false
    return
  }
  // 如果没有缓存，从文件中读取
  if (doc.id) {
    data.showMdEditor = true;
    // 读取成功后，更新元数据并缓存
    if (data.showMdEditor) {
      data.title_md = doc.name;
      data.currentChooseNoteAllPath_md = doc.id;

      // 确保 content 是字符串类型
      if (typeof doc.content === 'string') {
        data.editedMdtext_md = doc.content;
      } else if (doc.content instanceof ArrayBuffer) {
        // 如果是 ArrayBuffer，转换为字符串（通常用于二进制文件）
        const decoder = new TextDecoder('utf-8');
        data.editedMdtext_md = decoder.decode(doc.content);
      } else {
        // 默认为空字符串
        data.editedMdtext_md = '';
      }
    }
  } else {
    NotificationUtil.error("文件路径缺失");
  }

};

// 监听激活文档变化 - 移除 immediate，避免与 onMounted 重复加载
watch(activeDocId, async (newId) => {
  console.log(newId)
  if (newId) {
    await loadFile(newId)
  } else {
    data.showMdEditor = false
  }
})


onMounted(async () => {
  //页面布局
  //dragControllerDiv();

  //关闭窗口保存 监听
  listenClose();

  editorRef.value?.on('catalog', console.log);
  // 监听窗口 resize 事件
  window.addEventListener('resize', handleWindowResize);
  // 监听来自 AppMain 的自定义 resize 事件
  window.addEventListener('window-resize', handleWindowResize);

  // 初始加载当前文档
  if (activeDocId.value) {
    await loadFile(activeDocId.value)
  }
})

onUnmounted(() => {
  // 清理事件监听器
  window.removeEventListener('resize', handleWindowResize);
  window.removeEventListener('window-resize', handleWindowResize);

  if (resizeTimer) {
    clearTimeout(resizeTimer);
  }
});

// 组件被激活时（从缓存中恢复）- 数据保持在内存中，无需重新加载
onActivated(() => {
  console.log('[mdPreViewAndEdit] Activated - 从缓存恢复')
})

// 组件被停用时（保存到缓存）
onDeactivated(() => {
  console.log('[mdPreViewAndEdit] Deactivated - 保存到缓存')
  // 保存当前文档状态到 store
  if (activeDocId.value && data.editedMdtext_md !== data.notEditedMdtext_md) {
    docStore.updateDocumentContent(activeDocId.value, data.editedMdtext_md)
  }
})



// 事件监听清理函数
let unlistenClose: UnlistenFn | null = null;

// --- 核心逻辑 ---

// 防抖定时器 ID
let saveTimeoutId: number | null = null;

// 监听内容变化，实时保存到 pinia (带防抖)
const onContentChange = async (value: string) => {
  data.editedMdtext_md = value;
  if (!activeDocId.value) {
    return;
  }
  // 清除之前的定时器
  if (saveTimeoutId !== null) {
    clearTimeout(saveTimeoutId);
  }
  // 设置新的防抖保存 (例如 500ms)
  saveTimeoutId = window.setTimeout(async () => {
    try {
      // 更新文档内容到 store
      docStore.updateDocumentContent(activeDocId.value!, value)
      // 同时保存到本地临时存储
      if (store) {
        await store.set(pdf_store_tem_key_prefix + activeDocId.value, { value: value });
        await store.save();
        console.log("内容已防抖保存至临时存储");
      }
    } catch (error) {
      console.error("保存临时内容失败:", error);
      NotificationUtil.error("缓存数据保存失败！");
    }
  }, 500);
};

// 初始化 Store
const initStore = async () => {
  if (store == null || store == undefined) {
    try {
      store = await load(pdf_store_tem_key, {
        autoSave: false,
        defaults: {}
      });
    } catch (error) {
      console.error("初始化 Store 失败:", error);
      NotificationUtil.error("本地存储初始化失败");
    }
  }
};

// 读取文件内容
const read_note_file_bypath = async function (path: string) {
  if (!path) {
    NotificationUtil.error("文件路径为空");
    return;
  }

  try {
    const res = await tauri_request<ResponseData<string>>("read_note_file", {
      path: path
    });

    if (res && res.code === 200 && res.data !== undefined) {
      const content = res.data;
      data.editedMdtext_md = content;
      data.notEditedMdtext_md = content;
      data.showMdEditor = true;
    } else {
      NotificationUtil.error("读取失败：" + (res?.msg || "未知错误"));
    }
  } catch (error) {
    console.error("读取文件异常:", error);
    NotificationUtil.error("读取文件发生异常");
  }
};


// 监听关闭事件
const listenClose = async () => {
  // 先取消之前的监听，防止重复注册
  if (unlistenClose) {
    await unlistenClose();
  }

  try {
    unlistenClose = await listen('close', async () => {
      // 关闭前保存
      if (data.currentChooseNoteAllPath_md &&
        data.editedMdtext_md !== data.notEditedMdtext_md &&
        data.title_md) {
        await saveMd(data.editedMdtext_md, data.currentChooseNoteAllPath_md);
      }
      await getCurrentWindow().close();
    });
  } catch (error) {
    console.error("监听关闭事件失败:", error);
  }
};

// 错误处理
const onError = (err: { message: string; }) => {
  NotificationUtil.error(err.message || "编辑器发生错误");
};

/**
 * 保存笔记核心逻辑
 * @param mdText 笔记内容
 * @param path 文件路径
 */
const saveMd = async (mdText: string, path: string) => {
  if (!mdText || !path) {
    NotificationUtil.error("请选定笔记本和笔记，且内容不能为空");
    return false;
  }

  try {
    const res = await tauri_request<ResponseData<string[]>>("save_note", {
      path: path,
      mdText: mdText,
    });
    if (res && res.code === 200) {
      data.notEditedMdtext_md = mdText;
      NotificationUtil.success("笔记保存成功");
      // 保存成功后，更新永久缓存
      if (store) {
        await store.set(pdf_store_conten_key_prefix + path, { value: mdText });
        // 临时缓存也更新，保持一致
        await store.set(pdf_store_tem_key_prefix + path, { value: mdText });
        await store.save();
      }
      // 更新 store 中的文档状态
      docStore.updateDocumentContent(path, mdText)
      return true;
    } else {
      NotificationUtil.error("笔记保存失败：" + (res?.msg || "未知错误"));
      return false;
    }
  } catch (error) {
    console.error("保存笔记异常:", error);
    NotificationUtil.error("笔记保存发生异常");
    return false;
  }
};

/**
 * 编辑器点击保存按钮
 */
const saveMdText = async () => {
  if (!data.title_md) {
    NotificationUtil.error("请输入标题");
    return;
  }

  if (!data.currentChooseNoteAllPath_md) {
    NotificationUtil.error("当前笔记路径无效，无法保存");
    return;
  }

 await saveMd(data.editedMdtext_md, data.currentChooseNoteAllPath_md);
};

// 占位符按钮点击事件（原代码未实现，补充空函数避免报错）
const handleCreateNote = () => {
  console.log("创建新笔记功能待实现");
  // TODO: 实现创建笔记逻辑
};

onMounted(async () => {
  await listenClose();
  // dragControllerDiv(); // 原代码注释掉的功能，若需启用请确保正确实现并处理销毁
});

onUnmounted(async () => {
  // 清理事件监听
  if (unlistenClose) {
    await unlistenClose();
    unlistenClose = null;
  }
  // 清除防抖定时器
  if (saveTimeoutId !== null) {
    clearTimeout(saveTimeoutId);
  }
});

</script>

<style scoped>
.box {
  width: 100%;
  height: 100%;
  margin: 0;
  overflow: hidden;
  box-shadow: -1px 9px 10px 3px rgba(0, 0, 0, 0.11);
  display: flex;
}

/* 原代码中有 .left 样式但模板中无对应元素，保留以防外部依赖或未来使用 */
.left {
  width: calc(15% - 10px);
  height: 100%;
}

.resize {
  cursor: col-resize;
  width: 8px;
  height: 50px;
  position: relative;
  top: 45%;
  margin-top: -10px;
  border-radius: 5px;
  font-size: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.resize:hover {
  background-color: #c2c2c2;
  color: #444444;
}

.mid {
  flex: 1;
  height: 100%;
  box-shadow: -1px 4px 5px 3px rgba(0, 0, 0, 0.11);
}

.sidebar-content {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.sidebar-header {
  display: flex;
  justify-content: space-between;
  padding: 10px 8px;
  border-bottom: 1px solid #e4e7ed;
}

.sidebar-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.sidebar-controls {
  display: flex;
  gap: 5px;
  margin-left: 5px;
}

.folder-menu {
  border: none;
  flex: 1;
  overflow-y: auto;
  padding: 5px 0;
  margin: 0;
}

.wrapper {
  height: 100%;
  display: flex;
  flex-direction: column;
  flex: 1;
}

.empty-state {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.editor-wrapper {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.md-editor {
  flex: 1;
  min-height: 0;
}

.editor-placeholder {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.placeholder-content {
  text-align: center;
  color: #909399;
}

.placeholder-text {
  margin-top: 16px;
  font-size: 16px;
}

.create-note-button {
  margin-top: 20px;
}

.context-menu :deep(.v-contextmenu-item) {
  display: flex;
  align-items: center;
  gap: 8px;
}

.custom-dialog :deep(.el-dialog__body) {
  padding: 20px 20px 10px;
}

.dialog-footer {
  text-align: right;
}

/* 响应式优化 */
@media (max-width: 768px) {
  .box {
    flex-direction: column;
  }

  .left {
    width: 100%;
    height: 40%;
  }

  .mid {
    width: 100%;
    height: 60%;
  }

  .resize {
    display: none;
  }
}

.custom-dialog {
  border-radius: 8px;
  overflow: hidden;
}

.custom-dialog :deep(.el-dialog__header) {
  padding: 20px 20px 10px;
  border-bottom: 1px solid var(--el-border-color-light);
}

.custom-dialog :deep(.el-dialog__title) {
  font-size: 18px;
  font-weight: 600;
}

.custom-dialog :deep(.el-dialog__body) {
  padding: 20px;
}

.custom-dialog :deep(.el-dialog__footer) {
  padding: 15px 20px 20px;
  border-top: 1px solid var(--el-border-color-light);
}

.dialog-form {
  margin-right: 20px;
}

.dialog-form :deep(.el-form-item) {
  margin-bottom: 20px;
}

.dialog-form :deep(.el-form-item__label) {
  font-weight: 500;
}

.dialog-form :deep(.el-input__wrapper) {
  width: 100%;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}

.dialog-footer .el-button {
  padding: 8px 20px;
}
</style>
