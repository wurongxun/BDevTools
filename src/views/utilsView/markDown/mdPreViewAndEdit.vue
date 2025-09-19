<template>
    <div class="box" ref="box">
        <div class="mid">
            <div v-if="data.showMdEditor" class="editor-wrapper">
                <MdEditor v-model="data.editedMdtext_md" @onSave="saveMdText" :theme="editorTheme" class="md-editor"
                    @onError="onError" @change="onContentChange" />
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
<script setup lang="ts">
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
} from "vue";
//import { notification, Modal } from 'ant-design-vue';
import { ElMessage, ElMessageBox, ElSubMenu, ElMenuItem } from "element-plus";
import DirFileMenu from "@/utils/DirFileMenu";
import StringUtil from "@/utils/StringUtil";
import NotificationUtil from "@/utils/NotificationUtil";
import { invoke } from "@tauri-apps/api/core";
import { openPath } from '@tauri-apps/plugin-opener';
import { getCurrentWindow } from "@tauri-apps/api/window";
import { MdEditor } from "md-editor-v3";
import "md-editor-v3/lib/style.css";
import { NotebookInfo, ResponseData } from "@/entity/entitys";

// import TreeNode from "@/components/tsx/menu/TreeNode";
import { tauri_request } from "@/utils/tauri_request";
import {
    Document,
    Refresh,
    DocumentAdd,
    FolderAdd,
    Delete,
    Edit,
    FolderOpened
} from '@element-plus/icons-vue';
import { getCurrentWebview } from "@tauri-apps/api/webview";
// 引入VueUse暗黑模式核心
import { useDark, useToggle } from '@vueuse/core'
import { listen } from "@tauri-apps/api/event";
import { useRoute, useRouter } from 'vue-router'
import { NoteDataStore } from '@/Types/entitys'
import useStore from '@/store/index'
const { noteTem } = useStore()

// 获取当前路由信息
const route = useRoute()

// 初始化暗黑模式检测器
// 初始化暗黑模式检测器，配置持久化选项
const isDark = useDark({
    storageKey: 'theme-appearance', // 自定义localStorage存储名称
    valueDark: 'dark', // 类名暗黑模式下值是dark
    valueLight: 'light' // 类名亮色模式下值是light
})

// 根据主题状态动态设置编辑器主题
const editorTheme = computed(() => {
    return isDark.value ? 'dark' : 'light';
});

const data = reactive({
    folders: [], // 文件夹列表
    files: [] as { title: string }[], // 文件列表
    selectedKeys: [], // 笔记本选中项key
    selectFolderPath: "", // 当前选中文件夹 用于建文件和子文件夹
    selectIsFile: false,// 当前选中文件还是文件夹
    currentChooseFolderPath_md: "", // 当前选中文件的文件夹
    currentChooseNoteAllPath_md: "", // 当前选中笔记 全路径
    notEditedMdtext_md: "", // 未编辑的文件内容
    editedMdtext_md: "", // 文章内容Markdown格式
    title_md: "", // 文章标题
    lastTimeTitle_md: "", //旧标题
    showMdEditor: false, // 是否显示编辑器
    newFolderName: "", // 新建文件夹夹的名称
    newFileName: "", // 新建的笔记文件名
    upFileName: "",
    confirmLoading: false, // 添加笔记表单等待状态（是否转圈圈）
    formLabelWidth: "40%",
    loading: false,
    map: new Map(),
    spinning: false,
    sidebarSpan: 6, // 默认侧边栏宽度 (6/24 = 25%)
    isResizing: false,
});

let notebooks_data = ref<NotebookInfo>();

// 监听内容变化，实时保存到pinia
const onContentChange = (value: string) => {
    data.editedMdtext_md = value;
    console.log("onContentChange 监听内容变化，实时保存到pinia editedMdtext_md===============", data.editedMdtext_md);
    // 实时更新pinia中的临时内容
    if (route.meta.id) {
        noteTem.setNoteData(route.meta.id as string, {
            tempContent: value
        });
    }
};

