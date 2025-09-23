<template>
    <file-info-tree :data="tree_node" @node-click="handleNodeClick" @refresh="handleRefresh" @add-file="handleAddFile"
        @add-folder="handleAddFolder" @delete-node="handleDeleteNode" @rename-node="handleRenameNode" />
</template>

<script setup lang="ts">
import { computed, h, onMounted } from 'vue'
import FileInfoTree from '@/components/FileTree/FileInfoTree.vue'
import { ref } from 'vue'
import useStore from '@/store'
import { useRouter, useRoute } from 'vue-router'
import { tauri_request } from '@/utils/tauri_request'
import NotificationUtil from '@/utils/NotificationUtil'
import { ResponseData, TreeNode } from '@/Types/entitys'
import { ElMessageBox } from 'element-plus'
import StringUtil from '@/utils/StringUtil'
import { load, Store } from '@tauri-apps/plugin-store';
import { pdf_store_tem_key_prefix, pdf_store_tem_key, pdf_store_conten_key_prefix } from '@/setting'
import { ca } from 'element-plus/es/locales.mjs'
import { ITagItem } from '@/Types/TagView'

let store: Store | undefined = undefined;
const router = useRouter()
const route = useRoute()
const { routerNoteStore, tagview, noteTem } = useStore()
const tree_node: any = computed(() =>
    routerNoteStore.tree_node
)

onMounted(async () => { 
    if (!store) {
        store = await load(pdf_store_tem_key, {
            autoSave: false,
            defaults: {}
        });
    }
})
const handleNodeClick = (node) => {
    // 打印初始路由配置
    console.log("打印初始路由配置 node", node)
    // 打印初始路由配置
    console.log("打印初始路由配置", router.options.routes)
    // 打印所有已注册的路由
    console.log("打印所有已注册的路由", router.getRoutes())
    //打印当前点击的路由
    console.log("打印当前点击的路由", node.routerPath)
    if (!node.isFolder) {
        router.push({
            path: node.routerPath
        })
    }
}

const handleRefresh = () => {
    console.log('刷新树数据')
    // 重新加载数据逻辑
    const routerNote = routerNoteStore.setRoutes();
}

const handleAddFile = async (parent: TreeNode, name) => {
    console.log(`在 ${parent.folder} 下创建文件: ${name}`)
    // 添加文件逻辑
    let newNoteMdFileName = name;
    if (
        newNoteMdFileName == null ||
        newNoteMdFileName == undefined ||
        newNoteMdFileName == ""
    ) {
        NotificationUtil.error("创建文件失败，请填写文件名称！");
        return;
    }
    // 创建笔记
    let childrenFoldersAndFiles = await tauri_request<ResponseData<string[]>>("get_path_childrenFoldersAndFiles", { path: parent.folder });
    childrenFoldersAndFiles.data.map((n) => {
        if (n == newNoteMdFileName) {
            NotificationUtil.error("已有相同文件名，请检查！");
            return;
        }
    });

    var regex = /.*[/\\:*?|].*/;
    //存在返回true，不存在返回false
    if (regex.test(newNoteMdFileName)) {
        NotificationUtil.error(
            '笔记名称命名不规范，不能包含\\ / : * ? " < > |请检查！'
        );
        return;
    }
    if (/^\d+$/.test(newNoteMdFileName)) {
        NotificationUtil.error("笔记名称命名不规范，不能为纯数字！");
        return;
    }

    let create_note_file = await tauri_request<ResponseData<string>>("create_note_file", {
        fatherPath: parent.folder,
        newNoteMdFileName: newNoteMdFileName,
    })
    if (create_note_file && create_note_file.code == 200) {

        NotificationUtil.success("笔记创建成功");

    } else {
        NotificationUtil.error("笔记创建失败：" + create_note_file.msg);
    }
}

