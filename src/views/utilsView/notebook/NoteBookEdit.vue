<template>
  <el-row style="height: 100%">
    <el-col :span="5" style="height: 100%; background-color: #d9d9d9" v-contextmenu:contextmenu>
      <!-- <a-spin :spinning="spinning"> </a-spin> -->
      <div class="wrapper" v-if="show_notebooks_data.length > 0">
        <!-- <a-menu v-model:selectedKeys="state.selectedKeys" mode="inline" :open-keys="state.openKeys"
          @select="selectHandle" :items="show_notebooks_data" @openChange="onOpenChange"
          style="max-height: 100%; overflow: hidden auto"></a-menu> -->

        <el-menu default-active="2" class="el-menu-vertical-demo">
          <TreeNode v-if="notebooks_data" :node="notebooks_data" v-model="state.selectedNodeId"
            @node-click="selectHandle" />
        </el-menu>
      </div>
      <div style="height: 100%; position: relative" v-else>
        <span style="
            position: absolute;
            top: 50%;
            left: 50%;
            transform: translateX(-50%) translateY(-50%);
            color: rgb(165 164 164);
          ">
          右击添加文件夹
        </span>
      </div>
    </el-col>
    <!-- 编辑器栏 -->
    <el-col :span="19">
      <!-- <div v-if="showMdEditor"> -->
      <a-form layout="inline">
        <a-form-item style="width: 99.5%; margin: 0 auto; padding: 1px 0">
          <a-input class="mdtitle-input" :bordered="false" v-model:value="data.mdTitle" placeholder="文章标题"></a-input>
        </a-form-item>
      </a-form>
      <MdEditor v-model="data.mdText" @onSave="saveMdText" style="height: 100%" @onError="onError" />
      <!-- </div> -->
    </el-col>
  </el-row>
  <v-contextmenu ref="contextmenu" v-if="data_not_null">
    <v-contextmenu-item @click="showAddNotebookModalForm">
      添加笔记
    </v-contextmenu-item>
    <v-contextmenu-divider />
    <v-contextmenu-item @click="showAddFolderModalForm">
      添加文件夹
    </v-contextmenu-item>
    <v-contextmenu-divider />
    <v-contextmenu-item @click="reFresh">刷新</v-contextmenu-item>
    <v-contextmenu-divider />
    <v-contextmenu-item @click="removeNote">删除笔记</v-contextmenu-item>
    <v-contextmenu-divider />
    <v-contextmenu-item @click="removeFolder">删除文件夹</v-contextmenu-item>
  </v-contextmenu>
  <v-contextmenu ref="contextmenu" v-else>
    <v-contextmenu-item @click="showAddFolderModalForm">
      添加文件夹
    </v-contextmenu-item>
    <v-contextmenu-divider />
    <v-contextmenu-item @click="reFresh">刷新</v-contextmenu-item>
  </v-contextmenu>

  <el-dialog v-model:visible="addFolderModalFormVisible" @close="handleCancelByAddFolderModal" title="添加文件夹"
    width="500">
    <el-form>
      <el-form-item label="文件夹名称" :label-width="formLabelWidth">
        <el-input v-model="newFolderName" autocomplete="off" />
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
  <el-dialog v-model:visible="addFolderModalFormVisible" @close="handleCancelByAddNotebookModal" title="添加笔记"
    width="500">
    <el-form>
      <el-form-item label="笔记名称" :label-width="formLabelWidth">
        <el-input v-model="newNoteMdFileName" autocomplete="off" />
      </el-form-item>
    </el-form>
    <template #footer>
      <div class="dialog-footer">
        <el-button @click="handleCancelByAddNotebookModal">取消</el-button>
        <el-button type="primary" @click="addNodeFolder"> 确认 </el-button>
      </div>
    </template>
  </el-dialog>
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
  PropType,
  toRaw,
} from "vue";
//import { notification, Modal } from 'ant-design-vue';
import { ElMessage, ElMessageBox, ElSubMenu, ElMenuItem } from "element-plus";
import DirFileMenu from "@/utils/DirFileMenu";
import StringUtil from "@/utils/StringUtil";
import NotificationUtil from "@/utils/NotificationUtil";
import { invoke } from "@tauri-apps/api/core";
import { MdEditor } from "md-editor-v3";
import "md-editor-v3/lib/style.css";
import { NotebookInfo, ResponseData } from "@/entity/entitys";
import TreeNode from "@/components/tsx/menu/TreeNode";
import { tauri_request } from "@/utils/request";

