<template>
  <div class="app-link" @click="handleTo">
    <slot />
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import VerifyUtils from '@/utils/VerifyUtils'

const router = useRouter()
const props = defineProps(['to', 'redirect'])

function handleTo() {
  if (props.redirect && VerifyUtils.validURL(props.redirect)) {
    window.open(props.redirect, '_blank')
    return
  }
  console.log('handleTo', props.to)
  router.push({
    path: props.to
  })
}
</script>

<style scoped lang="scss">
.app-link {
  width: 100%;
  user-select: none;
}
</style>
