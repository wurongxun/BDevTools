import TypeIt from 'typeit'
import { onMounted } from 'vue'

// 使用 typeIt 插件实现的打字机效果， 具体使用可参考 https://www.typeitjs.com/

/**
 * @param { string } dom
 * @param { TypeItOptions } options
 * @description
 */

let instance: TypeIt | null = null as any

export const useTyper = (dom: string | Element, options?: any) => {
  const typeItOptions: any = {
    speed: 100,
    cursor: true,
    cursorSpeed: 800,
    waitUntilVisible: true
  }

  Object.assign(typeItOptions, options)
  instance = new (TypeIt as any)(dom, typeItOptions)
  instance?.go()
  return instance
}
