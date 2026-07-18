<template>
  <div class="splash-container" data-tauri-drag-region>
    <!-- 背景动画效果 -->
    <div class="background-animation">
      <div class="circle circle-1"></div>
      <div class="circle circle-2"></div>
      <div class="circle circle-3"></div>
    </div>

    <!-- 主内容区 -->
    <div class="splash-content">
      <!-- Logo 区域 -->
      <div class="logo-section">
        <div class="logo-wrapper">
          <el-icon class="logo-icon" :size="80">
            <Document />
          </el-icon>
        </div>
        <h1 class="app-title">{{ title }}</h1>
        <p class="app-subtitle">{{ login_desc }}</p>
      </div>

      <!-- 加载指示器 -->
      <div class="loading-section">
        <div class="loading-spinner">
          <div class="spinner-ring"></div>
          <div class="spinner-ring"></div>
          <div class="spinner-ring"></div>
        </div>
        <p class="loading-text">{{ loadingText }}</p>
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: progress + '%' }"></div>
        </div>
      </div>

      <!-- 版本信息 -->
      <div class="version-info">
        <span class="version">v{{ appVersion }}</span>
        <span class="separator">|</span>
        <span class="tech">Powered by Tauri</span>
      </div>
    </div>

    <!-- 底部装饰 -->
    <div class="footer-decoration">
      <div class="dots">
        <span class="dot"></span>
        <span class="dot"></span>
        <span class="dot"></span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts" name="SplashScreen">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { Document } from '@element-plus/icons-vue'
import { title, login_desc } from '@/setting'
import useStore from '@/store'
import Cookies from 'js-cookie'

const router = useRouter()
const { user, routerNoteStore, permission } = useStore()
const loadingText = ref('正在启动...')
const progress = ref(0)
const appVersion = ref('1.0.0')

let progressInterval: number | null = null
let hasInitialized = false // 防止重复初始化

const loadingSteps = [
  { progress: 20, text: '初始化应用...' },
  { progress: 40, text: '加载配置...' },
  { progress: 60, text: '准备资源...' },
  { progress: 80, text: '即将完成...' },
  { progress: 100, text: '启动完成！' }
]

// 最小显示时间（毫秒）- 确保动画至少播放这么长时间
const MIN_DISPLAY_TIME = 3000 // 3秒

onMounted(async () => {
  const startTime = Date.now()
  try {
    const version = import.meta.env.PACKAGE_VERSION || '1.0.0'
    appVersion.value = version
  } catch (e) {
    console.warn('Failed to get app version:', e)
  }

  let currentStep = 0

  progressInterval = window.setInterval(async () => {
    if (currentStep < loadingSteps.length) {
      const step = loadingSteps[currentStep]
      progress.value = step.progress
      loadingText.value = step.text
      currentStep++
    } else {
      // 所有步骤已完成，检查是否达到最小显示时间
      const elapsedTime = Date.now() - startTime

      if (elapsedTime < MIN_DISPLAY_TIME) {
        // 如果还没到最小显示时间，等待剩余时间
        const remainingTime = MIN_DISPLAY_TIME - elapsedTime

        setTimeout(async () => {
          await handleInitialization()
        }, remainingTime)

        // 清理当前定时器
        if (progressInterval) {
          clearInterval(progressInterval)
          progressInterval = null
        }
      } else {
        // 已经达到最小显示时间，直接初始化
        await handleInitialization()
      }
    }
  }, 500)
})

// 处理初始化和跳转
const handleInitialization = async () => {
  // 防止重复初始化
  if (hasInitialized) {
    return
  }
  hasInitialized = true

  // 清理定时器
  if (progressInterval) {
    clearInterval(progressInterval)
    progressInterval = null
  }

  // 设置 token（如果没有的话）
  if (!user.hasToken()) {
    const savedToken = Cookies.get('token')
    if (savedToken) {
      user.token = savedToken
    } else {
      // 生成本地 token
      const localToken = 'local-' + Date.now() + '-' + Math.random().toString(36).substr(2, 9)
      user.token = localToken
      Cookies.set('token', localToken, { expires: 365 })
    }
  }

  // 初始化应用数据
  try {
    if (user.hasUserInfo()) {
      await initializeApp()
    } else if (user.hasToken()) {
      try {
        await user.getUserInfo()
      } catch (error) {
        console.warn('Failed to get user info, using default:', error)
        // 如果获取用户信息失败，设置默认用户信息
        user.userInfo = {
          account: 'local_user',
          nickName: '本地用户',
          phone: '',
          avatar: '',
          email: '',
          postTitle: ''
        }
      }
      await initializeApp()
    }

    // 短暂延迟后跳转，让用户看到"启动完成"
    setTimeout(() => {
      router.replace('/home/homePage').catch(err => {
        console.error('Navigation failed:', err)
        // 如果 replace 失败，尝试 push
        router.push('/home/homePage').catch(e => {
          console.error('Force navigation failed:', e)
        })
      })
    }, 500)
  } catch (error) {
    console.error('App initialization failed:', error)
    // 初始化失败，仍然跳转到首页
    setTimeout(() => {
      router.replace('/home/homePage').catch(err => {
        console.error('Navigation failed:', err)
      })
    }, 500)
  }
}

// 初始化应用
const initializeApp = async () => {
  try {
    const routerNote = await routerNoteStore.setRoutes()
    await permission.getPermissions()
    console.log('App initialized successfully')
  } catch (error) {
    console.error('Initialize app error:', error)
    throw error
  }
}

onUnmounted(() => {
  if (progressInterval) {
    clearInterval(progressInterval)
    progressInterval = null
  }
  hasInitialized = false
})
</script>