// 处理节点点击事件
const handleNodeClick = (node: NotebookInfo) => {
  console.log("节点被点击:", node);
  console.log("节点被点击 electedNodeId:", state.selectedNodeId);
  // 这里可以添加自定义逻辑，比如：
  // - 加载节点内容
  // - 更新UI状态
  // - 执行特定操作
};

onMounted(() => {
  get_document_notebooks();
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
  newFolderName: "", // 新建问价夹的名称
  addNotebookModalFormVisible: false, // 是否显示添加笔记本表单
  addFolderModalFormVisible: false,
  confirmLoading: false, // 添加笔记表单等待状态（是否转圈圈）
  newNoteMdFileName: "", // 新建的笔记文件名

  loading: false,
  map: new Map(),
  spinning: false,
});

let notebooks_data = ref<NotebookInfo>();

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
const onError = (err) => {
  NotificationUtil.error(err.message);
};
/**
 * 文件夹及文件夹下笔记信息
 */
let show_notebooks_data = computed(() => {
  if (!notebooks_data.value) return [];
  return DirFileMenu.dir_file(notebooks_data.value.children);
});

/**
 * 判断数是否为空
 */
let data_not_null = computed(() => {
  return Array.isArray(notebooks_data.value) && notebooks_data.value.length > 0;
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
const onOpenChange = function (openKeys) {
  const latestOpenKey = openKeys.find(
    (key) => state.openKeys.indexOf(key) === -1
  );
  if (state.rootSubmenuKeys.indexOf(latestOpenKey) === -1) {
    state.openKeys = openKeys;
  } else {
    state.openKeys = latestOpenKey ? [latestOpenKey] : [];
  }
  data.currentNote = latestOpenKey;
};

/**
 * 被选中时调用
 * selectedKeys(v-model)	当前选中的菜单项 key 数组
 * @param {*} param0
 */
const selectHandle = function (node: NotebookInfo) {
  //console.log("selectedKeys {:?}", selectedKeys)
  data.selectIsFile = node.isfile;
  if (!node.isfile) {
    data.selectFolderPath = node.folder;
    return;
  }
  //重复点击自己 路径相同
  if (node.path == data.currentChooseFolderPath_md) {
    return;
  }
  //data.currentNote = node.id;
  //旧比较文件处理 
  // 保存原来编辑笔记
  if (
    (data.showMdEditor && data.notEditedMdtext_md != data.EditedMdtext_md) || (data.title_md != node.name &&
      StringUtil.isNotNullAndEmpty(data.title_md) &&
      StringUtil.isNotNullAndEmpty(data.lastTimeTitle_md))
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
                if (n == data.title_md) {
                  same_file_name = true;
                }
              });
              //无重复
              if (!same_file_name) {
                //先保存数据
                Promise.all([saveMd(data.editedMdtext_md, data.title_md, data.currentChooseNoteAllPath_md)]).then(() => {
                  // //点击了确定，开始删除
                  // invoke<ResponseData<string>>("remove_note", {
                  //   currentNotebook: data.oldpath,
                  //   currentNote: data.mdOldTitle + ".md",
                  // })
                  //   .then((res: ResponseData<string>) => {
                  //     if (res && res.code == 200) {
                  //       get_document_notebooks();
                  //       get_note_list();
                  //     } else {
                  //       NotificationUtil.error("删除旧文件操作失败!");
                  //     }
                  //   })
                  //   .catch((err) => {
                  //     console.log(err);
                  //   });
                  invoke<ResponseData<string>>("read_note_file", {
                    currentNotebook: node.path,
                    noteTitle: node.name,
                  })
                    .then((res: ResponseData<string>) => {
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
                    })
                    .catch((err) => {
                      NotificationUtil.error(err);
                    });
                })
                  .catch((err) => {
                    NotificationUtil.error("保存文件失败，" + err.getMessage());
                  });
              } else {
                //有重复名称
                NotificationUtil.warn("已有相同文件名，文件名将无法更改！");
                //保存旧文件名称
                Promise.all([saveMd(data.editedMdtext_md, data.title_md, data.currentChooseNoteAllPath_md)]).then(() => {
                  invoke<ResponseData<string>>("read_note_file", {
                    currentNotebook: node.path,
                    noteTitle: node.name,
                  })
                    .then((res) => {
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
                    })
                    .catch((err) => {
                      NotificationUtil.error(err);
                    });
                });
              }
            }
          })
          .catch((err) => {
            NotificationUtil.error("获取数据失败 ：" + err);
          });
      })
      .catch(() => {
        invoke<ResponseData<string>>("read_note_file", {
          currentNotebook: node.path,
          noteTitle: node.name,
        })
          .then((res: ResponseData<string>) => {
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
          })
          .catch((err) => {
            NotificationUtil.error(err);
          });
      });
  } else {
    invoke<ResponseData<string>>("read_note_file", {
      currentNotebook: node.path,
      noteTitle: node.name,
    })
      .then((res: ResponseData<string>) => {
        console.log("read_note_file", res);
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
      })
      .catch((err) => {
        NotificationUtil.error(err);
      });
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
  let create_folder_ = await tauri_request<ResponseData<string>>("create_folder", { father_path: data.selectFolderPath, new_folder_name: newFolderName })
  if (create_folder_ && create_folder_.code == 200) {
    NotificationUtil.success("文件夹创建成功");
    get_document_notebooks();
  } else {
    NotificationUtil.error("笔记本创建失败：" + create_folder_.msg);
  }
};
// 显示添加笔记对话框
const showAddNotebookModalForm = () => {
  let openkeys_ = state.openKeys;
  let openkey = "";
  if (openkeys_.length > 0) {
    openkey = openkeys_[openkeys_.length - 1];
  }
  if (StringUtil.isNullAndEmpty(openkey)) {
    NotificationUtil.error(`请选择文件夹`);
    return;
  }
  data.addNotebookModalFormVisible = true;
};
/**
 * 添加笔记
 */
