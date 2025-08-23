<script setup lang="ts">
  import { computed, nextTick, onMounted, ref, watch } from 'vue'
  import type { Message } from '@/types'
  import UserMessage from './UserMessage.vue'
  import AIMessage from './AIMessage.vue'
  import lottie, { type AnimationItem } from 'lottie-web'

  interface Props {
    messages: Message[]
  }

  const props = defineProps<Props>()

  // 消息列表容器引用
  const messageListRef = ref<HTMLElement | null>(null)
  // Lottie动画容器引用
  const lottieContainer = ref<HTMLElement | null>(null)

  // 消息列表
  const msgList = computed(() => {
    return props.messages.map(msg => ({
      ...msg,
      type: msg.role as 'user' | 'assistant',
    }))
  })

  // 自动滚动到底部
  const scrollToBottom = async () => {
    await nextTick()
    if (messageListRef.value) {
      messageListRef.value.scrollTop = messageListRef.value.scrollHeight
    }
  }

  // 监听消息变化，自动滚动到底部
  watch(
    () => msgList.value.length,
    () => {
      scrollToBottom()
    },
    { immediate: true }
  )

  // 当空状态容器挂载时初始化动画
  onMounted(() => {
    if (msgList.value.length === 0 && lottieContainer.value) {
      lottie.loadAnimation({
        container: lottieContainer.value,
        renderer: 'svg',
        loop: true,
        autoplay: true,
        path: '/Circle.json',
      })
    }
  })
</script>

<template>
  <div ref="messageListRef" class="message-list">
    <div v-if="msgList.length === 0" class="empty-state">
      <!-- Lottie动画容器 -->
      <div class="empty-icon">
        <div ref="lottieContainer" class="lottie-animation"></div>
      </div>
      <div class="empty-text">开始对话吧</div>
      <div class="empty-hint">发送消息开始与Orbit对话</div>
    </div>

    <div v-else class="message-container">
      <template v-for="message in msgList" :key="message.id">
        <!-- 用户消息 -->
        <UserMessage v-if="message.type === 'user'" :message="message" />

        <!-- AI消息 -->
        <AIMessage v-else-if="message.type === 'assistant'" :message="message" />
      </template>
    </div>
  </div>
</template>

<style scoped>
  .message-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--spacing-md);
    display: flex;
    flex-direction: column;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    color: var(--text-400);
    gap: var(--spacing-lg);
  }

  .empty-icon {
    margin-bottom: var(--spacing-md);
  }

  .lottie-animation {
    width: 120px;
    height: 120px;
    filter: drop-shadow(0 0 12px rgba(var(--color-primary-rgb, 59, 130, 246), 0.4));
  }

  /* Lottie动画中的SVG元素自动继承主题色彩 */
  .lottie-animation svg {
    color: var(--color-primary);
  }

  .lottie-animation svg path,
  .lottie-animation svg circle,
  .lottie-animation svg ellipse {
    stroke: var(--color-primary) !important;
    fill: var(--color-primary) !important;
  }

  .empty-text {
    font-size: var(--font-size-lg);
    font-weight: 500;
    color: var(--text-200);
  }

  .empty-hint {
    font-size: var(--font-size-sm);
    opacity: 0.7;
  }

  .message-container {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }
</style>
