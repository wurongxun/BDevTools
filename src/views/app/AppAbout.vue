<template>
  <div class="about-container">
    <div class="about-content">
      <div class="info-section">
        <h2>应用信息</h2>
        <div class="info-grid">
          <div class="info-item">
            <span class="info-label">应用名称:</span>
            <span class="info-value">{{ data.appName || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">Tauri版本:</span>
            <span class="info-value">{{ data.tauriVersion || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">应用版本:</span>
            <span class="info-value">{{ data.appVersion || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">系统架构:</span>
            <span class="info-value">{{ data.archName || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">平台:</span>
            <span class="info-value">{{ data.platformName || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">系统类型:</span>
            <span class="info-value">{{ data.osType || '获取中...' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">系统版本:</span>
            <span class="info-value">{{ data.osVersion || '获取中...' }}</span>
          </div>
        </div>

        <h2>联系信息</h2>
        <div class="contact-info">
          <p><strong>联系邮箱:</strong> <a href="mailto:1475316789@qq.com">1475316789@qq.com</a></p>
          <p><strong>图标出处:</strong></p>
          <p><a href="https://www.flaticon.com/free-icon/substance_9814620" target="_blank">
            https://www.flaticon.com/free-icon/substance_9814620
          </a></p>
          <p><strong>Gitee地址:</strong></p>
          <p><a href="https://gitee.com/wurongxun/md-note" target="_blank">
            https://gitee.com/wurongxun/md-note
          </a></p>
        </div>
      </div>

      <div class="coffee-section">
        <h2>请我喝杯咖啡</h2>
        <div class="coffee-content">
          <el-image 
            alt="请我喝杯咖啡" 
            class="coffee-image"
            :src="coffeeImage" 
          />
          <p class="coffee-text">如果您觉得这个应用对您有帮助，欢迎请我喝杯咖啡！您的支持是我持续开发的动力。</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, reactive } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getName, getTauriVersion, getVersion } from '@tauri-apps/api/app';
import { arch, platform, type, version } from '@tauri-apps/plugin-os';
import coffeeImage from '@/assets/coffee.jpg';

const data = reactive({
  appName: "",
  tauriVersion: "",
  appVersion: "",
  archName: "",
  platformName: "",
  osType: "",
  osVersion: ""
});

const handleWinMin = async () => {
  try {
    const res = await invoke('windows_hide', { label: "about-window" });
    console.log(res);
  } catch (error) {
    console.error('最小化窗口失败:', error);
  }
};

const handleWinClose = async () => {
  try {
    const res = await invoke('windows_close', { label: "about-window" });
    console.log(res);
  } catch (error) {
    console.error('关闭窗口失败:', error);
  }
};

onMounted(async () => {
  try {
    data.appName = await getName();
    data.tauriVersion = await getTauriVersion();
    data.appVersion = await getVersion();
    data.archName = await arch();
    data.platformName = await platform();
    data.osType = await type();
    data.osVersion = await version();
  } catch (error) {
    console.error('获取应用信息失败:', error);
  }
});
</script>

<style scoped>
.about-container {
  width: 100%;
  height: 100%;
  padding: 0px;
  box-sizing: border-box;
  overflow-y: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

/* 隐藏 Webkit 浏览器的滚动条 */
.about-container::-webkit-scrollbar {
  width: 0px;
  height: 0px;
  background: transparent; /* Chrome/Safari/Webkit */
}

.about-content {
  display: flex;
  flex-wrap: wrap;
  gap: 20px;
  width: 100%;
  padding: 20px;
  box-sizing: border-box;
}

.info-section {
  flex: 1;
  min-width: 300px;
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.coffee-section {
  flex: 1;
  min-width: 300px;
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  display: flex;
  flex-direction: column;
}

h2 {
  color: #333;
  border-bottom: 2px solid ;
  padding-bottom: 10px;
  margin-top: 0;
  margin-bottom: 20px;
}

.info-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 15px;
  margin-bottom: 30px;
}

.info-item {
  display: flex;
  flex-direction: column;
}

.info-label {
  font-weight: bold;
  color: #666;
  margin-bottom: 5px;
}

.info-value {
  color: #333;
  font-size: 14px;
}

.contact-info p {
  margin: 10px 0;
  line-height: 1.5;
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
  border-radius: 8px;
  margin-bottom: 20px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.coffee-text {
  color: #666;
  line-height: 1.6;
  font-size: 14px;
}

a {
  color: #409eff;
  text-decoration: none;
}

a:hover {
  text-decoration: underline;
}

@media (max-width: 768px) {
  .about-content {
    flex-direction: column;
  }
  
  .info-grid {
    grid-template-columns: 1fr;
  }
  
  .about-container {
    padding: 10px;
  }
}
</style>