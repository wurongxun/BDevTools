<script lang="ts" setup>
import lottie from 'lottie-web'
import { onMounted, ref } from 'vue'
import type { Ref } from 'vue'
import type { AnimationItem } from 'lottie-web'

const props = defineProps({
  option: {
    type: Object,
    default: () => ({})
  },
  width: {
    type: String,
    default: () => '40%'
  },
  height: {
    type: String
  },
  jsonName: {
    required: true
  },
  style: {
    type: Object,
    default: () => ({
      margin: '20px auto'
    })
  }
})

let lottieDomRef: Ref<Element | null> = ref(null)
let lottieIns: Ref<AnimationItem | null> = ref(null)

onMounted(() => {
  lottieIns.value = lottie.loadAnimation({
    container: lottieDomRef.value as Element, //选择渲染dom
    renderer: props.option.renderer || 'svg', //渲染格式
    loop: false, //循环播放
    autoplay: props.option.autoplay || true, //是否i自动播放
    animationData: props.jsonName //渲染动效json
  })
})
defineExpose({ lottieIns })
</script>

<template>
  <div
    class="lottie-instance"
    :style="{ ...props.style, width: props.width }"
    ref="lottieDomRef"
  ></div>
</template>
