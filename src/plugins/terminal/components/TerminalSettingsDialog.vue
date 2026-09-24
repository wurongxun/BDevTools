<template>
  <!-- 终端外观设置：字体 / 光标 / 回滚 / 配色（持久化到 plugins/terminal.json） -->
  <GlobalDialog
    :model-value="store.settingsOpen"
    title="终端外观设置"
    width="680px"
    @update:model-value="onVisibleChange"
  >
    <div class="tsd-body">
      <section class="tsd-group">
        <p class="tsd-group-title">显示</p>
        <div class="tsd-group-body">
          <div class="tsd-item">
            <div class="tsd-info">
              <span class="tsd-label">字体族</span>
              <span class="tsd-desc">
                必须等宽；中文建议选带等宽汉字的字体（Sarasa Mono SC / Microsoft YaHei Mono），否则中文会错位。
              </span>
            </div>
            <el-select
              v-model="store.appearance.fontFamily"
              class="tsd-select"
              size="small"
              filterable
              allow-create
              default-first-option
              placeholder="选择或输入字体族"
            >
              <el-option v-for="f in FONT_PRESETS" :key="f.value" :label="f.label" :value="f.value" />
            </el-select>
          </div>

          <div class="tsd-item">
            <div class="tsd-info">
              <span class="tsd-label">字号</span>
              <span class="tsd-desc">终端字符的像素大小</span>
            </div>
            <div class="tsd-value">
              <el-input-number v-model="store.appearance.fontSize" class="tsd-num" size="small" :min="8" :max="32" :step="1" />
              <span class="tsd-unit">px</span>
            </div>
          </div>

          <div class="tsd-item">
            <div class="tsd-info">
              <span class="tsd-label">行高</span>
              <span class="tsd-desc">1.0 为单倍行距，密排可试 1.0–1.1</span>
            </div>
            <el-input-number
              v-model="store.appearance.lineHeight"
              class="tsd-num"
              size="small"
              :min="0.8"
              :max="2.4"
              :step="0.05"
              :precision="2"
            />
          </div>

          <div class="tsd-item">
            <div class="tsd-info">
              <span class="tsd-label">配色方案</span>
              <span class="tsd-desc">「跟随应用主题」会随亮色 / 暗色模式自动切换</span>
            </div>
            <el-select v-model="store.appearance.colorScheme" class="tsd-select tsd-select--narrow" size="small">
              <el-option v-for="s in COLOR_SCHEMES" :key="s.id" :label="s.label" :value="s.id" />
            </el-select>
          </div>
        </div>
      </section>

      <section class="tsd-group">
        <p class="tsd-group-title">光标与交互</p>
        <div class="tsd-group-body">
          <div class="tsd-item">
            <div class="tsd-info">
              <span class="tsd-label">光标</span>
              <span class="tsd-desc">形状与是否闪烁</span>
            </div>
            <div class="tsd-value">
              <el-radio-group v-model="store.appearance.cursorStyle" size="small">
                <el-radio-button value="block">方块</el-radio-button>
                <el-radio-button value="bar">竖线</el-radio-button>
                <el-radio-button value="underline">下划线</el-radio-button>
              </el-radio-group>
              <span class="tsd-label tsd-label--inline">闪烁</span>
              <el-switch v-model="store.appearance.cursorBlink" size="small" />
            </div>
          </div>

          <div class="tsd-item">
            <div class="tsd-info">
              <span class="tsd-label">选中即复制</span>
              <span class="tsd-desc">X11 习惯；开启后鼠标选中就写剪贴板</span>
            </div>
            <el-switch v-model="store.appearance.copyOnSelect" size="small" />
          </div>
        </div>
      </section>

      <section class="tsd-group">
        <p class="tsd-group-title">缓冲区与面板</p>
        <div class="tsd-group-body">
          <div class="tsd-item">
            <div class="tsd-info">
              <span class="tsd-label">回滚行数</span>
              <span class="tsd-desc">缓冲区上限，越大越占内存（每行约 100 字节）</span>
            </div>
            <div class="tsd-value">
              <el-input-number
                v-model="store.appearance.scrollback"
                class="tsd-num tsd-num--wide"
                size="small"
                :min="0"
                :max="200000"
                :step="1000"
              />
              <span class="tsd-unit">行</span>
            </div>
          </div>

          <div class="tsd-item">
            <div class="tsd-info">
              <span class="tsd-label">面板高度</span>
              <span class="tsd-desc">底部终端面板的高度</span>
            </div>
            <div class="tsd-value">
              <el-input-number
                v-model="store.appearance.panelHeight"
                class="tsd-num"
                size="small"
                :min="80"
                :max="1600"
                :step="20"
              />
              <span class="tsd-unit">px</span>
            </div>
          </div>
        </div>
      </section>

      <section class="tsd-group">
        <p class="tsd-group-title">快捷键</p>
        <div class="tsd-group-body">
          <div class="tsd-keys">
            <div v-for="k in SHORTCUTS" :key="k.keys" class="tsd-key">
              <code>{{ k.keys }}</code>
              <span>{{ k.desc }}</span>
            </div>
          </div>
        </div>
      </section>
    </div>

    <template #footer>
      <app-button size="small" @click="store.resetAppearance()">恢复默认</app-button>
      <app-button size="small" type="primary" @click="store.settingsOpen = false">完成</app-button>
    </template>
  </GlobalDialog>
