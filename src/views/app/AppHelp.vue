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
            <p>{{ $t('appName')}} 是一款现代化的 Markdown 笔记应用，专为开发者和写作爱好者设计。它结合了简洁的界面与强大的功能，让您能够专注于内容创作。</p>
            <div class="features-grid">
              <div class="feature-card">
                <h3>实时预览</h3>
                <p>边写边看效果，所见即所得的编辑体验</p>
              </div>
              <div class="feature-card">
                <h3>云端同步</h3>
                <p>多设备数据同步，随时随地访问您的笔记</p>
              </div>
              <div class="feature-card">
                <h3>语法高亮</h3>
                <p>支持多种编程语言的代码高亮显示</p>
              </div>
              <div class="feature-card">
                <h3>导出功能</h3>
                <p>支持导出为 PDF、HTML、Word 等多种格式</p>
              </div>
            </div>
          </div>
        </section>

        <!-- 公司介绍 -->
        <section id="company-info" class="help-section">
          <h2>关于我们</h2>
          <div class="section-content">
            <p>我们是一家专注于开发高效工具软件的科技公司，致力于为用户提供简洁、强大且易用的生产力工具。自成立以来，我们始终坚持以用户需求为导向，不断创新和优化产品。</p>
            <p>我们的团队由一群热爱技术、追求卓越的开发者和设计师组成，秉承"让创作更简单"的使命，持续为用户创造价值。</p>
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
          <div class="section-content">
            <div class="faq-item">
              <h3>Q: 如何恢复误删的笔记？</h3>
              <p>A: 您可以在"回收站"中找到最近删除的笔记，并选择恢复或永久删除。</p>
            </div>
            <div class="faq-item">
              <h3>Q: 支持哪些格式的文件导入？</h3>
              <p>A: 目前支持 .md、.txt、.html 格式的文件导入，更多格式正在开发中。</p>
            </div>
            <div class="faq-item">
              <h3>Q: 如何在多设备间同步数据？</h3>
              <p>A: 登录您的账户后，数据会自动在所有设备间同步，确保一致性。</p>
            </div>
            <div class="faq-item">
              <h3>Q: 是否支持离线使用？</h3>
              <p>A: 是的，应用支持离线编辑，联网后会自动同步数据。</p>
            </div>
            <div class="faq-item">
              <h3>Q: 忘记密码怎么办？</h3>
              <p>A: 在登录页面点击"忘记密码"，按照提示操作即可重置密码。</p>
            </div>
          </div>
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
              <h3>同步失败</h3>
              <div class="solution">
                <p><strong>可能原因：</strong>网络连接问题或账户异常</p>
                <p><strong>解决方案：</strong></p>
                <ol>
                  <li>检查网络连接是否正常</li>
                  <li>确认账户登录状态</li>
                  <li>尝试退出账户后重新登录</li>
                  <li>联系技术支持 support@mdnote.com</li>
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
  border-radius: 8px;
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
  border-radius: 6px;
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
  border-radius: 6px;
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

@media (max-width: 768px) {
  .help-content {
    flex-direction: column;
  }
  
  .help-sidebar {
    width: 100%;
    border-right: none;
    border-bottom: 1px solid #e1e4e8;
    max-height: 200px;
  }
  
  .help-main {
    padding: 15px;
  }
  
  .features-grid {
    grid-template-columns: 1fr;
  }
}
</style>