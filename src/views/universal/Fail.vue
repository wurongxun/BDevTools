<script lang="ts" setup>
import Container from '@/components/Container.vue'
import { ref } from 'vue'
import type { Ref } from 'vue'
import fail from '@/lottie/fail.json'
import Lottie from '@/components/Lottie.vue'

let lottieDomRef: Ref<Element | null> = ref(null)

</script>

<template>
  <Container class="container">
    <Lottie ref="lottieDomRef" :json-name="fail"></Lottie>
    <p>失败</p>
  </Container>
</template>

<style scoped lang="scss">
.container {
  text-align: center;
}
</style>
