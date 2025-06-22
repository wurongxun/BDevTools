import { h } from 'vue';
import StringUtil from './StringUtil'
let _ = {}
_.dir_file = function (children_data) {
    //console.log("children_data: "+JSON.parse(children_data));
    //console.log("children_data: "+JSON.stringify(children_data));
    let data = new Array();
    for (var i in children_data) {
        console.log("i  : " + i);
        //let myObj = {};
        
        let parent_id = children_data[i].parent_id;
        let label = children_data[i].name;
        let key = children_data[i].id;
        let isfile = children_data[i].isfile;
        let path = children_data[i].path;
        let children = children_data[i].children;
        let children_ = ([]);
        if (children != null || children == undefined) {
            children_ = _.dir_file(children);
        }
        if (!isfile) {
            data.push(_.getItem(parent_id,isfile, path, label, key, () => h(), children_));
        } else {
            data.push(_.getItem(parent_id,isfile, path, label, key, () => h()));
        }

    }
    return data;
}

_.getRootSubmenuKeys = function (children_data) {
    let data = new Array();
    for (var i in children_data) {
        let key = children_data[i].id;
        data.push(key)
    }
    return data;
}

_.getItem = function (parent_id,isfile, path, label, key, icon, children, type) {
    let name=''
    if(StringUtil.isNotNullAndEmpty(label)){
        let arr=label.split('.');
        if(arr.length>1){
            name=arr[0];
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
}
/**
 * 
 * 获取目录下文件，在创建新文件时使用，验证是否同名 返回Set集合
 * children_data 目录及文件数据  openkey 打开的文件夹id
 */
_.getFolderChilrenFileNameSet = ({children_data, openkey}) => {
    let data = new Set();
    for (var i in children_data) {
        let id = children_data[i].id;
        if (openkey == id) {
            let children = children_data[i].children;
            for (var j in children) {
                let name_ = children[j].name;
                let array = name_.split(".");
                if(array[0] != null && array[0] != "" && array[0] != undefined){
                    data.add(array[0]);
                }
            }
        }
    }
    return data;
}
/**
 * 获取文件夹名称
 * @returns 
 */
_.getFolderNameSet = (children_data) => {
    let data = new Set();
    for (var i in children_data) {
        let name = children_data[i].name;
        if (name != null && name != "" && name != undefined) {
            data.add(name);
        }
    }
    return data;
}
export default _;