import { h } from "vue";
import StringUtil from "./StringUtil";
import { NotebookInfo } from "@/entity/entitys";
// 修复：定义接口确保类型安全
interface DirFileMenuUtils {
  dir_file: (children_data: NotebookInfo[]) => NotebookInfo[];
  getRootSubmenuKeys: (children_data: NotebookInfo[]) => string[];
  getItem: (...args: any[]) => any;
  getFolderChilrenFileNameSet: (params: any) => Set<any>;
  getFolderNameSet: (children_data: NotebookInfo) => Set<any>;
}
let _: DirFileMenuUtils = {
  dir_file: function (children_data: NotebookInfo[]): NotebookInfo[] {
    // console.log("children_data: ", children_data);
    // console.log("children_data: " + JSON.stringify(children_data));
    // let data = new Array();
    // for (var i in children_data) {
    //   console.log("i  : ", i);
    //   //let myObj = {};
    //   console.log("children_data[i]  : ", children_data[i]);
    //   let parent_id = children_data[i].parent_id;
    //   let label = children_data[i].name;
    //   let key = children_data[i].id;
    //   let isfile = children_data[i].isfile;
    //   let path = children_data[i].path;
    //   let children = children_data[i].children;
    //   let children_ = [];
    //   if (children != null || children == undefined) {
    //     children_ = _.dir_file(children);
    //   }
    //   if (!isfile) {
    //     data.push(
    //       _.getItem(parent_id, isfile, path, label, key, () => h(), children_)
    //     );
    //   } else {
    //     data.push(_.getItem(parent_id, isfile, path, label, key, () => h()));
    //   }
    // }
    return children_data;
  },

  getRootSubmenuKeys: function (children_data: NotebookInfo[]): Array<string> {
    let data = new Array<string>();
    for (var i in children_data) {
      let key = children_data[i].id;
      data.push(key);
    }
    return data;
  },

  getItem: function (
    parent_id,
    isfile,
    path,
    label,
    key,
    icon,
    children,
    type
  ) {
    let name = "";
    if (StringUtil.isNullOrEmpty(label)) {
      let arr = label.split(".");
      if (arr.length > 1) {
        name = arr[0];
      }
    }
    return {
      parent_id,
      path,
      isfile,
      key,
      icon,
      children,
      label,
      type,
      name,
    };
  },
  /**
   *
   * 获取目录下文件，在创建新文件时使用，验证是否同名 返回Set集合
   * children_data 目录及文件数据  openkey 打开的文件夹id
   */
  getFolderChilrenFileNameSet: ({ children_data, openkey }) => {
    let data = new Set();
    for (var i in children_data) {
      let id = children_data[i].id;
      if (openkey == id) {
        let children = children_data[i].children;
        for (var j in children) {
          let name_ = children[j].name;
          let array = name_.split(".");
          if (array[0] != null && array[0] != "" && array[0] != undefined) {
            data.add(array[0]);
          }
        }
      }
    }
    return data;
  },
  /**
   * 获取文件夹名称
   * @returns
   */
  getFolderNameSet: (children_data: NotebookInfo) => {
    let data = new Set();
    // let children= children_data.children;
    // for (var i in children) {
    //   let name = children[i].name;
    //   if (name != null && name != "" && name != undefined) {
    //     data.add(name);
    //   }
    // }
    return data;
  },
};
export default _;
