<template>
  <div class="help-container">
    <div class="help-content">
      <!-- 侧边导航 -->
      <div class="help-sidebar">
        <ul class="nav-menu">
          <li 
            v-for="section in sections" 
            :key="section.id"
            :class="{ active: activeSection === section.id }"
            @click="scrollToSection(section.id)"
          >
            {{ section.title }}
          </li>
        </ul>
      </div>

      <!-- 主要内容区域 -->
      <div class="help-main">
        <!-- 产品介绍 -->
        <section id="product-intro" class="help-section">
          <h2>产品介绍</h2>
          <div class="section-content">
            <p>
              <strong>{{ $t('appName') }}</strong>
              是一款面向开发者的桌面端工具集：写笔记、看日志、连服务器、查数据库、分析 Java 内存，
              不必在多个软件之间来回切换,所有数据默认保存在本机。应用采用「插件」结构，互不干扰。
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

        <!-- 公司介绍 -->
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
                  <li>清理应用缓存：设置 > 存储 > 清理缓存</li>
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
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
// 插件介绍卡片图标：与 src/plugins/*/index.ts 中各插件图标一致
import { Coin, Connection, DataAnalysis, DataLine, Folder } from '@element-plus/icons-vue'
const { t } = useI18n();
const activeSection = ref('product-intro')

const sections = [
  { id: 'product-intro', title: '产品介绍' },
  { id: 'company-info', title: '关于我们' },
  { id: 'user-guide', title: '操作指南' },
  { id: 'faq', title: '常见问题' },
  { id: 'troubleshooting', title: '问题解决' }
]

const scrollToSection = (sectionId) => {
  activeSection.value = sectionId
  const element = document.getElementById(sectionId)
  if (element) {
    element.scrollIntoView({ behavior: 'smooth', block: 'start' })
  }
}

onMounted(() => {
  // 监听滚动事件，高亮当前可见的章节
  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        activeSection.value = entry.target.id
      }
    })
  }, { threshold: 0.5 })

  sections.forEach(section => {
    const element = document.getElementById(section.id)
    if (element) {
      observer.observe(element)
    }
  })
})
</script>

<style scoped>
.help-container {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.help-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.help-sidebar {
  width: 250px;
  /* background: white; */
  border-right: 1px solid #e1e4e8;
  overflow-y: auto;
  padding: 20px 0;
  box-shadow: 2px 0 5px rgba(0, 0, 0, 0.05);
}

.nav-menu {
  list-style: none;
  padding: 0;
  margin: 0;
}

.nav-menu li {
  padding: 12px 20px;
  cursor: pointer;
  transition: all 0.3s;
  border-left: 3px solid transparent;
  color: #333;
  font-size: 1rem;
}

.nav-menu li:hover {
  /* background-color: #f0f4f8; */
}

.nav-menu li.active {
  border-left-color: #409eff;
  /* background-color: #ecf5ff; */
  color: #409eff;
  font-weight: 500;
}

.help-main {
  flex: 1;
  overflow-y: auto;
  padding: 30px;
  scrollbar-width: none;
}

.help-main::-webkit-scrollbar {
  width: 0;
  background: transparent;
}

.help-section {
  margin-bottom: 40px;
  /* background: white; */
  border-radius: var(--radius-xl);
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.05);
  padding: 25px;
}

.help-section h2 {
  color: #333;
  margin-top: 0;
  margin-bottom: 20px;
  padding-bottom: 15px;
  border-bottom: 2px solid #f0f4f8;
  font-size: 1.8rem;
  font-weight: 500;
}

.section-content {
  line-height: 1.7;
  color: #555;
}

.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 20px;
  margin-top: 20px;
}

.feature-card {
  /* background: #f8f9fa; */
  border-radius: var(--radius-lg);
  padding: 20px;
  transition: transform 0.3s, box-shadow 0.3s;
}

.feature-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 5px 15px rgba(0, 0, 0, 0.1);
}

.feature-card h3 {
  margin-top: 0;
  color: #333;
  font-size: 1.2rem;
  display: flex;
  align-items: center;
  gap: 6px;
}

/* 卡片标题前的内置插件图标 */
.feature-ic {
  color: var(--el-color-primary);
  font-size: var(--font-size-base);
  flex-shrink: 0;
}

.guide-step {
  margin-bottom: 25px;
}

.guide-step h3 {
  /* color: #409eff; */
  margin-bottom: 10px;
  font-size: 1.3rem;
}

.faq-item, .troubleshooting-item {
  margin-bottom: 25px;
  padding: 20px;
  /* background: #f8f9fa; */
  border-radius: var(--radius-lg);
}

.faq-item h3, .troubleshooting-item h3 {
  margin-top: 0;
  /* color: #333; */
}

.solution ol {
  padding-left: 20px;
}

.solution li {
  margin-bottom: 8px;
}

</style>