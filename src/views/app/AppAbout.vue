<template>
  <div class="about-container">
    <div class="about-content">
      <div class="info-section">
        <h2>应用信息</h2>
        <div class="info-grid">
          <div class="info-item">
            <span class="info-label">应用名称</span>
            <span class="info-value">{{ data.appName || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">Tauri 版本</span>
            <span class="info-value">{{ data.tauriVersion || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">应用版本</span>
            <span class="info-value">{{ data.appVersion || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">系统架构</span>
            <span class="info-value">{{ data.archName || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">平台</span>
            <span class="info-value">{{ data.platformName || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">系统类型</span>
            <span class="info-value">{{ data.osType || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">系统版本</span>
            <span class="info-value">{{ data.osVersion || '获取中...' }}</span>
          </div>
        </div>

        <h2>联系信息</h2>
        <div class="contact-info">
          <p>
            <span class="contact-label">联系邮箱</span>
            <a href="mailto:1475316789@qq.com">1475316789@qq.com</a>
          </p>
          <p>
            <span class="contact-label">图标出处</span>
            <a href="https://www.flaticon.com/free-icon/substance_9814620" target="_blank" rel="noreferrer">
              flaticon.com/free-icon/substance_9814620
            </a>
          </p>
          <p>
            <span class="contact-label">Gitee 地址</span>
            <a href="https://gitee.com/wurongxun/BackendDevTools" target="_blank" rel="noreferrer">
              gitee.com/wurongxun/BDevTools
            </a>
          </p>
        </div>
      </div>

      <div class="coffee-section">
        <h2>请我喝杯咖啡</h2>
        <div class="coffee-content">
          <el-image alt="请我喝杯咖啡" class="coffee-image" :src="coffeeImage" />
          <p class="coffee-text">如果您觉得这个应用对您有帮助，欢迎请我喝杯咖啡！您的支持是我持续开发的动力。</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { onMounted, reactive } from 'vue'
import { getName, getTauriVersion, getVersion } from '@/core/runtime'
import { osArch, osPlatform, osType, osVersion } from '@/core/runtime'
import coffeeImage from '@/assets/coffee.jpg'

const data = reactive({
  appName: '',
  tauriVersion: '',
  appVersion: '',
  archName: '',
  platformName: '',
  osType: '',
  osVersion: '',
})

onMounted(async () => {
  try {
    data.appName = await getName()
    data.tauriVersion = await getTauriVersion()
    data.appVersion = await getVersion()
    data.archName = await osArch()
    data.platformName = await osPlatform()
    data.osType = await osType()
    data.osVersion = await osVersion()
  } catch (error) {
    console.error('获取应用信息失败:', error)
  }
})
</script>

<style lang="scss" scoped>
@use '@/styles/theme.scss' as theme;

.about-container {
  width: 100%;
}

.about-content {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  gap: var(--space-4);
  width: 100%;
  box-sizing: border-box;
}

// 两块卡片与设置页其它分组同一视觉语言：bgColor + 1px 细边框、无投影
.info-section,
.coffee-section {
  flex: 1;
  min-width: 300px;
  padding: var(--space-4);
  border-radius: var(--radius-md);
  box-sizing: border-box;

  @include theme.useTheme {
    background-color: theme.getVar('bgColor');
    border: 1px solid theme.getVar('borderColor');
  }
}

.coffee-section {
  display: flex;
  flex-direction: column;
}

// 卡片标题：与 app 面板标题同档（14px / 600）——此前用浏览器默认的 1.5em 并写死 #333，
// 暗色主题下是深灰字压深灰底，基本看不见。
h2 {
  margin: 0 0 var(--space-3);
  padding-bottom: var(--space-2);
  font-size: var(--font-size-lg);
  font-weight: 600;

  @include theme.useTheme {
    color: theme.getVar('textColor');
    border-bottom: 1px solid theme.getVar('borderColor');
  }
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: var(--space-2) var(--space-4);
  margin-bottom: var(--space-4);
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.info-label {
  font-size: var(--font-size-base);

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.info-value {
  font-size: var(--font-size-md);

  @include theme.useTheme {
    color: theme.getVar('textColor');
  }
}

.contact-info {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.contact-info p {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-2);
  margin: 0;
  font-size: var(--font-size-base);
  line-height: 1.6;
}

.contact-label {
  flex: none;
  min-width: 64px;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

.coffee-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}

.coffee-image {
  max-width: 100%;
  height: auto;
  border-radius: var(--radius-md);
  margin-bottom: var(--space-3);

  @include theme.useTheme {
    border: 1px solid theme.getVar('borderColor');
  }
}

.coffee-text {
  margin: 0;
  font-size: var(--font-size-base);
  line-height: 1.7;

  @include theme.useTheme {
    color: theme.getVar('textSecondary');
  }
}

a {
  color: var(--el-color-primary);
  text-decoration: none;
  word-break: break-all;

  &:hover {
    text-decoration: underline;
  }
}
</style>
