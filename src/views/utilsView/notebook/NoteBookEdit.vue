<template>
  <div class="box" ref="box">
    <div class="left">
      <div class="sidebar-content">
        <div class="sidebar-header">
          <h3 class="sidebar-title">文件浏览器</h3>
          <div class="sidebar-controls">
            <el-button :icon="Refresh" circle size="small" @click="reFresh" title="刷新" />
          </div>
        </div>
        <div class="wrapper" v-if="show_notebooks_data" v-contextmenu:contextmenu>
          <el-menu default-active="2" class="folder-menu">
            <TreeNode v-if="notebooks_data" :node="notebooks_data" v-model="state.selectedNodeId"
              @node-click="selectHandle" />
          </el-menu>
        </div>
        <div class="empty-state" v-else>
          <el-empty description="暂无数据" :image-size="80">
            <el-button type="primary" @click="showAddFolderModalForm">创建文件夹</el-button>
          </el-empty>
        </div>
      </div>
    </div>
    <div class="resize" title="收缩侧边栏">⋮</div>
    <div class="mid">
      <div v-if="data.showMdEditor" class="editor-wrapper">
        <!-- <a-form layout="inline" class="editor-header">
          <a-form-item class="title-form-item">
            <a-input class="mdtitle-input" :bordered="false" v-model:value="data.title_md" placeholder="文章标题"
              size="large" />
          </a-form-item>
        </a-form> -->
        <MdEditor v-model="data.editedMdtext_md" @onSave="saveMdText" class="md-editor" @onError="onError" />
      </div>
      <div v-else class="editor-placeholder">
        <div class="placeholder-content">
          <el-icon :size="48" color="#909399">
            <Document />
          </el-icon>
          <p class="placeholder-text">选择或创建一个笔记开始编辑</p>
          <el-button v-if="notebooks_data && notebooks_data.children && notebooks_data.children.length > 0"
            type="primary" @click="showAddFileModalForm" class="create-note-button">
            创建新笔记
          </el-button>
        </div>
      </div>
    </div>
    <v-contextmenu ref="contextmenu" class="context-menu" v-if="data_not_null">
      <v-contextmenu-item @click="showAddFileModalForm">
        <el-icon>
          <DocumentAdd />
        </el-icon>
        添加文件
      </v-contextmenu-item>
      <v-contextmenu-divider />
      <v-contextmenu-item @click="showAddFolderModalForm">
        <el-icon>
          <FolderAdd />
        </el-icon>
        添加文件夹
      </v-contextmenu-item>
      <v-contextmenu-divider />
      <v-contextmenu-item @click="reFresh">
        <el-icon>
          <Refresh />
        </el-icon>
        刷新
      </v-contextmenu-item>
      <v-contextmenu-divider />
      <v-contextmenu-item v-show="data.selectIsFile" @click="removeNote">
        <el-icon>
          <Delete />
        </el-icon>
        删除文件
      </v-contextmenu-item>
      <v-contextmenu-divider v-show="data.selectIsFile" />
      <v-contextmenu-item v-show="data.selectIsFile" @click="removeFile">
        <el-icon>
          <Edit />
        </el-icon>
        修改文件名称
      </v-contextmenu-item>
      <v-contextmenu-divider v-show="!(data.selectIsFile)" />
      <v-contextmenu-item v-show="!(data.selectIsFile)" @click="removeFolder">
        <el-icon>
          <Delete />
        </el-icon>
        删除文件夹
      </v-contextmenu-item>
      <v-contextmenu-divider />
      <v-contextmenu-item @click="fileManagerOpenPath">
        <el-icon>
          <FolderOpened />
        </el-icon>
        在文件资源管理器中显示
      </v-contextmenu-item>
    </v-contextmenu>
    <v-contextmenu ref="contextmenu" class="context-menu" v-else>
      <v-contextmenu-item @click="showAddFolderModalForm">
        <el-icon>
          <FolderAdd />
        </el-icon>
        添加文件夹
      </v-contextmenu-item>
      <v-contextmenu-divider />
      <v-contextmenu-item @click="reFresh">
        <el-icon>
          <Refresh />
        </el-icon>
        刷新
      </v-contextmenu-item>
    </v-contextmenu>

    <!-- 添加文件夹表单 -->
    <el-dialog v-model="data.addFolderModalFormVisible" :before-close="handleCancelByAddFolderModal" title="添加文件夹"
      width="500" class="custom-dialog">
      <el-form>
        <el-form-item label="文件夹名称" :label-width="data.formLabelWidth">
          <el-input v-model="data.newFolderName" autocomplete="off" placeholder="请输入文件夹名称"
            @keyup.enter="addNodeFolder" />
        </el-form-item>
      </el-form>
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="handleCancelByAddFolderModal">取消</el-button>
          <el-button type="primary" @click="addNodeFolder"> 确认 </el-button>
        </div>
      </template>
    </el-dialog>

    <!-- 添加笔记本弹出表单 -->
    <el-dialog v-model="data.addFileModalFormVisible" :before-close="handleCancelByAddNotebookModal" title="添加文件"
      width="500" class="custom-dialog">
      <el-form>
        <el-form-item label="文件名称" :label-width="data.formLabelWidth">
          <el-input v-model="data.newFileName" autocomplete="off" placeholder="请输入文件名称" @keyup.enter="addFile" />
        </el-form-item>
      </el-form>
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="handleCancelByAddNotebookModal">取消</el-button>
          <el-button type="primary" @click="addFile"> 确认 </el-button>
        </div>
      </template>
    </el-dialog>

    <!-- 修改文件名称弹出表单 -->
    <el-dialog v-model="data.upFileNameModalFormVisible" :before-close="handleCancelByUpFileNameModal" title="修改文件名"
      width="500" class="custom-dialog">
      <el-form>
        <el-form-item label="文件名称" :label-width="data.formLabelWidth">
          <el-input v-model="data.upFileName" autocomplete="off" placeholder="请输入新的文件名称" @keyup.enter="upFileName" />
        </el-form-item>
      </el-form>
      <template #footer>
        <div class="dialog-footer">
          <el-button @click="handleCancelByUpFileNameModal">取消</el-button>
          <el-button type="primary" @click="upFileName">修改</el-button>
        </div>
      </template>
    </el-dialog>
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
} from "vue";
//import { notification, Modal } from 'ant-design-vue';
import { ElMessage, ElMessageBox, ElSubMenu, ElMenuItem } from "element-plus";
import DirFileMenu from "@/utils/DirFileMenu";
import StringUtil from "@/utils/StringUtil";
import NotificationUtil from "@/utils/NotificationUtil";
import { invoke } from "@tauri-apps/api/core";
import { openPath } from '@tauri-apps/plugin-opener';
import { MdEditor } from "md-editor-v3";
import "md-editor-v3/lib/style.css";
import { NotebookInfo, ResponseData } from "@/entity/entitys";
import TreeNode from "@/components/tsx/menu/TreeNode";
import { tauri_request } from "@/utils/request";
import {
  Document,
  Refresh,
  DocumentAdd,
  FolderAdd,
  Delete,
  Edit,
  FolderOpened
} from '@element-plus/icons-vue';
// 处理节点点击事件
const handleNodeClick = (node: NotebookInfo) => {
  console.log("节点被点击:", node);
  console.log("节点被点击 electedNodeId:", state.selectedNodeId);
  // 这里可以添加自定义逻辑，比如：
  // - 加载节点内容
  // - 更新UI状态
  // - 执行特定操作
};
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

        if (moveLen < 32) moveLen = 32;
        if (moveLen > maxT - 150) moveLen = maxT - 150;

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

