<template>
  <el-row style="height: 100%">
    <el-col :span="5" style="height: 100%; background-color: #d9d9d9" v-contextmenu:contextmenu>
      <!-- <a-spin :spinning="spinning"> </a-spin> -->
      <div class="wrapper" v-if="show_notebooks_data.length > 0">
        <!-- <a-menu v-model:selectedKeys="state.selectedKeys" mode="inline" :open-keys="state.openKeys"
          @select="selectHandle" :items="show_notebooks_data" @openChange="onOpenChange"
          style="max-height: 100%; overflow: hidden auto"></a-menu> -->
        <el-menu default-active="2" class="el-menu-vertical-demo">
          <TreeNode v-if="notebooks_data" :node="notebooks_data" />
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
      <MdEditor v-model="data.mdText" @onSave="saveMdText" @onError="onError" />
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
import { computed, h, defineComponent, onMounted, reactive, ref, toRefs, createVNode, PropType, toRaw } from 'vue';
//import { notification, Modal } from 'ant-design-vue';
import { ElMessage, ElMessageBox, ElSubMenu, ElMenuItem } from 'element-plus';
import DirFileMenu from '@/utils/DirFileMenu';
import StringUtil from '@/utils/StringUtil';
import NotificationUtil from '@/utils/NotificationUtil';
import { invoke } from '@tauri-apps/api/core';
import { MdEditor } from 'md-editor-v3';
import 'md-editor-v3/lib/style.css';
import { NotebookInfo, ResponseData, ResponseData2 } from '@/entity/entitys';

// 新增 TreeNode 组件

// 定义TreeNode组件
const TreeNode = defineComponent({
  name: 'TreeNode',
  props: {
    node: {
      type: Object as PropType<NotebookInfo>,
      required: true
    }
  },
  setup(props) {
    return () => {
      let node: NotebookInfo = props.node;
      //const node = Object.assign({},rawNode);
      console.log("node==============", node);
      console.log("node.id==============", node.id);
      if (!node || !node.id) {
        console.error("Invalid node data", node);
        return null; // 避免渲染错误节点
      }
      const commonProps = { index: node.id, key: node.id };
      if (node.children && node.children.length > 0) {
        // 文件夹节点
        return h(
          ElSubMenu,
          commonProps,
          {
            title: () => h('span', node.name),
            default: () => node.children.map((child: NotebookInfo) =>
              h(TreeNode, { node: child, key: child.id })
            )
          }
        );
      } else {
        // 文件节点
        return h(
          ElMenuItem,
          commonProps,
          () => h('span', node.name)
        );
      }
    };
  }
});
onMounted(() => {
  get_document_notebooks()
})

const data = reactive({
  notebooks: [], // 笔记本列表
  notes: [], // 笔记列表
  selectedKeys: [], // 笔记本选中项key
  currentNotebook: "", // 当前选中的文件夹
  currentOldNotebook: "",//上次选中的文件夹
  currentNote: "", // 当前选中笔记
  notEditedMdtext: "", // 未编辑的文章内容
  showMdEditor: false, // 是否显示编辑器
  newFolderName: "", // 新建笔记本的名称
  addNotebookModalFormVisible: false, // 是否显示添加笔记本表单
  addFolderModalFormVisible: false,
  confirmLoading: false, // 添加笔记表单等待状态（是否转圈圈）
  newNoteMdFileName: "", // 新建的笔记文件名
  mdTitle: "", // 文章标题
  mdOldTitle: "", // 文章旧标题标题
  mdText: "", // 文章内容Markdown格式
  loading: false,
  map: new Map(),
  spinning: false,
})

let notebooks_data = ref<NotebookInfo>()


/**
 *  获取笔记本数据
 */
let get_document_notebooks = () => {
  // 获取笔记本数据
  invoke<ResponseData2<NotebookInfo>>('get_document_notebooks').then((res: ResponseData2<NotebookInfo>) => {
    console.log("res===============", res);
    if (res && typeof res === 'object' && 'code' in res && res.code == 200) {
      //值 返回ResponseData 对象，包含data、code、msg参数 例如：data {"id":"1752761254929","parent_id":"","path":"C:\\Users\\kun15\\Documents\\notebooks","name":"notebooks","isfile":false,"children":[]}
      let data_: NotebookInfo = typeof res.data === 'string' ? JSON.parse(res.data) : res.data;
      notebooks_data.value = data_;
      state.rootSubmenuKeys = DirFileMenu.getRootSubmenuKeys(notebooks_data.value .children);
    }

  }).catch((err) => {
    NotificationUtil.error("获取数据失败 ：" + err);
  });
}
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
})

