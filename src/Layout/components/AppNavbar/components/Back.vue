<template>
  <div class="scale-icon">
    <el-icon size="20px">
      <Back />
    </el-icon>
  </div>
</template>

<script lang="ts" setup>
import useStore from '@/store'
import { storeToRefs } from 'pinia'
import { Back, Minus, Close } from "@element-plus/icons-vue";

const { system } = useStore()
const { isCollapse } = storeToRefs(system)
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss';

.scale-icon {
  cursor: pointer;
  width: 60px;
  height: 100%;
  font-size: 0;
  display: flex;
  align-items: center;
  padding-left: 20px;
  margin-right: 10px;

  &:hover {
    @include useTheme {
      background-color: getVar('hoverColor');
    }
  }
}
</style>
