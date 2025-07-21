// ```我理解您希望将组件转换为 TypeScript (TSX) 格式。以下是完整的 TypeScript 版本实现：
// typescript|CODE_EDIT_BLOCK|f:\VSCodeProject\md-note\src\components\jsx\TreeNode.tsx```;
import { NotebookInfo } from "@/entity/entitys";
import { ElMenuItem, ElSubMenu } from "element-plus";
// 定义 NotebookInfo 接口
import { defineComponent, PropType, h } from "vue";
import styles from "./TreeNode.module.css";
// 添加全局声明以解决JSX类型问题
declare global {
  namespace JSX {
    interface IntrinsicElements {
      [key: string]: any; // 为所有JSX元素提供任意类型
    }
  }
}

const TreeNode = defineComponent({
  name: "TreeNode",
  props: {
    node: {
      type: Object as PropType<NotebookInfo>,
      required: true,
    },
    modelValue: {
      type: String,
      default: "",
    },
  },
  emits: ["update:modelValue", "node-click"],
  setup(props, { emit }) {
    const handleClick = (node: NotebookInfo) => {
      emit("update:modelValue", node.id);
      emit("node-click", node);
    };

    return () => {
      const node = props.node;
      if (!node || !node.id) return null;

      const isSelected = props.modelValue === node.id;
      const itemClass = {
        "selected-node": isSelected,
        "folder-node": node.children && node.children.length > 0,
        "file-node": !node.children || node.children.length === 0,
      };

      if (node.children && node.children.length > 0) {
        return (
          <ElSubMenu
            index={node.id}
            key={node.id}
            class={itemClass}
            v-slots={{
              title: () => (
                <span
                  onClick={(e) => {
                    e.stopPropagation();
                    handleClick(node);
                  }}
                >
                  {node.name}
                </span>
              ),
              default: () =>
                node.children!.map((child) => (
                  <TreeNode
                    node={child}
                    key={child.id}
                    modelValue={props.modelValue}
                    onNodeClick={(n: NotebookInfo) => emit("node-click", n)}
                  />
                )),
            }}
          />
        );
      } else {
        return (
          <ElMenuItem
            index={node.id}
            key={node.id}
            class={itemClass}
            onClick={() => handleClick(node)}
          >
            <span>{node.name}</span>
          </ElMenuItem>
        );
      }
    };
  },
});

export default TreeNode;
