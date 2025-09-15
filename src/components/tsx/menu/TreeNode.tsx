import { NotebookInfo } from '@/entity/entitys'
import { ElMenuItem, ElSubMenu } from 'element-plus'
import { defineComponent, PropType, h } from 'vue'
import { Folder, Document } from '@element-plus/icons-vue'
import MarkDownColorful from '@/components/Icon/MarkDownColorful.vue'
import PDF from '@/components/Icon/PDF.vue'
import FlodeColorful from '@/components/Icon/FlodeColorful.vue'
import { RouteRecordRaw } from 'vue-router'

const TreeNode = defineComponent({
  name: 'TreeNode',
  props: {
    node: {
      type: Object as PropType<RouteRecordRaw>,
      required: true
    },
    modelValue: {
      type: String,
      default: ''
    }
  },
  emits: ['update:modelValue', 'node-click'],
  setup(props, { emit }) {
    const handleClick = (node: RouteRecordRaw) => {
      emit('update:modelValue', node.meta?.id)
      emit('node-click', node)
    }

    return () => {
      const node = props.node
      console.log('Rendering node:', node)

      if (!node || !node.meta?.id) return null

      const isSelected = props.modelValue === node.meta.id
      const itemClass = {
        'selected-node': isSelected,
        'folder-node': node.children && node.children.length > 0,
        'file-node': !node.children || node.children.length === 0
      }

      if (node.children && node.children.length > 0) {
        // 文件夹节点
        return (
          <ElSubMenu
            index={node.meta.id}
            key={node.meta.id}
            class={itemClass}
            v-slots={{
              title: () => (
                <>
                  <el-icon>
                    <FlodeColorful />
                  </el-icon>
                  <span
                    onClick={(e: MouseEvent) => {
                      e.stopPropagation()
                      handleClick(node)
                    }}
                  >
                    {node.name}
                  </span>
                </>
              ),
              default: () =>
                node.children!.map((child) => (
                  <TreeNode
                    node={child}
                    key={child.meta?.id}
                    modelValue={props.modelValue}
                    onUpdate:modelValue={(val: string) => emit('update:modelValue', val)}
                    onNodeClick={(n: RouteRecordRaw) => emit('node-click', n)}
                  />
                ))
            }}
          />
        )
      } else {
        // 文件节点
        return (
          <ElMenuItem
            index={node.meta.id}
            key={node.meta.id}
            class={itemClass}
            onClick={() => handleClick(node)}
          >
            <el-icon>{node.meta?.type === 'pdf' ? <PDF /> : <MarkDownColorful />}</el-icon>
            <span>{node.name}</span>
          </ElMenuItem>
        )
      }
    }
  }
})

export default TreeNode
