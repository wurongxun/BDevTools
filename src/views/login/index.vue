<template>
  <div class="container">
    <!-- <img class="icon" src="../../assets/login/login_icon.png" alt="icon" /> -->
    <div class="login-model">
      <h2 class="title">{{ title }}</h2>
      <p class="desc">{{ login_desc }}</p>
      <p>
        <el-input v-model="accountInfo.account" size="large" placeholder="请输入账号" />
      </p>
      <p>
        <el-input v-model="accountInfo.password" size="large" type="password" placeholder="请输入密码" show-password />
      </p>
      <p class="login-btn-box">
        <el-button class="login-btn" type="primary" plain @click="handleLogin">登录</el-button>
        <el-button type="primary" text>忘记密码？</el-button>
      </p>
    </div>
  </div>
</template>

<script setup lang="ts" name="Login">
import { onMounted } from 'vue'
import useStore from '@/store'
import { title, login_desc } from '@/setting'
import { useRouter } from 'vue-router'
import { ElNotification } from 'element-plus'
import { useTyper } from '@/composables/useTyper'

const router = useRouter()
const { user } = useStore()

const accountInfo = {
  account: 'admin',
  password: 'admin'
}

function handleLogin() {
  user.userLogin(accountInfo).then(() => {
    console.log('Login successful, navigating to home')
    router.push('/').then(() => {
      console.log('Navigation completed')
    }).catch(err => {
      console.error('Navigation failed:', err)
    })
    
    ElNotification({
      title: title,
      message: `登录成功`,
      type: 'success'
    })
  }).catch(error => {
    console.error('Login failed:', error)
  })
}

onMounted(() => {
  useTyper('.title')
})
</script>
<style lang="scss" scoped>
.container {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(to right, #97c6fe, #f2f5ff);
  padding: 20px; // 添加内边距防止小屏幕时内容被截断
  
  .icon {
    position: absolute;
    width: 360px;
    left: 60px;
    top: 45px;
  }

  .desc {
    color: #aaa;
    font-size: 12px;
  }

  .title {
    font-size: 32px;
    text-align: center;
    color: #333;
  }

  .login-btn-box {
    display: flex;

    .login-btn {
      width: 40%;
    }
  }
}

.login-model {
  width: 100%;
  max-width: 500px; // 使用 max-width 而不是固定宽度
  padding: 36px 80px;
  border-radius: 5px;
  background: #f1f1f1;
  box-shadow: 0 0 5px 0 #97c6fe;
  background: url('../../assets/login/login_background.jpg') no-repeat;
  background-size: cover;
  
  // 小屏幕适配
  @media (max-width: 768px) {
    padding: 24px 30px;
    margin: 0 10px;
  }
}
</style>