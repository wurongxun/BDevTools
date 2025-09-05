<!-- 创建一个新的组件 WrapperNoteBook.vue -->
<template>
  <div class="notebook-wrapper">
    <component 
      :is="currentComponent" 
      :ref="setCurrentRef"
      v-bind="currentProps"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import NoteBookEdit from './NoteBookEdit.vue'

const route = useRoute()

// 存储组件实例的缓存
const componentCache = new Map()
const componentRefs = ref(new Map())

// 当前组件引用
const currentComponent = ref(NoteBookEdit)
const currentProps = ref({})

// 设置当前组件引用
const setCurrentRef = (el) => {
  if (el) {
    componentRefs.value.set(route.path, el)
  }
}

// 获取当前路由对应的组件实例
const getCurrentInstance = () => {
  const key = route.path
  
  if (!componentCache.has(key)) {
    // 创建新的组件实例配置
    const instanceData = {
      component: NoteBookEdit,
      props: {
        ...route.meta,
        routePath: route.path,
        routeName: route.name
      }
    }
    componentCache.set(key, instanceData)
  }
  
  return componentCache.get(key)
}

// 监听路由变化
watch(
  () => route.path,
  () => {
    const instance = getCurrentInstance()
    currentComponent.value = instance.component
    currentProps.value = instance.props
  },
  { immediate: true }
)

// 获取指定路由的组件实例
const getComponentRef = (path) => {
  return componentRefs.value.get(path)
}

// 暴露给父组件的方法
defineExpose({
  getComponentRef
})
</script>