<template>
  <div class="help-container">
    <!-- 顶部固定条：章节导航。住在滚动容器之外，正文滚动时保持不动。 -->
    <nav class="help-topbar" :aria-label="$t('settingsPlugin.help.navAria')">
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
          {{ $t(section.key) }}
        </button>
      </div>
    </nav>

    <!-- 正文：唯一滚动容器 -->
    <div ref="scrollEl" class="help-scroll">
      <!-- 产品介绍 -->
      <section id="product-intro" class="help-section">
        <h2>{{ $t('settingsPlugin.help.productIntro') }}</h2>
        <div class="section-content">
          <p>
            {{ $t('settingsPlugin.help.productIntroDesc', { appName: $t('appName') }) }}
          </p>
          <div class="features-grid">
            <div class="feature-card">
              <h3><el-icon class="feature-ic"><Folder /></el-icon>{{ $t('settingsPlugin.help.featFile') }}</h3>
              <p>{{ $t('settingsPlugin.help.featFileDesc') }}</p>
            </div>
            <div class="feature-card">
              <h3><el-icon class="feature-ic"><DataLine /></el-icon>{{ $t('settingsPlugin.help.featLog') }}</h3>
              <p>{{ $t('settingsPlugin.help.featLogDesc') }}</p>
            </div>
            <div class="feature-card">
              <h3><el-icon class="feature-ic"><Connection /></el-icon>{{ $t('settingsPlugin.help.featSsh') }}</h3>
              <p>{{ $t('settingsPlugin.help.featSshDesc') }}</p>
            </div>
            <div class="feature-card">
              <h3><el-icon class="feature-ic"><Coin /></el-icon>{{ $t('settingsPlugin.help.featDb') }}</h3>
              <p>{{ $t('settingsPlugin.help.featDbDesc') }}</p>
            </div>
            <div class="feature-card">
              <h3><el-icon class="feature-ic"><DataAnalysis /></el-icon>{{ $t('settingsPlugin.help.featMemory') }}</h3>
              <p>{{ $t('settingsPlugin.help.featMemoryDesc') }}</p>
            </div>
          </div>
        </div>
      </section>

      <!-- 关于我们 -->
      <section id="company-info" class="help-section">
        <h2>{{ $t('settingsPlugin.help.aboutUs') }}</h2>
        <div class="section-content">

        </div>
      </section>

      <!-- 操作指南 -->
      <section id="user-guide" class="help-section">
        <h2>{{ $t('settingsPlugin.help.guide') }}</h2>
        <div class="section-content">
          <div class="guide-step">
            <h3>{{ $t('settingsPlugin.help.guide1Title') }}</h3>
            <p>{{ $t('settingsPlugin.help.guide1Desc') }}</p>
          </div>
          <div class="guide-step">
            <h3>{{ $t('settingsPlugin.help.guide2Title') }}</h3>
            <p>{{ $t('settingsPlugin.help.guide2Desc') }}</p>
          </div>
          <div class="guide-step">
            <h3>{{ $t('settingsPlugin.help.guide3Title') }}</h3>
            <p>{{ $t('settingsPlugin.help.guide3Desc') }}</p>
          </div>
          <div class="guide-step">
            <h3>{{ $t('settingsPlugin.help.guide4Title') }}</h3>
            <p>{{ $t('settingsPlugin.help.guide4Desc') }}</p>
          </div>
        </div>
      </section>

      <!-- 常见问题 -->
      <section id="faq" class="help-section">
        <h2>{{ $t('settingsPlugin.help.faq') }}</h2>

      </section>

      <!-- 问题解决指南 -->
      <section id="troubleshooting" class="help-section">
        <h2>{{ $t('settingsPlugin.help.troubleshooting') }}</h2>
        <div class="section-content">
          <div class="troubleshooting-item">
            <h3>{{ $t('settingsPlugin.help.trobSlowTitle') }}</h3>
            <div class="solution">
              <p><strong>{{ $t('settingsPlugin.help.trobSlowCause') }}</strong></p>
              <p><strong>{{ $t('settingsPlugin.help.solution') }}</strong></p>
              <ol>
                <li>{{ $t('settingsPlugin.help.trobSlowStep1') }}</li>
                <li>{{ $t('settingsPlugin.help.trobSlowStep2') }}</li>
                <li>{{ $t('settingsPlugin.help.trobSlowStep3') }}</li>
              </ol>
            </div>
          </div>
          <div class="troubleshooting-item">
            <h3>{{ $t('settingsPlugin.help.trobDisplayTitle') }}</h3>
            <div class="solution">
              <p><strong>{{ $t('settingsPlugin.help.trobDisplayCause') }}</strong></p>
              <p><strong>{{ $t('settingsPlugin.help.solution') }}</strong></p>
              <ol>
                <li>{{ $t('settingsPlugin.help.trobDisplayStep1') }}</li>
                <li>{{ $t('settingsPlugin.help.trobDisplayStep2') }}</li>
                <li>{{ $t('settingsPlugin.help.trobDisplayStep3') }}</li>
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
const SECTIONS: ReadonlyArray<{ id: string; key: string }> = [
  { id: 'product-intro', key: 'settingsPlugin.help.productIntro' },
  { id: 'company-info', key: 'settingsPlugin.help.aboutUs' },
  { id: 'user-guide', key: 'settingsPlugin.help.guide' },
  { id: 'faq', key: 'settingsPlugin.help.faq' },
  { id: 'troubleshooting', key: 'settingsPlugin.help.troubleshooting' },
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
@use '@/styles/theme' as theme;
@use '@/styles/mixin' as mixin;

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