const state = reactive({
  rootSubmenuKeys: [] as string[],
  openKeys: [],
  selectedKeys: [],
});
/**
 * SubMenu 展开/关闭的回调
 * @param {*当前展开的 SubMenu 菜单项 key 数组} openKeys
 */
const onOpenChange = openKeys => {
  const latestOpenKey = openKeys.find(key => state.openKeys.indexOf(key) === -1);
  if (state.rootSubmenuKeys.indexOf(latestOpenKey) === -1) {
    state.openKeys = openKeys;
  } else {
    state.openKeys = latestOpenKey ? [latestOpenKey] : [];
  }
  data.currentNotebook = latestOpenKey;
};

/**
  * 被选中时调用
 * selectedKeys(v-model)	当前选中的菜单项 key 数组
 * @param {*} param0
 */
const selectHandle = function ({ item, key, selectedKeys }) {
  console.log("selectedKeys {:?}", selectedKeys)
  data.currentNote = key;
  //保存原来编辑笔记
  if ((data.showMdEditor && data.notEditedMdtext != data.mdText) || (data.mdTitle != data.mdOldTitle && StringUtil.isNotNullAndEmpty(data.mdTitle) && StringUtil.isNotNullAndEmpty(data.mdOldTitle))) {
    ElMessageBox.confirm(
      '上一份笔记未保存，需要保存吗?',
      '保存笔记',
      {
        confirmButtonText: '保存',
        cancelButtonText: '不保存',
        type: 'warning',
      }
    )
      .then(() => {

        invoke('get_note_list', { notebook: data.currentOldNotebook }).then(res => {
          let same_file_name = false;
          if (res && res.code == 200) {
            var tem = res.data;
            tem.map(n => {
              if (n == data.mdTitle && data.mdTitle != data.mdOldTitle) {
                same_file_name = true;
              }
            });
            if (!same_file_name) {
              Promise.all([saveMd(data.mdText, data.mdTitle)])
                .then(() => {

                  //点击了确定，开始删除
                  invoke('remove_note',
                    {
                      currentNotebook: data.currentOldNotebook,
                      currentNote: data.mdOldTitle + ".md"
                    }).then(res => {
                      if (res && res.code == 200) {
                        get_document_notebooks();
                        get_note_list();

                      } else {
                        NotificationUtil.error("删除旧文件操作失败!");
                      }
                    }).catch(err => { console.log(err); });
                  invoke('read_note_file', { currentNotebook: data.currentNotebook, noteTitle: item.name }).then(res => {
                    if (res && res.code == 200) {
                      var tem = res.data;
                      data.mdText = tem;
                      data.notEditedMdtext = tem; // 原始笔记内容
                      data.mdTitle = item.name;
                      data.mdOldTitle = item.name;
                      data.currentOldNotebook = data.currentNotebook;
                      data.showMdEditor = true;
                    } else {
                      NotificationUtil.error("读取失败：" + res.msg);
                    }
                  }).catch(err => {
                    NotificationUtil.error(err);
                  });
                });
            } else {
              NotificationUtil.warn("已有相同文件名，文件名将无法更改！");
              Promise.all([saveMd(data.mdText, data.mdOldTitle)])
                .then(() => {
                  invoke('read_note_file', { currentNotebook: data.currentNotebook, noteTitle: item.name }).then(res => {
                    if (res && res.code == 200) {
                      var tem = res.data;
                      data.mdText = tem;
                      data.notEditedMdtext = tem; // 原始笔记内容
                      data.mdTitle = item.name;
                      data.mdOldTitle = item.name;
                      data.currentOldNotebook = data.currentNotebook;
                      data.showMdEditor = true;
                    } else {
                      NotificationUtil.error("读取失败：" + res.msg);
                    }
                  }).catch(err => {
                    NotificationUtil.error(err);
                  });
                });
            }
          }
        }).catch((err) => {
          NotificationUtil.error("获取数据失败 ：" + err);
        });
      })
      .catch(() => {
        invoke('read_note_file', { currentNotebook: data.currentNotebook, noteTitle: item.name }).then(res => {
          if (res && res.code == 200) {
            var tem = res.data;
            data.mdText = tem;
            data.notEditedMdtext = tem; // 原始笔记内容
            data.mdTitle = item.name;
            data.mdOldTitle = item.name;
            data.showMdEditor = true;
            data.currentOldNotebook = data.currentNotebook;
          } else {
            NotificationUtil.error("读取失败：" + res.msg);
          }
        }).catch(err => {
          NotificationUtil.error(err);
        });
      })


  } else {
    invoke('read_note_file', { currentNotebook: data.currentNotebook, noteTitle: item.name }).then(res => {
      if (res && res.code == 200) {
        var tem = res.data;
        data.mdText = tem;
        data.notEditedMdtext = tem; // 原始笔记内容
        data.mdTitle = item.name;
        data.mdOldTitle = item.name;
        data.showMdEditor = true;
        data.currentOldNotebook = data.currentNotebook;
      } else {
        NotificationUtil.error("读取失败：" + res.msg);
      }
    }).catch(err => {
      NotificationUtil.error(err);
    });
  }



}