onMounted(async () => {
    //页面布局
    //dragControllerDiv();

    //关闭窗口保存
    listenClose();
    // 从本地存储恢复侧边栏宽度
    const savedSidebarWidth = localStorage.getItem('mdnote-sidebar-width');
    if (savedSidebarWidth) {
        const width = parseInt(savedSidebarWidth, 10);
        if (width >= 4 && width <= 12) {
            data.sidebarSpan = width;
        }
    }
    loadFile();
});

//读取文件
const read_note_file_bypath = async function (path: string) {

    var res = await tauri_request<ResponseData<string>>("read_note_file", {
        path: path
    });
    if (res && res.code == 200) {
        //重新赋值

        var tem = res.data;
        console.log("read_note_file_bypath res===============", res);
        // console.log("read_note_file node===============", node);
        // console.log("read_note_file  node.name===============", node.name);
        data.editedMdtext_md = tem;
        data.notEditedMdtext_md = tem;
        // data.title_md = node.name;
        // data.lastTimeTitle_md = node.name;
        // data.currentChooseNoteAllPath_md = node.path;
        // data.currentChooseFolderPath_md = node.folder;
        data.showMdEditor = true;
        console.log("read_note_file_bypath editedMdtext_md===============", data.editedMdtext_md);
        console.log("read_note_file_bypath notEditedMdtext_md===============", data.notEditedMdtext_md);
    } else {
        NotificationUtil.error("读取失败：" + res.msg);
    }

};



const fileName = ref('')
// 根据路由参数加载文件
const loadFile = async () => {
    console.log('加载文件:', route)
    let path = route.path;
    let name = route.name;
    let meta = route.meta;
    let md_id = meta.id;
    console.log("onMounted ", path, name, meta);
    console.log("onMounted meta", meta);

    // 先检查pinia中是否有缓存的内容
    let cachedNoteData: NoteDataStore = null;
    if (md_id) {
        cachedNoteData = noteTem.getNoteData(md_id as string);
    }
    if (cachedNoteData && cachedNoteData.tempContent) {
        // 如果pinia中有缓存内容，使用缓存的内容

        // 使用缓存的内容
        data.editedMdtext_md = cachedNoteData.tempContent;
        data.notEditedMdtext_md = cachedNoteData.content;
        data.title_md = cachedNoteData.title;
        data.lastTimeTitle_md = cachedNoteData.title;
        data.currentChooseNoteAllPath_md = cachedNoteData.filePath || "";
        data.currentChooseFolderPath_md = meta.folder as string;
        data.showMdEditor = true;
        console.log("从pinia缓存加载内容:", cachedNoteData);
    } else {
        // 如果没有缓存，从文件中读取
        var res = await tauri_request<ResponseData<string>>("read_note_file", {
            path: meta.filePath
        });
        if (res && res.code == 200) {
            let tem_store: NoteDataStore = {
                id: meta.id as string,
                title: meta.name as string,
                content: res.data,
                filePath: meta.filePath as string,
                tempContent: res.data,
                createdAt: new Date(),
                updatedAt: new Date(),
            }
            noteTem.setNoteData(md_id as string, tem_store)
            //重新赋值
            var tem = res.data;
            data.editedMdtext_md = tem;
            data.notEditedMdtext_md = tem;
            data.title_md = name as string;
            data.lastTimeTitle_md = name as string;
            data.currentChooseNoteAllPath_md = meta.path as string;
            data.currentChooseFolderPath_md = meta.folder as string;
            data.showMdEditor = true;
            console.log("read_note_file editedMdtext_md===============", data.editedMdtext_md);
            console.log("read_note_file notEditedMdtext_md===============", data.notEditedMdtext_md);
        } else {
            NotificationUtil.error("读取失败：" + res.msg);
        }
    }
}
// 监听路由参数变化
watch(
    () => route.path,
    () => {
        loadFile()
    },
    { immediate: true }
)
// 组件激活时的逻辑
onActivated(() => {
    console.log('NoteBookEdit 组件被激活')
    //loadFile()
})