</template>

<script setup lang="ts" name="TerminalSettingsDialog">
import GlobalDialog from '@/components/GlobalDialog.vue'
import { COLOR_SCHEMES } from '../theme'
import { useTerminalStore } from '../store'

const store = useTerminalStore()

/** 字体预设（值即 CSS font-family 栈） */
const FONT_PRESETS = [
  {
    label: 'Cascadia Mono（Windows Terminal 默认）',
    value: "'Cascadia Mono', 'JetBrains Mono', Consolas, 'Sarasa Mono SC', 'Microsoft YaHei Mono', Menlo, Monaco, 'Courier New', monospace",
  },
  { label: 'JetBrains Mono', value: "'JetBrains Mono', 'Cascadia Mono', Consolas, monospace" },
  { label: 'Consolas', value: "Consolas, 'Cascadia Mono', monospace" },
  {
    label: 'Sarasa Mono SC（中文等宽）',
    value: "'Sarasa Mono SC', 'Cascadia Mono', Consolas, monospace",
  },
  { label: 'Menlo / Monaco（macOS）', value: "Menlo, Monaco, 'Cascadia Mono', Consolas, monospace" },
]

/** 快捷键说明（只读，放底部一组；每行一条，避免弹窗过高要滚动） */
const SHORTCUTS = [
  { keys: 'Ctrl/Cmd + Shift + C', desc: '复制选区（无选区时 Ctrl+C 照常发送 SIGINT）' },
  { keys: 'Ctrl/Cmd + V', desc: '粘贴（括号粘贴模式，多行命令不会被逐行执行）' },
  { keys: 'Ctrl/Cmd + Shift + F', desc: '查找 · Ctrl/Cmd + Shift + T 新建会话' },
  { keys: 'Ctrl/Cmd + K', desc: '清屏 · Ctrl/Cmd + A 全选缓冲' },
]

/** GlobalDialog 只有 update:modelValue / close，关闭时同步 store 状态 */
function onVisibleChange(visible: boolean) {
  store.settingsOpen = visible
}
</script>

<style scoped lang="scss">
@use '@/styles/theme.scss' as theme;

/**
 * 终端外观设置弹窗
 *
 * 沿用应用设置页的桌面式分组：**组标题在容器外**，容器只做「细边框 + 行分隔线」，
 * 每一行是「左侧标签与说明 / 右侧控件」两栏。
 *
 * 早先这里是 560px 宽的单行 flex（label + 控件 + 提示全在一行、`flex-wrap: wrap`），
 * 一遇到长文案就被挤到第二行折断，看起来非常拥挤 —— 说明性文字必须放在标签下方，
 * 不能和控件抢同一行的宽度。
 */
.tsd-body {
  display: flex;
  flex-direction: column;
  // 组间距收到 space-4：弹窗内容全部展开后总高要尽量压在 GlobalDialog 的 60vh 之内
  gap: var(--space-4);
}

.tsd-group-title {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-base);
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.tsd-group-body {
  border-radius: var(--radius-md);
  overflow: hidden;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
  }
}

.tsd-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-2) var(--space-3);

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }

  &:last-child {
    border-bottom: none;
  }
}

.tsd-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.tsd-label {
  font-size: var(--font-size-md);

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }

  &--inline {
    margin-left: var(--space-2);
    font-size: var(--font-size-sm);

    @include theme.useTheme {
      color: theme.getVar('textSecondary');
    }
  }
}

.tsd-desc {
  font-size: var(--font-size-base);
  line-height: 1.5;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

/** 控件容器：整体贴右、不随窗口拉伸（数值框拉满会显得松散） */
.tsd-value {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.tsd-select {
  width: 300px;

  &--narrow {
    width: 200px;
  }
}

.tsd-num {
  width: 120px;

  &--wide {
    width: 150px;
  }
}

.tsd-unit {
  font-size: var(--font-size-sm);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

/** 快捷键：左列等宽键位（固定宽，保证右侧说明文字左边缘对齐） */
.tsd-keys {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: var(--space-3);
}

.tsd-key {
  display: flex;
  align-items: baseline;
  gap: var(--space-4);
  font-size: var(--font-size-base);

  code {
    width: 168px;
    flex-shrink: 0;
    font-family: var(--font-family-mono);

    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }

  span {
    @include theme.useTheme {
      color: theme.getVar('textSecondary');
    }
  }
}
</style>
