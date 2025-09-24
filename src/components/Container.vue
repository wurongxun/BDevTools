<!-- 
  项目常用的容器，只是实现了一些简单的样式，
  其实使用 scss 的 @mixin 也能达到一样的效果
  但是基于代码可扩展性的考量，还是使用了组件的方式
  后续可以通过  props  来实现扩展
 -->

<template>
  <div class="__common-component-container">
    <slot></slot>
  </div>
</template>

<style scoped lang="scss">
.__common-component-container {
  margin: 2px;
  min-height: calc(100% - 30px);
  padding: 30px 20px 20px 20px;
  border-radius: 5px;

  @include useTheme {
    background-color: getVar('bgColor');
    box-shadow: 0 0 5px getVar('shadowColor');
  }
}
</style>
