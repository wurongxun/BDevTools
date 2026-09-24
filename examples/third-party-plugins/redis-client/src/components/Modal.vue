<template>
  <div class="dialog-mask" @click.self="$emit('close')">
    <div class="dialog">
      <h3>{{ title }}</h3>
      <div class="d-body"><slot /></div>
      <div v-if="$slots.footer" class="d-foot">
        <slot name="footer" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ title: string }>()
defineEmits<{ (e: 'close'): void }>()
</script>