<style lang="scss" scoped>
@use '@/styles/variables.scss' as *;

.splash-container {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  overflow: hidden;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);

  .background-animation {
    position: absolute;
    width: 100%;
    height: 100%;
    overflow: hidden;

    .circle {
      position: absolute;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.1);
      animation: float 20s infinite ease-in-out;

      &.circle-1 {
        width: 300px;
        height: 300px;
        top: -100px;
        left: -100px;
        animation-delay: 0s;
      }

      &.circle-2 {
        width: 200px;
        height: 200px;
        bottom: -50px;
        right: -50px;
        animation-delay: 5s;
      }

      &.circle-3 {
        width: 150px;
        height: 150px;
        top: 50%;
        right: 20%;
        animation-delay: 10s;
      }
    }
  }

  @keyframes float {
    0%, 100% {
      transform: translate(0, 0) scale(1);
      opacity: 0.3;
    }
    50% {
      transform: translate(30px, -30px) scale(1.1);
      opacity: 0.5;
    }
  }

  .splash-content {
    position: relative;
    z-index: 10;
    text-align: center;
    color: white;
    max-width: 500px;
    padding: 40px;

    .logo-section {
      margin-bottom: 60px;
      animation: fadeInDown 0.8s ease-out;

      .logo-wrapper {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        width: 120px;
        height: 120px;
        background: rgba(255, 255, 255, 0.15);
        border-radius: 30px;
        backdrop-filter: blur(10px);
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
        margin-bottom: 24px;

        .logo-icon {
          color: white;
          filter: drop-shadow(0 4px 8px rgba(0, 0, 0, 0.2));
        }
      }

      .app-title {
        font-size: 42px;
        font-weight: 700;
        margin: 0 0 12px 0;
        letter-spacing: 2px;
        text-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
      }

      .app-subtitle {
        font-size: 16px;
        color: rgba(255, 255, 255, 0.85);
        margin: 0;
        font-weight: 300;
      }
    }

    .loading-section {
      margin-bottom: 40px;
      animation: fadeInUp 0.8s ease-out 0.3s both;

      .loading-spinner {
        position: relative;
        width: 80px;
        height: 80px;
        margin: 0 auto 24px;

        .spinner-ring {
          position: absolute;
          width: 100%;
          height: 100%;
          border: 3px solid transparent;
          border-top-color: rgba(255, 255, 255, 0.9);
          border-radius: 50%;
          animation: spin 1.5s cubic-bezier(0.68, -0.55, 0.265, 1.55) infinite;

          &:nth-child(2) {
            width: 60px;
            height: 60px;
            top: 10px;
            left: 10px;
            border-top-color: rgba(255, 255, 255, 0.7);
            animation-duration: 1.2s;
            animation-direction: reverse;
          }

          &:nth-child(3) {
            width: 40px;
            height: 40px;
            top: 20px;
            left: 20px;
            border-top-color: rgba(255, 255, 255, 0.5);
            animation-duration: 0.9s;
          }
        }
      }

      @keyframes spin {
        0% {
          transform: rotate(0deg);
        }
        100% {
          transform: rotate(360deg);
        }
      }

      .loading-text {
        font-size: 16px;
        color: rgba(255, 255, 255, 0.9);
        margin: 0 0 20px 0;
        min-height: 24px;
        transition: all 0.3s ease;
      }

      .progress-bar {
        width: 100%;
        height: 4px;
        background: rgba(255, 255, 255, 0.2);
        border-radius: 2px;
        overflow: hidden;

        .progress-fill {
          height: 100%;
          background: linear-gradient(90deg, rgba(255, 255, 255, 0.8), rgba(255, 255, 255, 1));
          border-radius: 2px;
          transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
          box-shadow: 0 0 10px rgba(255, 255, 255, 0.5);
        }
      }
    }

    .version-info {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 12px;
      font-size: 13px;
      color: rgba(255, 255, 255, 0.6);
      animation: fadeIn 0.8s ease-out 0.6s both;

      .separator {
        color: rgba(255, 255, 255, 0.4);
      }

      .tech {
        font-weight: 500;
      }
    }
  }

  .footer-decoration {
    position: absolute;
    bottom: 40px;
    left: 50%;
    transform: translateX(-50%);
    animation: fadeIn 0.8s ease-out 0.9s both;

    .dots {
      display: flex;
      gap: 8px;

      .dot {
        width: 8px;
        height: 8px;
        background: rgba(255, 255, 255, 0.4);
        border-radius: 50%;
        animation: pulse 2s infinite ease-in-out;

        &:nth-child(2) {
          animation-delay: 0.2s;
        }

        &:nth-child(3) {
          animation-delay: 0.4s;
        }
      }
    }
  }

  @keyframes pulse {
    0%, 100% {
      transform: scale(1);
      opacity: 0.4;
    }
    50% {
      transform: scale(1.3);
      opacity: 0.8;
    }
  }
}

@keyframes fadeInDown {
  from {
    opacity: 0;
    transform: translateY(-30px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(30px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@media (max-width: 768px) {
  .splash-container {
    .splash-content {
      padding: 30px 20px;

      .logo-section {
        margin-bottom: 40px;

        .logo-wrapper {
          width: 100px;
          height: 100px;

          .logo-icon {
            font-size: 60px !important;
          }
        }

        .app-title {
          font-size: 32px;
        }

        .app-subtitle {
          font-size: 14px;
        }
      }

      .loading-section {
        .loading-spinner {
          width: 60px;
          height: 60px;

          .spinner-ring:nth-child(2) {
            width: 45px;
            height: 45px;
          }

          .spinner-ring:nth-child(3) {
            width: 30px;
            height: 30px;
          }
        }
      }
    }
  }
}
</style>
