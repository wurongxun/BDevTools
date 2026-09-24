<template>
  <div class="help-container">
    <!-- 顶部固定条：章节导航。住在滚动容器之外，正文滚动时保持不动。 -->
    <nav class="help-topbar" aria-label="帮助章节">
      <div class="help-tablist" role="tablist" aria-orientation="horizontal">
        <button
          v-for="section in SECTIONS"
          :key="section.id"
          type="button"
          class="help-tab"
          role="tab"
          :aria-selected="activeSection === section.id"
          :class="{ 'is-active': activeSection === section.id }"
          @click="scrollToSection(section.id)"
        >
          {{ section.title }}
        </button>
      </div>
    </nav>

    <!-- 正文：唯一滚动容器 -->
    <div ref="scrollEl" class="help-scroll">
      <!-- 产品介绍 -->
      <section id="product-intro" class="help-section">
        <h2>产品介绍</h2>
        <div class="section-content">
          <p>
            <strong>{{ $t('appName') }}</strong>
            是一款面向开发者的桌面端工具集：写笔记、看日志、连服务器、查数据库、分析 Java 内存，
            不必在多个软件之间来回切换，所有数据默认保存在本机。应用采用「插件」结构，互不干扰。
          </p>
          <div class="features-grid">
            <div class="feature-card">
              <h3><el-icon class="feature-ic"><Folder /></el-icon>文件</h3>
              <p>文件树浏览、代码与 Markdown 编辑；超过 1MB 的大文件自动降级为只读虚拟滚动。</p>
            </div>
            <div class="feature-card">
              <h3><el-icon class="feature-ic"><DataLine /></el-icon>日志</h3>
              <p>GB 级日志分块懒加载，支持关键词 / 正则检索、级别配色、自定义高亮与堆栈识别。</p>
            </div>
            <div class="feature-card">
              <h3><el-icon class="feature-ic"><Connection /></el-icon>SSH 连接</h3>
              <p>多标签 SSH 终端与 SFTP 文件传输，连接可保存复用。</p>
            </div>
            <div class="feature-card">
              <h3><el-icon class="feature-ic"><Coin /></el-icon>数据库</h3>
              <p>多数据库连接与库表浏览，SQL 编写执行、结果分页查看，可导出 CSV / Excel。</p>
            </div>
            <div class="feature-card">
              <h3><el-icon class="feature-ic"><DataAnalysis /></el-icon>内存分析</h3>
              <p>Java 堆转储（.hprof / .phd）解析：直方图、泄漏检测（支配树 + Retained Heap）、GC Roots 引用追踪。</p>
            </div>
          </div>
        </div>
      </section>

      <!-- 关于我们 -->
      <section id="company-info" class="help-section">
        <h2>关于我们</h2>
        <div class="section-content">

        </div>
      </section>

      <!-- 操作指南 -->
      <section id="user-guide" class="help-section">
        <h2>操作指南</h2>
        <div class="section-content">
          <div class="guide-step">
            <h3>1. 创建新笔记</h3>
            <p>点击主界面的"新建笔记"按钮，即可开始创建您的第一篇笔记。</p>
          </div>
          <div class="guide-step">
            <h3>2. 编辑与格式化</h3>
            <p>使用 Markdown 语法进行内容编辑，支持标题、列表、代码块、链接等常用格式。</p>
          </div>
          <div class="guide-step">
            <h3>3. 保存与管理</h3>
            <p>笔记会自动保存，您可以在侧边栏的笔记列表中查看和管理所有笔记。</p>
          </div>
          <div class="guide-step">
            <h3>4. 导出分享</h3>
            <p>点击"导出"按钮，将您的笔记导出为多种格式，方便分享给他人。</p>
          </div>
        </div>
      </section>

      <!-- 常见问题 -->
      <section id="faq" class="help-section">
        <h2>常见问题解答</h2>

      </section>

      <!-- 问题解决指南 -->
      <section id="troubleshooting" class="help-section">
        <h2>问题解决指南</h2>
        <div class="section-content">
          <div class="troubleshooting-item">
            <h3>应用启动缓慢</h3>
            <div class="solution">
              <p><strong>可能原因：</strong>系统资源不足或缓存文件过多</p>
              <p><strong>解决方案：</strong></p>
              <ol>
                <li>关闭不必要的后台程序以释放系统资源</li>
                <li>清理应用缓存：设置 &gt; 存储 &gt; 清理缓存</li>
                <li>重启应用</li>
              </ol>
            </div>
          </div>
          <div class="troubleshooting-item">
            <h3>内容显示异常</h3>
            <div class="solution">
              <p><strong>可能原因：</strong>渲染引擎问题或文件损坏</p>
              <p><strong>解决方案：</strong></p>
              <ol>
                <li>刷新页面或重启应用</li>
                <li>检查 Markdown 语法是否正确</li>
                <li>尝试在其他编辑器中打开文件</li>
              </ol>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