// 组件停用时的逻辑
onDeactivated(() => {
    console.log('NoteBookEdit 组件被停用')
})
const listenClose = async () => {

    //持续监听关闭事件
    const listenClose_ = await listen('close', async (event) => {
        //关闭前保存
        if (data.currentChooseNoteAllPath_md != "" && data.editedMdtext_md != data.notEditedMdtext_md && data.title_md != "") {
            saveMd(data.editedMdtext_md, data.title_md, data.currentChooseNoteAllPath_md)
        }
        await getCurrentWindow().close();
    });
}

/**
 * 页面布局
 */
const dragControllerDiv = () => {
    const resize = document.getElementsByClassName('resize') as HTMLCollectionOf<HTMLElement>;
    const left = document.getElementsByClassName('left') as HTMLCollectionOf<HTMLElement>;
    const mid = document.getElementsByClassName('mid') as HTMLCollectionOf<HTMLElement>;
    const box = document.getElementsByClassName('box') as HTMLCollectionOf<HTMLElement>;

    for (let i = 0; i < resize.length; i++) {
        resize[i].onmousedown = function (e) {
            resize[i].style.background = '#818181';
            const startX = e.clientX;
            const originalLeft = resize[i].offsetLeft; // 使用变量保存left值

            document.onmousemove = function (e) {
                const endX = e.clientX;
                let moveLen = originalLeft + (endX - startX); // 使用变量代替resize[i].left
                const maxT = box[i].clientWidth - resize[i].offsetWidth;

                if (moveLen < 135) moveLen = 135;
                if (moveLen > maxT - 500) moveLen = maxT - 500;
                //resize[i].style.left = moveLen + 'px';
                for (let j = 0; j < left.length; j++) {
                    left[j].style.width = moveLen + 'px';
                    mid[j].style.width = (box[i].clientWidth - moveLen - 10) + 'px';
                }
            };

            document.onmouseup = function () {
                resize[i].style.background = '#d6d6d6';
                document.onmousemove = null;
                document.onmouseup = null;
                //resize[i].releaseCapture && resize[i].releaseCapture();
            };

            //resize[i].setCapture && resize[i].setCapture();
            return false;
        };
    }
}



const onError = (err: { message: string; }) => {
    NotificationUtil.error(err.message);
};



const state = reactive({
    rootSubmenuKeys: [] as string[],
    openKeys: [],
    selectedKeys: [] as string[],
    selectedNodeId: "", // 存储当前选中的节点ID
});


/**
 * 保存笔记 切换笔记但未保存时调用
 * @param {笔记内容} mdText
 * @param {标题} mdTitle
 */
const saveMd = (mdText: string, mdTitle: string, path: string,) => {
    var p = new Promise(async function () {
        if (mdTitle == "") {
            // 标题为空
            NotificationUtil.error("请输入标题");
            return;
        }

        if (mdText && mdTitle && path) {
            let res = await tauri_request<ResponseData<String[]>>("save_note", {
                path: path,
                mdTitle: mdTitle,
                mdText: mdText,
            })
            if (res && res.code == 200) {
                data.notEditedMdtext_md = mdText;
                NotificationUtil.success("笔记保存成功");
            } else {
                NotificationUtil.error("笔记保存失败：" + res.msg);
            }
        } else {
            // 存在为空情况
            NotificationUtil.error("请选定笔记本和笔记");
        }
        return p;
    });
};

/**
 * 保存文章/笔记 编辑器点击保存
 * @param {*} mdText
 */
const saveMdText = (mdText: string) => {
    if (data.title_md == "") {
        // 标题为空
        NotificationUtil.error("请输入标题");
        return;
    }
    saveMd(data.editedMdtext_md, data.title_md, data.currentChooseNoteAllPath_md)
};






/**
 * 获取文件夹下笔记列表
 */
const get_note_list = () => {
    invoke<ResponseData<string[]>>("get_note_list", {
        path: data.selectFolderPath,
    })
        .then((res: ResponseData<string[]>) => {
            if (res && res.code == 200) {
                var tem = res.data;
                data.files = tem.map((n) => {
                    return { title: n };
                });
            } else {
                NotificationUtil.error("获取笔记列表数据失败：" + res.msg);
            }
        })
        .catch((err) => {
            console.log(err);
        });
};


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
    /* 增加左侧间距 */
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

/* 在已有样式部分添加以下内容 */

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