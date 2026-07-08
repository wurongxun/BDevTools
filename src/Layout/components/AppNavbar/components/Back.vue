<template>
  <div class="scale-icon" @click="back">
    <el-icon size="20px">
      <Back />
    </el-icon>
  </div>
</template>

<script lang="ts" setup>
import useStore from '@/store'
import { storeToRefs } from 'pinia'
import { Back, Minus, Close } from "@element-plus/icons-vue";
//import router from '@/router';
import { ITagItem } from '@/Types/TagView';
import { useRouter, useRoute } from 'vue-router'

const { system } = useStore()
const { isCollapse } = storeToRefs(system)
const router = useRouter()
const route = useRoute()
const back = async function () {
  router.back();

};
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