let addNotebook = async function () {
  let newNoteMdFileName = data.newNoteMdFileName.trim();
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
  if (regex.test(data.newNoteMdFileName)) {
    NotificationUtil.error(
      '笔记名称命名不规范，不能包含\\ / : * ? " < > |请检查！'
    );
    return;
  }
  if (/^\d+$/.test(data.newNoteMdFileName)) {
    NotificationUtil.error("笔记名称命名不规范，不能为纯数字！");
    return;
  }

  let create_note_file = await tauri_request<ResponseData<string>>("create_note_file", {
    father_path: data.selectFolderPath,
    new_note_md_file_name: newNoteMdFileName,
  })
  if (create_note_file && create_note_file.code == 200) {
    data.addNotebookModalFormVisible = false; // 隐藏模态表单
    NotificationUtil.success("笔记创建成功");
    get_document_notebooks();
  } else {
    NotificationUtil.error("笔记创建失败：" + create_note_file.msg);
  }
};

/**
 * 删文件
 */
let removeNote = async function () {
  if (StringUtil.isNullAndEmpty(data.currentChooseNoteAllPath_md) && data.selectIsFile) {
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
/**
 * 删除文件夹
 */
let removeFolder = async function () {
  if (StringUtil.isNullAndEmpty(data.selectFolderPath) && !data.selectIsFile) {
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
  data.addNotebookModalFormVisible = false;
  data.newNoteMdFileName = "";
};
/**
 * 刷新笔记信息
 */
const reFresh = () => {
  invoke<ResponseData<NotebookInfo>>("get_document_notebooks")
    .then((res: ResponseData<NotebookInfo>) => {
      if (res && res.code == 200) {
        let data_ = res.data;
        console.log("\===========================", data_);
        notebooks_data.value = data_;
        state.rootSubmenuKeys = DirFileMenu.getRootSubmenuKeys(
          notebooks_data.value.children
        );
        NotificationUtil.success("刷新完成");
      }
    })
    .catch((err) => {
      NotificationUtil.error("获取数据失败 ：" + err);
    });
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
<style>
.wrapper {
  height: 100%;
  background-color: #e0e0e0;
  border: solid #cbcbcb;
  border-width: 0px 1px 0px 0px;
}
</style>
