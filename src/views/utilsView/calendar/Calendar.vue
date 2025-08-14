<script lang="ts" setup>
import { ref } from 'vue'
const value = ref(new Date())
</script>
<template>
    <div class="calendar">
        <el-calendar v-model="value" />
    </div>
   
</template>
<style lang="scss">
.calendar {
    width: 100%;
    height: 100%;
    overflow: hidden;
    .el-calendar {
        width: 100%;
        height: 100%;
        overflow: hidden;
    }
}
</style>