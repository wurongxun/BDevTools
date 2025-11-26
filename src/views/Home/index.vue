<template>
  <container>
    <section class="echarts-box">
      <echarts-data></echarts-data>
    </section>
  </container>
</template>

<script setup lang="ts" name="HomePage">
import { onMounted, ref, reactive, toRefs } from 'vue'
import Container from '@/components/Container.vue'
import CountUp from '@/components/CountUp.vue'
import EchartsData from './components/EchartsData.vue'

const numState = reactive({
  waitNum: 458,
  overNum: 3546,
  passNum: 8851,
  rejectNum: 356
})
const { waitNum, overNum, passNum, rejectNum } = toRefs(numState)
const countUpRef = ref()
onMounted(() => { })

const initCount = () => {
  countUpRef.value.initCount()
}
</script>

<style scoped lang="scss">
@use '@/styles/variables.scss' as variables;

.num-card {
  text-align: center;
  font-size: 24px;
  font-weight: bold;
  display: flex;
  justify-content: space-between;
  align-items: center;
  width: 100%;

  .num-wrapper {
    flex: 1;
    text-align: left;
    padding: 0 8px;

    .num-label {
      color: #999;
      font-size: 16px;
      display: block;
      margin-bottom: 12px;
    }
  }

  .svg-icon {
    font-size: 80px;
    padding: 10px;
  }

  .count-wrapper {
    display: block;
  }

  &.wait-num {
    color: #67c23a;

    svg {
      &:hover {
        fill: #fff;
        background-color: #67c23a;
        cursor: pointer;
      }
    }
  }

  &.over-num {
    color: #e6a23c;

    svg {
      &:hover {
        fill: #fff;
        background-color: #e6a23c;
        cursor: pointer;
      }
    }
  }

  &.pass-num {
    color: variables.$theme-color;

    svg {
      &:hover {
        fill: #fff;
        background-color: variables.$theme-color;
        cursor: pointer;
      }
    }
  }

  &.reject-num {
    color: #f56c6c;

    svg {
      fill: #f56c6c;

      &:hover {
        fill: #fff;
        background-color: #f56c6c;
        cursor: pointer;
      }
    }
  }
}

.echarts-box {
  margin-top: 20px;
}
</style>
