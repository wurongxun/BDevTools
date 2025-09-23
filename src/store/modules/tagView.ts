import { defineStore } from 'pinia'
import ToolUtils from '@/utils/ToolUtils'

import type { ITagItem } from '@/Types/TagView'
import type { RouteRecordRaw } from 'vue-router'

interface IResult extends ITagItem {
  index: number
}

/**
 * 具体逻辑：
 *1. 遍历传入的标签数组arr
 *2. 比较每个元素的path属性与目标项item的path属性
 *3. 如果找到path相同的项，返回包含该项和其索引的对象
 *4. 如果遍历完都没找到，返回false
 *用于避免重复标签页。
 * @param item
 * @param arr
 * @returns
 */
function hasIncludeTab(item: ITagItem, arr: Array<ITagItem>): boolean | IResult {
  for (let index = 0; index < arr.length; index++) {
    const element = arr[index]
    if (element.path === item.path) return { ...item, index }
  }
  return false
}
/**
 * 代码的功能是递归遍历路由记录数组，筛选出带有[affix]标记的路由，并将它们转换为标签项格式添加到结果数组中。
 * 具体逻辑：
 * 1. 遍历路由数组中的每一项
 * 2. 如果路由项有[affix]标记，则创建对应的标签项并加入结果数组
 * 3. 如果路由项有子路由，则递归处理子路由
 * 4. 返回包含所有affix标签的数组
 * 这是用于生成固定标签页的功能。
 * */
function getAffix(arr: RouteRecordRaw[], result: ITagItem[]): ITagItem[] {
  arr.map((item) => {
    if (item.meta?.affix) {
      let current = JSON.parse(JSON.stringify(item))
      result.push({
        name: current,
        path: current.path,
        title: current.meta.title,
        query: current.query,
        params: current.params,
        affix: true
      })
    }
    if (item.children && item.children.length) {
      getAffix(item.children, result)
    }
  })
  return result
}

const tagViewStore = defineStore('tagViewStore', {
  state: () => {
    return {
      tagViewList: [] as Array<ITagItem>,
      cacheList: [] as Array<string>
    }
  },
  actions: {
    /**
     * @description 初始化 tagview 数组， 把 affix 为 true 的数据放入其中
     */
    initTagViewList(router: RouteRecordRaw[]) {
      getAffix(router, this.tagViewList)
    },

    /**
     * @param { ITagItem } item
     * @description 添加 tag view 信息
     */
    addTagView(item: ITagItem) {
      const result = hasIncludeTab(item, this.tagViewList)
      if (!result) {
        this.tagViewList.push(item)
      } else {
        // 判断当前修改 params 是否为同一个， 如果不是的话需要重载
        const obj1 = JSON.parse(
          JSON.stringify(this.tagViewList[(result as IResult).index].params || {})
        )
        const obj2 = JSON.parse(JSON.stringify((result as IResult).params || {}))
        if (ToolUtils.notEmptyObject(obj2)) {
          if (!ToolUtils.objectIsEqual(obj1, obj2)) {
            const name: string = (result as IResult).name
            const index = this.cacheList.indexOf(name)
            this.cacheList.splice(index, 1)
            this.tagViewList[(result as IResult).index] = { ...(result as IResult) }
            setTimeout(() => {
              this.cacheList.push(name)
            }, 500)
          }
        }
      }
    },
    /**
     * @param { ITagItem } tag
     * @description 删除 tag view
     */
    removeTagView(tag: ITagItem) {
      for (let index = 0; index < this.tagViewList.length; index++) {
        const element = this.tagViewList[index]
        if (element.path === tag.path) {
          this.tagViewList.splice(index, 1)
          return
        }
      }
    },
    /**
     * @param { string } path
     * @description 删除 tag view
     */
    removeTagViewByPath(path: string) {
      for (let index = 0; index < this.tagViewList.length; index++) {
        const element = this.tagViewList[index]
        if (element.path === path) {
          this.tagViewList.splice(index, 1)
          return
        }
      }
    },
    /**
     * @param { string } cacheName
     * @description 添加 keep-alive 缓存
     */
    addCacheList(cacheName: string) {
      if (!this.cacheList.includes(cacheName)) {
        this.cacheList.push(cacheName)
      }
    },
    getTagviewList() {
      return this.tagViewList
    },
    /**
     * @description 所有数据清空
     */
    resetTagViewList() {
      this.$reset()
    }
  }
})

export default tagViewStore