onMounted(() => {
  dragControllerDiv();
  get_document_notebooks();
  // 从本地存储恢复侧边栏宽度
  const savedSidebarWidth = localStorage.getItem('mdnote-sidebar-width');
  if (savedSidebarWidth) {
    const width = parseInt(savedSidebarWidth, 10);
    if (width >= 4 && width <= 12) {
      data.sidebarSpan = width;
    }
  }
});

const data = reactive({
  folders: [], // 文件夹列表
  files: [] as { title: string }[], // 文件列表
  selectedKeys: [], // 笔记本选中项key
  selectFolderPath: "", // 当前选中文件夹 用于建文件和子文件夹
  selectIsFile: false,// 当前选中文件还是文件夹
  currentChooseFolderPath_md: "", // 当前选中文件的文件夹
  //lastTimeChooseFolderPath_md: "", //上次选中文件的文件夹
  currentChooseNoteAllPath_md: "", // 当前选中笔记 全路径
  //lastTimeChooseNoteAllPath_md: "", // 上次选中笔记 全路径
  notEditedMdtext_md: "", // 未编辑的文件内容
  editedMdtext_md: "", // 文章内容Markdown格式
  title_md: "", // 文章标题
  lastTimeTitle_md: "", //旧标题
  showMdEditor: false, // 是否显示编辑器
  newFolderName: "", // 新建文件夹夹的名称
  newFileName: "", // 新建的笔记文件名
  upFileName: "",
  addFileModalFormVisible: false, // 是否显示添加笔记本表单
  addFolderModalFormVisible: false,
  upFileNameModalFormVisible: false,
  confirmLoading: false, // 添加笔记表单等待状态（是否转圈圈）
  formLabelWidth: "40%",
  loading: false,
  map: new Map(),
  spinning: false,
  sidebarSpan: 6, // 默认侧边栏宽度 (6/24 = 25%)
  isResizing: false,
});

