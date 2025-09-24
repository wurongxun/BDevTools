<template>
  <el-icon @click="dark_change" class="icon-box" v-show="!isDark">
    <Sunny />
  </el-icon>
  <el-icon @click="dark_change" class="icon-box" v-show="isDark">
    <Moon />
  </el-icon>
</template>

<script lang="ts" setup>
import { isDark, toggleDark } from '@/composables/useDark'
import useStore from '@/store'
import { onMounted } from 'vue';
import { load, Store } from '@tauri-apps/plugin-store';

const { system } = useStore()
let store: Store | undefined = undefined;

async function dark_change() {
  if (store) {
    toggleDark()
    await store.set('dark', { value: isDark.value });
    await store.save();
    
    system.toggleDark(isDark.value)
  }
}
onMounted(async () => {
  store = await load('settings.json', {
    autoSave: false,
    defaults: {}
  });
  let dark_val = await store.get<{ value: boolean }>('dark');
  if (dark_val !== undefined) {
    if (dark_val.value != isDark.value) {
      toggleDark()
      system.toggleDark(isDark.value)
    }
  }
})
const switchTheme = () => {
  toggleDark()
  system.toggleDark(isDark.value)
}
</script>

<style lang="scss" scoped>
@use '@/styles/variables.scss';

.icon-box {
  cursor: pointer;
  margin-right: 12px;
  margin-bottom: 2px;

  &:hover {
    color: $--theme-color;
  }
}
</style>