/**
 * 保存笔记 切换笔记但未保存时调用
 * @param {笔记内容} mdText
 * @param {标题} mdTitle
 */
const saveMd = (mdText, mdTitle) => {
  var p = new Promise(function () {

    if (data.mdTitle == '') {    // 标题为空
      NotificationUtil.error('请输入标题', 2);
      return;
    }

    if (data.currentOldNotebook && data.currentNote) { // 不为空
      // 调用保存笔记方法
      invoke('save_note', {
        currentNotebook: data.currentOldNotebook,
        currentNote: data.currentNote,
        mdTitle: mdTitle,
        mdText: mdText
      }).then(res => {
        if (res && res.code == 200) {
          NotificationUtil.success("笔记保存成功", 1);
        } else {
          NotificationUtil.error('笔记保存失败：' + res.msg);
        }

      }).then(err => { console.log(err); })

    } else {  // 存在为空情况
      NotificationUtil.error('请选定笔记本和笔记');
    }
    return p
  })

};

/**
 * 保存文章/笔记 编辑器点击保存
 * @param {*} mdText
 */
const saveMdText = (mdText) => {
  if (data.mdTitle == '') {    // 标题为空
    NotificationUtil.error('请输入标题', 2);
    return;
  }
  invoke('get_note_list', { notebook: data.currentOldNotebook }).then(res => {
    let same_file_name = false;
    if (res && res.code == 200) {
      var tem = res.data;
      tem.map(n => {
        if (n == data.mdTitle && data.mdTitle != data.mdOldTitle) {
          same_file_name = true;
        }
      });
      if (!same_file_name) {
        if (data.currentOldNotebook && data.currentNote) { // 不为空
          // 调用保存笔记方法
          invoke('save_note', {
            currentNotebook: data.currentOldNotebook,
            currentNote: data.currentNote,
            mdTitle: data.mdTitle,
            mdText: mdText
          }).then(res => {
            if (res && res.code == 200) {
              NotificationUtil.success("笔记保存成功");
              //新文件和旧文件名不同，需要将旧文件删除
              if (data.mdTitle != data.mdOldTitle && StringUtil.isNotNullAndEmpty(data.mdTitle) && StringUtil.isNotNullAndEmpty(data.mdOldTitle)) {
                //点击了确定，开始删除
                invoke('remove_note',
                  {
                    currentNotebook: data.currentOldNotebook,
                    currentNote: data.mdOldTitle + ".md"
                  }).then(res => {
                    if (res && res.code == 200) {
                      data.currentNote = data.mdTitle;
                      data.mdOldTitle = data.mdTitle;
                      data.notEditedMdtext = mdText;
                      get_document_notebooks();
                      get_note_list();

                    } else {
                      NotificationUtil.error("删除旧文件操作失败!");
                    }
                  }).catch(err => { console.log(err); });
              }
            } else {
              NotificationUtil.error('笔记保存失败：' + res.msg);
            }

          }).then(err => { console.log(err); })

        } else {  // 存在为空情况
          NotificationUtil.error('请选定笔记本和笔记');
        }
      } else {
        NotificationUtil.warning("已有相同文件名，文件名将无法更改！");
        if (data.currentOldNotebook && data.currentNote) {
          // 调用保存笔记方法 保存新文件
          invoke('save_note', {
            currentNotebook: data.currentOldNotebook,
            currentNote: data.currentNote,
            mdTitle: data.mdOldTitle,
            mdText: mdText
          }).then(res => {
            if (res && res.code == 200) {
              data.mdTitle = data.mdOldTitle;
              data.notEditedMdtext = mdText;
            } else {
              NotificationUtil.error('笔记保存失败：' + res.msg);
            }

          })

        } else {  // 存在为空情况
          NotificationUtil.error('请选定笔记本和笔记');
        }
      }
    } else {
      NotificationUtil.error("获取笔记列表数据失败：" + res.msg);
    }
  }).catch(err => {
    console.log(err)
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
let addNodeFolder = function () {
  let notebookName = data.newFolderName.trim();
  if (notebookName == null || notebookName == undefined || notebookName == "") {
    NotificationUtil.error("创建文件夹失败，请填写文件夹名称！");
    return;
  }
  let FolderNameSet = DirFileMenu.getFolderNameSet(notebooks_data.value);
  if (FolderNameSet.has(notebookName)) {
    NotificationUtil.success("已有相同文件夹名，请检查！");
    data.newFolderName = "";
    return;
  }
  var regex = /.*[/\\:*?|].*/;
  //存在返回true，不存在返回false
  if (regex.test(notebookName)) {
    NotificationUtil.error("文件夹命名不规范，不能包含\\ / : * ? \" < > |请检查！");
    return;
  }
  if (/^\d+$/.test(notebookName)) {
    NotificationUtil.error("文件夹命名不规范，不能为纯数字！");
    return;
  }
  data.confirmLoading = true;
  data.loading = true;
  // 创建笔记本
  invoke('create_notebook', { notebookName: notebookName }).then(res => {
    if (res && res.code == 200) {
      NotificationUtil.success("文件夹创建成功", 1);
      // data.notebooks.push(notebookName);
      data.currentNotebook = notebookName; // 当前笔记本为新建的
      state.selectedKeys = [notebookName]; // 选中新建的笔记本
      data.newFolderName = ""; // 清空
      data.addFolderModalFormVisible = false; // 隐藏模态表单
      get_document_notebooks();
    } else {
      NotificationUtil.error("笔记本创建失败：" + res.msg);
    }
    data.confirmLoading = false; // 停止转圈圈
    data.loading = false;
  }).catch(err => { console.log(err); });
};
// 显示添加笔记对话框
const showAddNotebookModalForm = () => {
  let openkeys_ = state.openKeys;
  let openkey = "";
  if (openkeys_.length > 0) {
    openkey = openkeys_[openkeys_.length - 1]
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
let addNotebook = function () {
  //判断是否能已有相同文件
  let openkeys_ = state.openKeys;
  let openkey = "";
  if (openkeys_.length > 0) {
    openkey = openkeys_[openkeys_.length - 1]
  }
  let FolderChilrenFileNameSet = DirFileMenu.getFolderChilrenFileNameSet(notebooks_data.value, openkey);

  let newNoteMdFileName = data.newNoteMdFileName.trim();
  if (newNoteMdFileName == null || newNoteMdFileName == undefined || newNoteMdFileName == "") {
    NotificationUtil.error("创建文件失败，请填写文件名称！");
    return;
  }
  if (FolderChilrenFileNameSet.has(newNoteMdFileName)) {
    data.newNoteMdFileName = "";
    NotificationUtil.error("已有相同文件名，请检查！");
    return;
  }
  var regex = /.*[/\\:*?|].*/;
  //存在返回true，不存在返回false
  if (regex.test(data.newNoteMdFileName)) {
    NotificationUtil.error("笔记名称命名不规范，不能包含\\ / : * ? \" < > |请检查！");
    return;
  }
  if (/^\d+$/.test(data.newNoteMdFileName)) {
    NotificationUtil.error("笔记名称命名不规范，不能为纯数字！");
    return;
  }
  var notebook = data.currentNotebook;
  if (notebook) {
    invoke('create_note_file', { notebook: notebook, newNoteMdFileName: newNoteMdFileName }).then(res => {
      if (res && res.code == 200) {
        var fileName = res.data;
        NotificationUtil.success("笔记创建成功：" + fileName, 1);
        data.newNoteMdFileName = fileName;
        data.notes.push({ title: fileName });
        data.currentNote = fileName;    // 置当前笔记为新建的临时笔记
        // 打开该临时笔记
        data.mdTitle = fileName;
        data.mdText = '';
        data.showMdEditor = true;
        data.mdOldTitle = fileName;
        data.notEditedMdtext = '';
        data.addNotebookModalFormVisible = false; // 隐藏模态表单
        get_document_notebooks();
      } else {
        NotificationUtil.error("笔记创建失败：" + res.msg);
      }
    }).catch(err => { console.log(err); });
  }

};


/**
 * 删除笔记
 */
let removeNote = function () {
  if (StringUtil.isNullAndEmpty(data.currentNotebook) || StringUtil.isNullAndEmpty(data.currentNote) || data.currentNotebook != data.currentOldNotebook) {
    NotificationUtil.warning('请选择笔记');
    return;
  };
  ElMessageBox({
    title: '删除笔记',
    confirmButtonText: 'OK',
    cancelButtonText: 'Cancel',
    type: 'warning',
    message: h('p', null, [
      h('span', null, `确认删除笔记 ${data.currentNote} 吗？ `)
    ]),
  })
    .then(() => {
      //查看删除笔记是否是正在编辑的笔记
      //点击了确定，开始删除
      invoke('remove_note',
        {
          currentNotebook: data.currentNotebook,
          currentNote: data.currentNote
        }).then(res => {
          if (res && res.code == 200) {
            NotificationUtil.success("操作成功");
            data.currentNote = "";
            data.showMdEditor = false;
            data.mdText = "";
            data.mdTitle = "";
            data.mdOldTitle = "";
            data.notEditedMdtext = "";
            get_document_notebooks();
          } else {
            NotificationUtil.error("操作失败：" + res.msg);
          }
        }).catch(err => { console.log(err); });
    })
    .catch(() => {

    })
};
/**
* 删除文件夹
*/
let removeFolder = function () {
  if (StringUtil.isNullAndEmpty(data.currentNotebook)) {
    NotificationUtil.error("请选择文件夹！");
    return;
  }
  ElMessageBox({
    title: '删除文件',
    message: h('div', {}, [
      h('br'),
      h('br'),
      h('p', { style: 'color:red;' }, `删除文件夹 "${data.currentNotebook}" 吗，文件夹下笔记文件也将一并删除！`),
      h('br'),
      h('br'),
    ]),
    showCancelButton: true,
    confirmButtonText: '删除',
    cancelButtonText: '取消',

  }).then((action) => {
    // 点击了确定，开始删除
    invoke('remove_notebook',
      {
        currentNotebook: data.currentNotebook,
      }).then(res => {
        if (res && res.code == 200) {
          NotificationUtil.success("操作成功");
          data.currentNotebook = "";
          // 重新获取笔记本列表数据
          get_document_notebooks();
        } else {
          NotificationUtil.error("操作失败：" + res.msg);
        }
      }).catch(err => { console.log(err); });

  })

}

/**
 * 关闭添加文件夹模态框
 */
const handleCancelByAddFolderModal = () => {
  data.addFolderModalFormVisible = false;
  data.newFolderName = '';
};
/**
 * 关闭添加笔记模态框
 */
const handleCancelByAddNotebookModal = () => {
  data.addNotebookModalFormVisible = false;
  data.newNoteMdFileName = '';
};
/**
 * 刷新笔记信息
 */
const reFresh = () => {
  invoke('get_document_notebooks').then((res) => {
    if (res && res.code == 200) {
      let data_ = JSON.parse(res.data);
      console.log("\===========================", data_);
      notebooks_data.value = data_.children;
      state.rootSubmenuKeys = DirFileMenu.getRootSubmenuKeys(notebooks_data.value);
      NotificationUtil.success("刷新完成");
    }
  }).catch((err) => {
    NotificationUtil.error("获取数据失败 ：" + err);
  });
};

/**
 * 获取文件夹下笔记列表
 */
const get_note_list = () => {
  invoke('get_note_list', { notebook: data.currentNotebook }).then(res => {
    if (res && res.code == 200) {
      var tem = res.data;
      data.notes = tem.map(n => {
        return { title: n }
      });
    } else {
      NotificationUtil.error("获取笔记列表数据失败：" + res.msg);
    }
  }).catch(err => {
    console.log(err);
  });
}
//const [api, contextHolder] = NotificationUtil.useNotification();
// return {
//   state,
//   onOpenChange,
//   get_document_notebooks,
//   notebooks_data,
//   show_notebooks_data,
//   data_not_null,
//   handleCancelByAddFolderModal,
//   handleCancelByAddNotebookModal,
//   ...toRefs(data),
//   saveMdText,
//   saveMd,
//   selectHandle,
//   addNotebook,
//   addNodeFolder,
//   removeNote,
//   removeFolder,
//   showAddFolderModalForm,
//   showAddNotebookModalForm,

//   reFresh,
//   get_note_list
// }
//api,
//  contextHolder,
</script>
<style>
.wrapper {
  height: 100%;
  background-color: #e0e0e0;
  border: solid #cbcbcb;
  border-width: 0px 1px 0px 0px;
}
</style>