let notebooks_data = ref<NotebookInfo>();
const sidebarContainer = ref<HTMLElement | null>(null);

/**
 *  获取笔记本数据
 */
let get_document_notebooks = () => {
  // 获取笔记本数据
  invoke<ResponseData<NotebookInfo>>("get_document_notebooks")
    .then((res: ResponseData<NotebookInfo>) => {
      console.log("res===============", res);
      if (res && typeof res === "object" && "code" in res && res.code == 200) {
        //值 返回ResponseData 对象，包含data、code、msg参数 例如：data {"id":"1752761254929","parent_id":"","path":"C:\\Users\\kun15\\Documents\\notebooks","name":"notebooks","isfile":false,"children":[]}
        let data_: NotebookInfo =
          typeof res.data === "string" ? JSON.parse(res.data) : res.data;
        notebooks_data.value = data_;
        state.rootSubmenuKeys = DirFileMenu.getRootSubmenuKeys(
          notebooks_data.value.children
        );
      }
    })
    .catch((err) => {
      NotificationUtil.error("获取数据失败 ：" + err);
    });
};
const onError = (err: { message: string; }) => {
  NotificationUtil.error(err.message);
};
/**
 * 文件夹及文件夹下笔记信息
 */
let show_notebooks_data = computed(() => {
  return notebooks_data.value != null && notebooks_data.value != undefined;
});

/**
 * 判断数是否为空
 */
let data_not_null = computed(() => {
  return notebooks_data.value != null && notebooks_data.value != undefined && Array.isArray(notebooks_data.value.children) && notebooks_data.value.children.length > 0;
});

const state = reactive({
  rootSubmenuKeys: [] as string[],
  openKeys: [],
  selectedKeys: [] as string[],
  selectedNodeId: "", // 存储当前选中的节点ID
});
/**
 * SubMenu 展开/关闭的回调
 * @param {*当前展开的 SubMenu 菜单项 key 数组} openKeys
 */
// const onOpenChange = function (openKeys) {
//   const latestOpenKey = openKeys.find(
//     (key) => state.openKeys.indexOf(key) === -1
//   );
//   if (state.rootSubmenuKeys.indexOf(latestOpenKey) === -1) {
//     state.openKeys = openKeys;
//   } else {
//     state.openKeys = latestOpenKey ? [latestOpenKey] : [];
//   }
//   data.currentNote = latestOpenKey;
// };