const handleAddFolder = async (parent, name) => {
    console.log(`在 ${parent.folder} 下创建文件夹: ${name}`)
    // 添加文件夹逻辑
    let newFolderName = name;
    if (newFolderName == null || newFolderName == undefined || newFolderName == "") {
        NotificationUtil.error("创建文件夹失败，请填写文件夹名称！");
        return;
    }
    let selectFolderPath = parent.folder;
    if (selectFolderPath == null || selectFolderPath == undefined || selectFolderPath == "") {
        NotificationUtil.error("未选择文件夹！");
        return;
    }

    // 创建笔记本
    let childrenFoldersAndFiles = await tauri_request<ResponseData<string[]>>("get_path_childrenFoldersAndFiles", { path: parent.folder });
    childrenFoldersAndFiles.data.map((n) => {
        if (n == newFolderName) {
            NotificationUtil.error("已有相同文件夹名或者文件名，请检查！");

            return;
        }
    });

    var regex = /.*[/\\:*?|].*/;
    //存在返回true，不存在返回false
    if (regex.test(newFolderName)) {
        NotificationUtil.error(
            '文件夹命名不规范，不能包含\\ / : * ? " < > |请检查！'
        );
        return;
    }
    if (/^\d+$/.test(newFolderName)) {
        NotificationUtil.error("文件夹命名不规范，不能为纯数字！");
        return;
    }
    // 创建文件夹
    let create_folder_ = await tauri_request<ResponseData<string>>("create_folder", { fatherPath: parent.folder, newFolderName: newFolderName })
    if (create_folder_ && create_folder_.code == 200) {
        NotificationUtil.success("文件夹创建成功");

    } else {
        NotificationUtil.error("笔记本创建失败：" + create_folder_.msg);
    }
}

const handleDeleteNode = async (node: TreeNode) => {
    console.log(' handleDeleteNode 删除节点:', node)
    // 删除节点逻辑
    if (node.isfile) {
        ElMessageBox({
            title: "删除文件",
            message: h("div", {}, [
                h("br"),
                h("br"),
                h(
                    "p",
                    { style: "color:red;" },
                    `确定删除文件 "${node.label}" 吗？`
                ),
                h("br"),
                h("br"),
            ]),
            showCancelButton: true,
            confirmButtonText: "删除",
            cancelButtonText: "取消",
        }).then(async (action) => {
            // 点击了确定，开始删除
            let deleteFile_re = await tauri_request<ResponseData<string>>("remove_file", {
                path: node.filePath
            });
            if (deleteFile_re && deleteFile_re.code == 200) {
                NotificationUtil.success("删除文件成功！");

            } else {
                NotificationUtil.error("删除文件失败：" + deleteFile_re.msg);
            }
        }).catch((err) => {
            NotificationUtil.error("操作失败：" + err);
        });
    } else {
        /**
         *删除文件夹
         */
        if (StringUtil.isNullOrEmpty(node.folder) && node.isFolder) {
            NotificationUtil.error("请选择文件夹！");
            return;
        }
        ElMessageBox({
            title: "删除文件夹",
            message: h("div", {}, [
                h("br"),
                h("br"),
                h(
                    "p",
                    { style: "color:red;" },
                    `删除文件夹 "${node.folder}" 吗，文件夹下笔记文件也将一并删除！`
                ),
                h("br"),
                h("br"),
            ]),
            showCancelButton: true,
            confirmButtonText: "删除",
            cancelButtonText: "取消",
        }).then(async (action) => {
            let active_tagview_clear: boolean = false;
            //查找文件夹节点下所有的文件
            const list: TreeNode[] = [];
            getAllChildrenFile(node, list, active_tagview_clear);
            // 使用 Promise.all 并行处理所有文件的缓存
            const cacheResults = await Promise.all(list.map(item => processNodeCache(item)));
            let kes: string[] = await store.keys();
            console.log("processNodeCache kes:", kes);
            // 如果有任何一个文件处理失败，则返回
            if (cacheResults.some(result => !result)) {
                NotificationUtil.error("文件缓存处理失败！");
                return;
            }

            // 点击了确定，开始删除
            let remove_folder = await tauri_request<ResponseData<string>>("remove_folder", {
                path: node.folder
            });
            if (remove_folder && remove_folder.code == 200) {
                NotificationUtil.success("操作成功");
            } else {
                NotificationUtil.error("操作失败：" + remove_folder.msg);
            }
            if (active_tagview_clear) {
                activeRouteRedirect()
            }
        }).catch((err) => {
            NotificationUtil.error("操作失败：" + err);
        });
    }
    const routerNote = routerNoteStore.setRoutes();
}

