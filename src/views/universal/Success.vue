<script lang="ts" setup>
import Container from '@/components/Container.vue'
import { ref } from 'vue'
import type { Ref } from 'vue'
import success from '@/lottie/success.json'
import Lottie from '@/components/Lottie.vue'

let lottieDomRef: Ref<InstanceType<typeof Lottie> | null> = ref(null)

const onHover = () => {
  lottieDomRef.value!.lottieIns?.play()
}
</script>

<template>
  <Container class="container">
    <Lottie ref="lottieDomRef" :json-name="success" @click="onHover"></Lottie>
    <p>成功</p>
  </Container>
</template>

<style scoped lang="scss">
.container {
  text-align: center;
}
</style>