// 插件介绍卡片图标：与 src/plugins/*/index.ts 中各插件图标一致
import { Coin, Connection, DataAnalysis, DataLine, Folder } from '@element-plus/icons-vue'

/**
 * 章节清单 = 顶部固定条的页签 = 正文各 <section id>。
 *
 * 三处必须同源：此前页签是手写的 <li> 列表、章节是手写的 <section>，
 * 增删章节要改两个地方，漏改就会点空。
 */
const SECTIONS: ReadonlyArray<{ id: string; title: string }> = [
  { id: 'product-intro', title: '产品介绍' },
  { id: 'company-info', title: '关于我们' },
  { id: 'user-guide', title: '操作指南' },
  { id: 'faq', title: '常见问题' },
  { id: 'troubleshooting', title: '问题解决' },
]

const activeSection = ref(SECTIONS[0].id)
/** 正文滚动容器（也是 IntersectionObserver 的 root） */
const scrollEl = ref<HTMLElement | null>(null)
let observer: IntersectionObserver | null = null
/** 点击页签平滑滚动期间，忽略观察器上报（否则中途经过的章节会把高亮抢走） */
let lockUntil = 0

function scrollToSection(sectionId: string) {
  activeSection.value = sectionId
  lockUntil = Date.now() + 600
  const element = document.getElementById(sectionId)
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

onMounted(() => {
  const root = scrollEl.value
  if (!root) return

  // root 必须是本组件的滚动容器：默认 root=viewport 时，容器内部滚动不会产生交叉变化，
  // 高亮会一直停在进页面时那一节。
  // 判定区取「容器顶部 30% 条带」（rootMargin -70% + threshold 0）而不是阈值法：
  // 章节高度常常超过容器高度，阈值 0.5 这种比例在此类页面上永远不会达成。
  observer = new IntersectionObserver(
    entries => {
      if (Date.now() < lockUntil) return
      const visible = entries
        .filter(e => e.isIntersecting)
        .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top)
      if (visible.length) {
        activeSection.value = visible[0].target.id
      }
    },
    { root, rootMargin: '0px 0px -70% 0px', threshold: 0 },
  )

  SECTIONS.forEach(section => {
    const element = document.getElementById(section.id)
    if (element) observer?.observe(element)
  })
})

onBeforeUnmount(() => {
  observer?.disconnect()
  observer = null
})
</script>

<style lang="scss" scoped>
@use '@/styles/theme.scss' as theme;
@use '@/styles/mixin.scss' as mixin;

.help-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/**
 * 顶部固定条：章节导航。
 * 与设置页其它分区标题同一视觉基准（40px 高 + 下边框），
 * 住在滚动容器**之外**，所以正文滚到底也不会被带走。
 */