/**
 * 被选中时调用
 * selectedKeys(v-model)	当前选中的菜单项 key 数组
 * @param {*} param0
 */
const selectHandle = function (node: NotebookInfo) {
  console.log("node {:?}", node)
  data.selectIsFile = node.isfile;
  if (!node.isfile) {
    data.selectFolderPath = node.folder;
    console.log("data.selectFolderPath {:?}", data.selectFolderPath)
    return;
  }
  //重复点击自己 路径相同
  if (node.path == data.currentChooseFolderPath_md) {
    return;
  }

  // 
  //处理原来编辑笔记
  if ((data.notEditedMdtext_md != data.editedMdtext_md) || (data.title_md != data.lastTimeTitle_md &&
    StringUtil.isNotNullOrEmpty(data.title_md) &&
    StringUtil.isNotNullOrEmpty(data.lastTimeTitle_md))
  ) {
    ElMessageBox.confirm("上一份笔记未保存，需要保存吗?", "保存笔记", {
      confirmButtonText: "保存",
      cancelButtonText: "不保存",
      type: "warning",
    })
      .then(() => {
        //获取目录下笔记
        invoke<ResponseData<String[]>>("get_note_list", {
          path: data.currentChooseFolderPath_md,
        })
          .then((res: ResponseData<String[]>) => {
            //获取旧笔记
            let same_file_name = false;
            if (res && res.code == 200) {
              //String 数组集合 所属文件夹下全部文件名 查看是否有重复
              var tem = res.data;
              tem.map((n) => {
                if (n == data.title_md && data.title_md != data.lastTimeTitle_md) {
                  same_file_name = true;
                }
              });
              //无重复
              if (!same_file_name) {
                //先保存数据
                Promise.all([saveMd(data.editedMdtext_md, data.title_md, data.currentChooseNoteAllPath_md)]).then(() => {
                  read_note_file(node);
                })
                  .catch((err) => {
                    NotificationUtil.error("保存文件失败，" + err.getMessage());
                  });
              } else {
                //有重复名称
                NotificationUtil.warn("已有相同文件名，文件名将无法更改！");
                //保存旧文件名称
                Promise.all([saveMd(data.editedMdtext_md, data.title_md, data.currentChooseNoteAllPath_md)]).then(() => {
                  read_note_file(node);
                }).catch((err) => {
                  NotificationUtil.error("保存文件失败，" + err.getMessage());
                });
              }
            }
          })
          .catch((err) => {
            NotificationUtil.error("获取数据失败 ：" + err);
          });
      })
      .catch(() => {
        read_note_file(node);
      });
  } else {
    read_note_file(node);
  }
};

/**
 * 保存笔记 切换笔记但未保存时调用
 * @param {笔记内容} mdText
 * @param {标题} mdTitle
 */
