<template>
  <div class="button-demo">
    <div class="demo-container">
      <h1 class="demo-title">按钮组件演示</h1>
      <p class="demo-description">展示优化后的现代化按钮组件样式和交互效果</p>

      <!-- 基本按钮变体 -->
      <section class="demo-section">
        <h2 class="section-title">按钮变体</h2>
        <div class="button-group">
          <Button variant="primary">主要按钮</Button>
          <Button variant="secondary">次要按钮</Button>
          <Button variant="danger">危险按钮</Button>
          <Button variant="ghost">幽灵按钮</Button>
          <Button variant="link">链接按钮</Button>
        </div>
      </section>

      <!-- 按钮尺寸 -->
      <section class="demo-section">
        <h2 class="section-title">按钮尺寸</h2>
        <div class="button-group">
          <Button size="small" variant="primary">小按钮</Button>
          <Button size="medium" variant="primary">中按钮</Button>
          <Button size="large" variant="primary">大按钮</Button>
        </div>
      </section>

      <!-- 带图标的按钮 -->
      <section class="demo-section">
        <h2 class="section-title">图标按钮</h2>
        <div class="button-group">
          <Button variant="primary" icon="plus" icon-position="left">添加</Button>
          <Button variant="secondary" icon="download" icon-position="right">下载</Button>
          <Button variant="ghost" icon="settings" icon-position="left">设置</Button>
        </div>
      </section>

      <!-- 圆形和圆角按钮 -->
      <section class="demo-section">
        <h2 class="section-title">形状变化</h2>
        <div class="button-group">
          <Button variant="primary" round>圆角按钮</Button>
          <Button variant="secondary" circle icon="heart"></Button>
          <Button variant="danger" circle icon="close"></Button>
        </div>
      </section>

      <!-- 按钮状态 -->
      <section class="demo-section">
        <h2 class="section-title">按钮状态</h2>
        <div class="button-group">
          <Button variant="primary">正常状态</Button>
          <Button variant="primary" loading>加载中</Button>
          <Button variant="primary" disabled>禁用状态</Button>
        </div>
      </section>

      <!-- 块级按钮 -->
      <section class="demo-section">
        <h2 class="section-title">块级按钮</h2>
        <div class="button-group block">
          <Button variant="primary" block>块级主要按钮</Button>
          <Button variant="secondary" block>块级次要按钮</Button>
        </div>
      </section>

      <!-- 交互演示 -->
      <section class="demo-section">
        <h2 class="section-title">交互演示</h2>
        <div class="button-group">
          <Button variant="primary" @click="handleClick">点击我</Button>
          <Button variant="secondary" @click="showMessage">显示消息</Button>
          <Button variant="danger" @click="confirmAction">确认操作</Button>
        </div>
        <div v-if="message" class="demo-message">
          {{ message }}
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref } from 'vue'
  import Button from '../ui/components/Button.vue'

  const message = ref('')

  const handleClick = () => {
    message.value = '按钮被点击了！'
    setTimeout(() => {
      message.value = ''
    }, 2000)
  }

  const showMessage = () => {
    message.value = '这是一个演示消息'
    setTimeout(() => {
      message.value = ''
    }, 2000)
  }

  const confirmAction = () => {
    if (confirm('确定要执行这个操作吗？')) {
      message.value = '操作已确认执行'
      setTimeout(() => {
        message.value = ''
      }, 2000)
    }
  }
</script>

<style scoped>
  .button-demo {
    min-height: 100vh;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    padding: 40px 20px;
  }

  .demo-container {
    max-width: 1200px;
    margin: 0 auto;
    background: rgba(255, 255, 255, 0.95);
    border-radius: var(--border-radius-2xl);
    padding: 40px;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
    backdrop-filter: blur(10px);
  }

  .demo-title {
    font-size: 2.5rem;
    font-weight: 700;
    color: #1e293b;
    text-align: center;
    margin-bottom: 16px;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .demo-description {
    font-size: 1.125rem;
    color: #64748b;
    text-align: center;
    margin-bottom: 48px;
    line-height: 1.6;
  }

  .demo-section {
    margin-bottom: 48px;
  }

  .section-title {
    font-size: 1.5rem;
    font-weight: 600;
    color: #334155;
    margin-bottom: 24px;
    padding-bottom: 8px;
    border-bottom: 2px solid #e2e8f0;
  }

  .button-group {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    align-items: center;
  }

  .button-group.block {
    flex-direction: column;
    align-items: stretch;
    max-width: 400px;
  }

  .demo-message {
    margin-top: 16px;
    padding: 12px 16px;
    background: linear-gradient(135deg, #10b981 0%, #059669 100%);
    color: white;
    border-radius: var(--border-radius-lg);
    font-weight: 500;
    box-shadow: 0 4px 12px rgba(16, 185, 129, 0.3);
    animation: slideIn 0.3s ease-out;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @media (max-width: 768px) {
    .demo-container {
      padding: 24px;
    }

    .demo-title {
      font-size: 2rem;
    }

    .button-group {
      justify-content: center;
    }
  }
</style>