.help-topbar {
  flex: none;
  height: 40px;
  display: flex;
  align-items: stretch;
  padding: 0 var(--space-5);
  box-sizing: border-box;
  overflow-x: auto;
  overflow-y: hidden;
  @include mixin.scrollbar();

  @include theme.useTheme {
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

// 页签横向排列；窗口变窄时靠 .help-topbar 的横向滚动而不是换行（换行会把固定条顶高）
.help-tablist {
  display: flex;
  align-items: stretch;
  gap: var(--space-5);
  flex: none;
}

.help-tab {
  position: relative;
  padding: 0;
  border: 0;
  background: transparent;
  font-family: inherit;
  font-size: var(--font-size-md);
  line-height: 1;
  white-space: nowrap;
  cursor: pointer;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  &:hover {
    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }

  &.is-active {
    font-weight: 600;

    @include theme.useTheme {
      color: theme.getVar('textColor');
    }

    // 底部 2px 指示条，压在固定条的下边框上（与插件分区页签同一套语言）
    &::after {
      content: '';
      position: absolute;
      left: 0;
      right: 0;
      bottom: 0;
      height: 2px;

      @include theme.useTheme {
        background: theme.getVar('focusRing');
      }
    }
  }
}

// 正文滚动容器：唯一滚动源（外层 .settings-body 在这个分区是 is-fill，不滚）
.help-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: var(--space-4) var(--space-5) var(--space-6);
  box-sizing: border-box;
  scroll-behavior: smooth;
  @include mixin.scrollbar();
}

.help-section {
  margin-bottom: var(--space-4);
  padding: var(--space-4);
  border-radius: var(--radius-md);
  box-sizing: border-box;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
  }

  &:last-child {
    margin-bottom: 0;
  }
}

// 章节标题：与 app 的面板标题同档（14px / 600），不再用 1.8rem 这类网页级字号
.help-section h2 {
  margin: 0 0 var(--space-3);
  padding-bottom: var(--space-2);
  font-size: var(--font-size-lg);
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('textColor');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.section-content {
  font-size: var(--font-size-base);
  line-height: 1.7;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }

  p {
    margin: 0 0 var(--space-2);
  }

  strong {
    @include theme.useTheme {
      color: theme.getVar('textColor');
    }
  }
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--space-3);
  margin-top: var(--space-3);
}

.feature-card {
  padding: var(--space-3);
  border-radius: var(--radius-md);
  box-sizing: border-box;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
    border: 1px solid theme.getVar('borderColor');
  }

  p {
    margin: 0;
    font-size: var(--font-size-base);
    line-height: 1.6;
  }
}

.feature-card h3 {
  margin: 0 0 var(--space-1);
  font-size: var(--font-size-md);
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 6px;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

/* 卡片标题前的内置插件图标 */
.feature-ic {
  color: var(--el-color-primary);
  font-size: var(--font-size-base);
  flex-shrink: 0;
}

.guide-step {
  margin-bottom: var(--space-3);

  &:last-child {
    margin-bottom: 0;
  }

  p {
    margin: 0;
  }
}

.guide-step h3 {
  margin: 0 0 var(--space-1);
  font-size: var(--font-size-md);
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.faq-item,
.troubleshooting-item {
  margin-bottom: var(--space-3);
  padding: var(--space-3);
  border-radius: var(--radius-md);
  box-sizing: border-box;

  @include theme.useTheme {
    background-color: theme.getVar('sidebarBg');
  }

  &:last-child {
    margin-bottom: 0;
  }
}

.faq-item h3,
.troubleshooting-item h3 {
  margin: 0 0 var(--space-1);
  font-size: var(--font-size-md);
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.solution {
  p {
    margin: 0 0 var(--space-1);

    &:last-child {
      margin-bottom: 0;
    }
  }

  ol {
    margin: var(--space-1) 0 0;
    padding-left: var(--space-5);
  }

  li {
    margin-bottom: var(--space-1);

    &:last-child {
      margin-bottom: 0;
    }
  }
}
</style>