const handleRenameNode = async (node: TreeNode, newName) => {
    console.log(`重命名节点 ${node} 为 ${newName}`)
    //判断条件，检查 store 实例是否存在
    if (!store) {
        store = await load(pdf_store_tem_key, {
            autoSave: false,
            defaults: {}
        });
    }
    //当前活动的路由
    let active_path = route.path;
    //是否清空到活动路由
    let active_tagview_clear: boolean = false;
    console.log("handleRenameNode active_path:", active_path);


    // 重命名逻辑
    if (node.isfile) {
        // 处理文件缓存
        const cacheProcessed = await processNodeCache(node);
        if (!cacheProcessed) return;

        const upFileName_re = await tauri_request<ResponseData<string>>("up_file_name", {
            path: node.filePath,
            upFileName: newName
        });

        if (upFileName_re && upFileName_re.code == 200) {
            NotificationUtil.success("修改文件名成功！");
        } else {
            NotificationUtil.error("修改文件名失败：" + (upFileName_re?.msg || "未知错误"));
        }
    } else if (node.isFolder) {
        //查找文件夹节点下所有的文件
        const list: TreeNode[] = [];
        getAllChildrenFile(node, list, active_tagview_clear);
        console.log("processNodeCache 用 Promise.all 并行处理所有文件的缓存:");
        // 使用 Promise.all 并行处理所有文件的缓存
        const cacheResults = await Promise.all(list.map(item => processNodeCache(item)));
        let kes: string[] = await store.keys();
        console.log("processNodeCache kes:", kes);
        // 如果有任何一个文件处理失败，则返回
        if (cacheResults.some(result => !result)) {
            NotificationUtil.error("文件缓存处理失败！");
            return;
        }
        console.log("processNodeCache 重命名文件夹:");
        //重命名文件夹
        let upFolderName_re = await tauri_request<ResponseData<string>>("up_folder_name", {
            path: node.folder,
            upFolderName: newName
        });
        console.log("upFolderName_re  重命名文件夹:", upFolderName_re);
        if (upFolderName_re && upFolderName_re.code == 200) {
            NotificationUtil.success("修改文件夹名成功！");
        } else {
            NotificationUtil.error("修改文件夹名失败：" + (upFolderName_re?.msg || "未知错误"));
        }
    }
    const routerNote = routerNoteStore.setRoutes();
    if (active_tagview_clear) {
        activeRouteRedirect()
    }

}
const activeRouteRedirect =  async() => {
    const endRoute = tagview.tagViewList[tagview.tagViewList.length - 1] as ITagItem
    if (endRoute) {
        router.push({
            path: endRoute.path
        })
    } else {
        router.push({
            path: '/'
        })
    }
}
const getAllChildrenFile = (node: TreeNode, list: TreeNode[], active_tagview_clear: boolean) => {
    let active_path = route.path;
    if (active_path == node.routerPath && node.isfile) {
        active_tagview_clear = true;
    }
    if (node.isfile) {
        list.push(node);
    } else {
        node.children.forEach(async (child) => {
            getAllChildrenFile(child, list, active_tagview_clear);
        });
    }
}
// 提取公共的缓存处理函数
const processNodeCache = async (item: TreeNode) => {
    try {
        let cached_content: string | null = null;
        let content: string | null = null;
        const key_tem = pdf_store_tem_key_prefix + item.id;
        const key_conten = pdf_store_conten_key_prefix + item.id;
        console.log("processNodeCache key_tem key_conten:", key_tem, key_conten);
        const tem_val = await store.get<{ value: string }>(key_tem);
        console.log("processNodeCache tem_val:", tem_val);
        if (tem_val != undefined && tem_val.value != null && tem_val.value != "") {
            cached_content = tem_val.value;
        } else {
            console.log("processNodeCache 无需处理:", key_tem, key_conten);
            return true
        }

        const content_val = await store.get<{ value: string }>(key_conten);
        if (content_val !== undefined) {
            content = content_val.value;
        }

        // 简化条件判断
        console.log("processNodeCache 简化条件判断:", cached_content, content);
        if (cached_content && content != cached_content) {
            console.log("processNodeCache 简化条件判断:",);
            //保存文件
            const save_file_re = await tauri_request<ResponseData<String[]>>("save_note", {
                path: item.filePath, // 修复：使用正确的路径
                mdText: cached_content,
            })
            if (save_file_re && save_file_re.code != 200) {
                NotificationUtil.error("文件保存失败：" + save_file_re.msg + "请检查！");
                return false;
            }
            // 删除缓存


        }
        let delete_tem: boolean = await store.delete(key_tem);
        let delete_conten: boolean = await store.delete(key_conten);
        console.log("processNodeCache 删除缓存 删除结果:", delete_tem, delete_conten);
        if (!delete_tem || !delete_conten) {
            NotificationUtil.error("缓存删除失败！");
        }
        tagview.removeTagViewByPath(item.routerPath);
        return true;
    } catch (error) {
        NotificationUtil.error("操作失败：" + error);
        return false;
    }
};
</script>