const saveMd = (mdText: string, mdTitle: string, path: string,) => {
  var p = new Promise(function () {
    if (mdTitle == "") {
      // 标题为空
      NotificationUtil.error("请输入标题");
      return;
    }

    if (mdText && mdTitle && path) {
      // 调用保存笔记方法
      invoke<ResponseData<string>>("save_note", {
        path: path,
        mdTitle: mdTitle,
        mdText: mdText,
      })
        .then((res: ResponseData<string>) => {
          if (res && res.code == 200) {
            NotificationUtil.success("笔记保存成功");
          } else {
            NotificationUtil.error("笔记保存失败：" + res.msg);
          }
        })
        .then((err) => {
          console.log(err);
        });
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
  invoke<ResponseData<string[]>>("get_note_list", {
    notebook: data.currentChooseFolderPath_md,
  })
    .then((res: ResponseData<string[]>) => {
      let same_file_name = false;
      if (res && res.code == 200) {
        var tem = res.data;
        tem.map((n) => {
          if (n == data.title_md) {
            same_file_name = true;
          }
        });
        if (!same_file_name) {
          if (data.currentChooseNoteAllPath_md) {
            // 不为空
            // 调用保存笔记方法
            invoke<ResponseData<string>>("save_note", {
              path: data.currentChooseNoteAllPath_md,
              mdTitle: data.title_md,
              mdText: mdText,
            })
              .then((res: ResponseData<string>) => {
                if (res && res.code == 200) {
                  NotificationUtil.success("笔记保存成功");
                } else {
                  NotificationUtil.error("笔记保存失败：" + res.msg);
                }
              })
              .then((err) => {
                console.log(err);
              });
          } else {
            // 存在为空情况
            NotificationUtil.error("请选定笔记本和笔记");
          }
        } else {
          NotificationUtil.warn("已有相同文件名，文件名将无法更改！");
          if (data.currentChooseNoteAllPath_md) {
            // 调用保存笔记方法 保存新文件
            invoke<ResponseData<string>>("save_note", {
              currentNote: data.currentChooseNoteAllPath_md,
              mdTitle: data.title_md,
              mdText: mdText,
            }).then((res: ResponseData<string>) => {
              if (res && res.code == 200) {
                data.lastTimeTitle_md = data.title_md;
                data.notEditedMdtext_md = mdText;
              } else {
                NotificationUtil.error("笔记保存失败：" + res.msg);
              }
            });
          } else {
            // 存在为空情况
            NotificationUtil.error("请选定笔记本和笔记");
          }
        }
      } else {
        NotificationUtil.error("获取笔记列表数据失败：" + res.msg);
      }
    })
    .catch((err) => {
      console.log(err);
      NotificationUtil.error("获取文件信息异常！");
      return;
    });
};

/**
 * 显示添加文件夹对话框
 */
const showAddFolderModalForm = () => {
  data.addFolderModalFormVisible = true;
};
/**
 * 添加文件夹
 */
let addNodeFolder = async function () {
  let newFolderName = data.newFolderName.trim();
  if (newFolderName == null || newFolderName == undefined || newFolderName == "") {
    NotificationUtil.error("创建文件夹失败，请填写文件夹名称！");
    return;
  }
  let selectFolderPath = data.selectFolderPath.trim();
  if (selectFolderPath == null || selectFolderPath == undefined || selectFolderPath == "") {
    NotificationUtil.error("未选择文件夹！");
    return;
  }

  // 创建笔记本
  let childrenFoldersAndFiles = await tauri_request<ResponseData<string[]>>("get_path_childrenFoldersAndFiles", { path: data.selectFolderPath });
  childrenFoldersAndFiles.data.map((n) => {
    if (n == newFolderName) {
      NotificationUtil.error("已有相同文件夹名或者文件名，请检查！");
      data.newFolderName = "";
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
  data.confirmLoading = true;
  data.loading = true;
  // 创建文件夹
  let create_folder_ = await tauri_request<ResponseData<string>>("create_folder", { fatherPath: data.selectFolderPath, newFolderName: newFolderName })
  if (create_folder_ && create_folder_.code == 200) {
    NotificationUtil.success("文件夹创建成功");
    data.addFolderModalFormVisible = false;
    data.newFolderName = "";
    data.loading = false;
    get_document_notebooks();
  } else {
    NotificationUtil.error("笔记本创建失败：" + create_folder_.msg);
  }
};
// 显示添加笔记对话框
const showAddFileModalForm = () => {
  console.log("showAddFileModalForm ", data.selectFolderPath);
  if (StringUtil.isNotNullOrEmpty(data.selectFolderPath)) {
    NotificationUtil.error(`请选择文件夹`);
    return;
  }
  data.addFileModalFormVisible = true;
};
/**
 * 添加笔记
 */
let addFile = async function () {
  let newNoteMdFileName = data.newFileName.trim();
  if (
    newNoteMdFileName == null ||
    newNoteMdFileName == undefined ||
    newNoteMdFileName == ""
  ) {
    NotificationUtil.error("创建文件失败，请填写文件名称！");
    return;
  }
  // 创建笔记
  let childrenFoldersAndFiles = await tauri_request<ResponseData<string[]>>("get_path_childrenFoldersAndFiles", { path: data.selectFolderPath });
  childrenFoldersAndFiles.data.map((n) => {
    if (n == newNoteMdFileName) {
      NotificationUtil.error("已有相同文件名，请检查！");
      data.newFolderName = "";
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
    fatherPath: data.selectFolderPath,
    newNoteMdFileName: newNoteMdFileName,
  })
  if (create_note_file && create_note_file.code == 200) {
    data.addFileModalFormVisible = false; // 隐藏模态表单
    NotificationUtil.success("笔记创建成功");
    get_document_notebooks();
  } else {
    NotificationUtil.error("笔记创建失败：" + create_note_file.msg);
  }
};

let upFileNameMenu = async function () {
  if (StringUtil.isNotNullOrEmpty(data.currentChooseNoteAllPath_md) && data.selectIsFile) {
    NotificationUtil.warn("请选择笔记");
    return;
  }
  data.upFileNameModalFormVisible = true;
}
let handleCancelByUpFileNameModal = async function () {
  data.upFileNameModalFormVisible = false;
  data.upFileName = "";
}
let upFileName = async function () {
  let upFileName_re = await tauri_request<ResponseData<string>>("up_file_name", {
    path: data.currentChooseNoteAllPath_md,
    upFileName: data.upFileName
  });
  if (upFileName_re && upFileName_re.code == 200) {
    data.upFileNameModalFormVisible = false; // 隐藏模态表单
    NotificationUtil.success("修改文件名成功！");
    get_document_notebooks();
  } else {
    NotificationUtil.error("修改文件名失败：" + upFileName_re.msg);
  }
}
/**
 * 删文件
 */
let removeNote = async function () {
  if (StringUtil.isNotNullOrEmpty(data.currentChooseNoteAllPath_md) && data.selectIsFile) {
    NotificationUtil.warn("请选择笔记");
    return;
  }
  ElMessageBox({
    title: "删除笔记",
    confirmButtonText: "OK",
    cancelButtonText: "Cancel",
    type: "warning",
    message: h("p", null, [
      h("span", null, `确认删除笔记 ${data.title_md} 吗？ `),
    ]),
  })
    .then(async () => {
      //查看删除笔记是否是正在编辑的笔记
      //点击了确定，开始删除

      let remove_note = await tauri_request<ResponseData<string>>("remove_file", {
        path: data.currentChooseNoteAllPath_md
      });
      if (remove_note && remove_note.code == 200) {
        NotificationUtil.success("操作成功");
        data.currentChooseNoteAllPath_md = "";
        data.currentChooseFolderPath_md = "";
        data.showMdEditor = false;
        data.title_md = "";
        data.editedMdtext_md = "";
        data.lastTimeTitle_md = "";
        data.notEditedMdtext_md = "";
        get_document_notebooks();
      } else {
        NotificationUtil.error("操作失败：" + remove_note.msg);
      }
    })
    .catch((e) => { NotificationUtil.error("操作失败：" + e.msg); });
};

let removeFile = async function () {
  if (StringUtil.isNotNullOrEmpty(data.currentChooseNoteAllPath_md) && data.selectIsFile) {
    NotificationUtil.error("请选择文件！");
    return;
  }
  ElMessageBox({
    title: "删除文件",
    message: h("div", {}, [
      h("br"),
      h("br"),
      h(
        "p",
        { style: "color:red;" },
        `删除文件 "${data.title_md}" 吗?`
      ),
      h("br"),
      h("br"),
    ]),
    showCancelButton: true,
    confirmButtonText: "删除",
    cancelButtonText: "取消",
  }).then(async (action) => {
    // 点击了确定，开始删除
    let remove_file = await tauri_request<ResponseData<string>>("remove_file", {
      path: data.currentChooseNoteAllPath_md
    });
    if (remove_file && remove_file.code == 200) {
      NotificationUtil.success("操作成功");
      data.currentChooseNoteAllPath_md = "";
      data.editedMdtext_md = "";
      data.showMdEditor = false;
      data.title_md = "";
      data.selectIsFile = false;
      get_document_notebooks();
    } else {
      NotificationUtil.error("操作失败：" + remove_file.msg);
    }

  }).catch((err) => {
    NotificationUtil.error("操作失败：" + err);
  });
}
/**
 * 删除文件夹
 */
let removeFolder = async function () {
  if (StringUtil.isNotNullOrEmpty(data.selectFolderPath) && !data.selectIsFile) {
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
        `删除文件夹 "${data.selectFolderPath}" 吗，文件夹下笔记文件也将一并删除！`
      ),
      h("br"),
      h("br"),
    ]),
    showCancelButton: true,
    confirmButtonText: "删除",
    cancelButtonText: "取消",
  }).then(async (action) => {
    // 点击了确定，开始删除
    let remove_folder = await tauri_request<ResponseData<string>>("remove_folder", {
      path: data.selectFolderPath
    });
    if (remove_folder && remove_folder.code == 200) {
      NotificationUtil.success("操作成功");
      get_document_notebooks();
    } else {
      NotificationUtil.error("操作失败：" + remove_folder.msg);
    }

  }).catch((err) => {
    NotificationUtil.error("操作失败：" + err);
  });
};
let fileManagerOpenPath = async function () {
  //await openPath(data.selectFolderPath);

  let res = await tauri_request<ResponseData<string>>("file_manager_open_path", { path: data.selectFolderPath });
  if (res && res.code != 200) {
    NotificationUtil.error(res.msg);
  }
}

/**
 * 关闭添加文件夹模态框
 */
const handleCancelByAddFolderModal = () => {
  data.addFolderModalFormVisible = false;
  data.newFolderName = "";
};
/**
 * 关闭添加笔记模态框
 */
const handleCancelByAddNotebookModal = () => {
  data.addFileModalFormVisible = false;
  data.newFileName = "";
};
/**
 * 关闭修改文件名模态框
 */
const handleCancelByUpFileModal = () => {
  data.upFileNameModalFormVisible = false;
  data.newFileName = "";
};
/**
 * 刷新笔记信息
 */
const reFresh = async () => {
  let res = await tauri_request<ResponseData<NotebookInfo>>("get_document_notebooks", {});
  console.log("res===============", res);
  if (res && typeof res === "object" && "code" in res && res.code == 200) {
    //值 返回ResponseData 对象，包含data、code、msg参数 例如：data {"id":"1752761254929","parent_id":"","path":"C:\\Users\\kun15\\Documents\\notebooks","name":"notebooks","isfile":false,"children":[]}
    let data_: NotebookInfo =
      typeof res.data === "string" ? JSON.parse(res.data) : res.data;
    notebooks_data.value = data_;
    state.rootSubmenuKeys = DirFileMenu.getRootSubmenuKeys(
      notebooks_data.value.children
    );
    NotificationUtil.success("刷新完成");
  }

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
//读取文件
const read_note_file = async function (node: NotebookInfo) {
  if (node.isfile) {
    var res = await tauri_request<ResponseData<string>>("read_note_file", {
      path: node.path
    });
    if (res && res.code == 200) {
      //重新赋值
      var tem = res.data;
      data.editedMdtext_md = tem;
      data.notEditedMdtext_md = tem;
      data.title_md = node.name;
      data.lastTimeTitle_md = node.name;
      data.currentChooseNoteAllPath_md = node.path;
      data.currentChooseFolderPath_md = node.folder;
      data.showMdEditor = true;
    } else {
      NotificationUtil.error("读取失败：" + res.msg);
    }
  } else {
    NotificationUtil.error("读取失败,不是文件!");
  }
};

// 调整侧边栏宽度功能
const startResize = (e: MouseEvent) => {
  data.isResizing = true;
  const startX = e.clientX;
  const startWidth = data.sidebarSpan;

  const doResize = (moveEvent: MouseEvent) => {
    if (!data.isResizing) return;

    const diffX = moveEvent.clientX - startX;
    const containerWidth = (sidebarContainer.value?.parentElement?.offsetWidth || window.innerWidth);
    const percentChange = (diffX / containerWidth) * 24;

    let newSpan = startWidth + percentChange;
    newSpan = Math.max(4, Math.min(12, newSpan)); // 限制在4-12之间 (1/6到1/2的宽度)
    data.sidebarSpan = Math.round(newSpan);
  };
  const stopResize = () => {
    data.isResizing = false;
    document.removeEventListener('mousemove', doResize);
    document.removeEventListener('mouseup', stopResize);

    // 保存宽度设置到本地存储
    localStorage.setItem('mdnote-sidebar-width', data.sidebarSpan.toString());
  };

  document.addEventListener('mousemove', doResize);
  document.addEventListener('mouseup', stopResize);
};
</script>

<style scoped>
.notebook-container {
  height: 100%;
  min-height: 500px;
  display: flex;
  position: relative;
  width: 100%;
}

.sidebar-container {
  height: 100%;
  background-color: #f5f5f5;
  border-right: 1px solid #dcdfe6;
  display: flex;
  flex-direction: column;
  min-width: 200px;
  position: relative;
}

.sidebar-content {
  height: 100%;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.sidebar-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 15px;
  border-bottom: 1px solid #e4e7ed;
  background-color: #fff;
}

.sidebar-title {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.sidebar-controls {
  display: flex;
  gap: 5px;
}

.folder-menu {
  border: none;
  flex: 1;
  overflow-y: auto;
  padding: 5px 0;
}

.wrapper {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.empty-state {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: #fafafa;
}

.editor-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  background-color: #ffffff;
}

.editor-wrapper {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.editor-header {
  padding: 10px 15px;
  border-bottom: 1px solid #ebeef5;
  background-color: #f8f9fa;
}

.title-form-item {
  width: 100%;
}

.mdtitle-input {
  font-size: 18px;
  font-weight: 500;
  padding: 8px 0;
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
  background-color: #fafafa;
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

.context-menu {
  border-radius: 4px;
  box-shadow: 0 2px 12px 0 rgba(0, 0, 0, 0.1);
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

/* 调整手柄 */
.resize-handle {
  width: 5px;
  height: 100%;
  background-color: #dcdfe6;
  cursor: col-resize;
  position: relative;
  transition: background-color 0.2s;
  z-index: 10;
}

.resize-handle:hover {
  background-color: #409eff;
}

/* 响应式优化 */
@media (max-width: 768px) {
  .notebook-container {
    flex-direction: column;
  }

  .sidebar-container,
  .editor-container {
    width: 100%;
  }

  .sidebar-container {
    height: 40%;
    border-right: none;
    border-bottom: 1px solid #dcdfe6;
  }

  .editor-container {
    height: 60%;
  }

  .resize-handle {
    display: none;
  }
}


.box {
  width: 100%;
  height: 100%;
  margin: 0 0;
  overflow: hidden;
  box-shadow: -1px 9px 10px 3px rgba(0, 0, 0, 0.11);
}

.left {
  width: calc(20% - 10px);
  height: 100%;
  background: #c01a1a;
  float: left;
}

.resize {
  cursor: col-resize;
  float: left;
  position: relative;
  top: 45%;
  background-color: #d6d6d6;
  border-radius: 5px;
  margin-top: -10px;
  width: 10px;
  height: 50px;
  background-size: cover;
  background-position: center;
  font-size: 32px;
  color: white;
}

.resize:hover {
  color: #444444;
}

.mid {
  float: left;
  width: 80%;
  height: 100%;
  background: #1812c8;
  box-shadow: -1px 4px 5px 3px rgba(0, 0, 0, 0.11);
}
</style>