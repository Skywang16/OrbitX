<script setup lang="ts">
  import type { ChatMessage } from '@/types'
  import { marked } from 'marked'
  import { computed, ref } from 'vue'

  interface Props {
    message: ChatMessage
    isStreaming?: boolean
  }

  const props = withDefaults(defineProps<Props>(), {
    isStreaming: false,
  })

  const codeBlocks = ref<Array<{ id: string; code: string; lang: string }>>([])
  const copiedStates = ref<Record<string, boolean>>({})

  // 代码复制功能
  const copyCode = async (code: string, blockId: string) => {
    try {
      await navigator.clipboard.writeText(code)
      copiedStates.value[blockId] = true
      setTimeout(() => {
        copiedStates.value[blockId] = false
      }, 2000)
    } catch (error) {
      console.error('复制失败:', error)
    }
  }

  const getCopyButtonText = (blockId: string) => {
    return copiedStates.value[blockId] ? '已复制' : '复制'
  }

  // 计算属性
  const isUser = computed(() => props.message.messageType === 'user')
  const isAssistant = computed(() => props.message.messageType === 'assistant')

  const formattedTime = computed(() => {
    return new Intl.DateTimeFormat('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
    }).format(props.message.timestamp)
  })

  // 配置marked
  marked.setOptions({
    highlight: function (code, lang) {
      // 简单的代码高亮，可以后续扩展
      return code
    },
    breaks: true, // 支持换行符转换为<br>
    gfm: true, // 启用GitHub风格的Markdown
  })

  // 自定义渲染器，为代码块添加复制按钮
  const renderer = new marked.Renderer()
  renderer.code = function (code, language) {
    const lang = language || 'text'
    const id = `code-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`

    // 记录代码块信息
    codeBlocks.value.push({ id, code, lang })

    return `<div class="code-block" data-code-id="${id}">
      <div class="code-header">
        <span class="code-lang">${lang}</span>
        <button class="copy-btn" data-code-id="${id}">复制</button>
      </div>
      <pre><code class="language-${lang}">${code}</code></pre>
    </div>`
  }

  marked.use({ renderer })

  // 格式化AI消息内容
  const formattedContent = computed(() => {
    if (!isAssistant.value) return props.message.content

    try {
      // 重置代码块记录
      codeBlocks.value = []
      return marked.parse(props.message.content)
    } catch (error) {
      console.error('Markdown解析错误:', error)
      return props.message.content
    }
  })

  // 处理复制按钮点击
  const handleCopyClick = (event: Event) => {
    const target = event.target as HTMLElement
    if (target.classList.contains('copy-btn')) {
      const codeId = target.getAttribute('data-code-id')
      if (codeId) {
        const codeBlock = codeBlocks.value.find(block => block.id === codeId)
        if (codeBlock) {
          copyCode(codeBlock.code, codeId)
          // 更新按钮文本
          target.textContent = getCopyButtonText(codeId)
        }
      }
    }
  }
</script>

<template>
  <!-- 用户消息：极简样式 -->
  <div v-if="isUser" class="user-message">
    <div class="message-time">{{ formattedTime }}</div>
    <div class="user-content">
      <div class="message-content">{{ message.content }}</div>
    </div>
  </div>

  <!-- AI消息：直接渲染，无气泡 -->
  <div v-else class="ai-message">
    <div class="ai-content" v-html="formattedContent" @click="handleCopyClick"></div>
    <div v-if="isStreaming" class="streaming-indicator">
      <span class="typing-cursor">|</span>
      <span class="streaming-text">AI 正在回复...</span>
    </div>
  </div>
</template>

<style scoped>
  /* 用户消息样式 */
  .user-message {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    margin-bottom: 16px;
  }

  .user-message .message-time {
    font-size: 10px;
    opacity: 0.6;
    color: var(--color-text-secondary);
    margin-bottom: 4px;
    margin-right: 4px;
  }

  .user-content {
    max-width: 70%;
    background: rgba(24, 144, 255, 0.05);
    color: var(--color-text-primary);
    border: 1px solid rgba(24, 144, 255, 0.15);
    border-radius: 12px;
    padding: 6px 10px;
    transition: all 0.2s ease;
  }

  .user-content .message-content {
    line-height: 1.4;
    word-wrap: break-word;
    font-size: 14px;
    font-weight: 400;
  }

  /* AI消息样式 */
  .ai-message {
    margin-bottom: 16px;
  }

  .ai-content {
    line-height: 1.5;
    color: var(--color-ai-message-text);
    word-wrap: break-word;
    font-size: 14px;
  }

  .streaming-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    opacity: 0.7;
  }

  .typing-cursor {
    color: #1890ff;
    font-weight: bold;
    animation: blink 1s infinite;
  }

  .streaming-text {
    font-size: 12px;
    color: var(--color-text-secondary);
    font-style: italic;
  }

  @keyframes blink {
    0%,
    50% {
      opacity: 1;
    }
    51%,
    100% {
      opacity: 0;
    }
  }

  /* Markdown基础样式 */
  :deep(h1),
  :deep(h2),
  :deep(h3),
  :deep(h4),
  :deep(h5),
  :deep(h6) {
    margin: 16px 0 8px 0;
    font-weight: 600;
    line-height: 1.25;
    color: var(--color-ai-message-text);
  }

  :deep(h1) {
    font-size: 1.5em;
  }
  :deep(h2) {
    font-size: 1.3em;
  }
  :deep(h3) {
    font-size: 1.1em;
  }
  :deep(h4),
  :deep(h5),
  :deep(h6) {
    font-size: 1em;
  }

  :deep(p) {
    margin: 8px 0;
    line-height: 1.6;
  }

  :deep(strong) {
    font-weight: 600;
  }

  :deep(em) {
    font-style: italic;
  }

  /* 列表样式 */
  :deep(ul),
  :deep(ol) {
    margin: 8px 0;
    padding-left: 20px;
  }

  :deep(li) {
    margin: 4px 0;
    line-height: 1.5;
  }

  /* 引用样式 */
  :deep(blockquote) {
    margin: 8px 0;
    padding: 8px 16px;
    border-left: 4px solid var(--color-primary);
    background: var(--color-background-secondary);
    font-style: italic;
  }

  /* 代码块样式 */
  :deep(.code-block) {
    margin: 12px 0;
    border-radius: 6px;
    background: var(--color-background-secondary);
    border: 1px solid var(--color-border);
    overflow: hidden;
  }

  :deep(.code-header) {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--color-background-hover);
    border-bottom: 1px solid var(--color-border);
    font-size: 12px;
    color: var(--color-text-secondary);
  }

  :deep(.code-lang) {
    font-weight: 500;
    text-transform: uppercase;
  }

  :deep(.copy-btn) {
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 11px;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  :deep(.copy-btn:hover) {
    background: var(--color-primary-hover);
  }

  :deep(.code-block pre) {
    margin: 0;
    padding: 12px;
    overflow-x: auto;
    background: transparent;
    font-family: var(--font-family-mono);
    font-size: 13px;
    line-height: 1.4;
  }

  :deep(.code-block code) {
    color: var(--color-ai-message-text);
    background: transparent;
  }

  /* 行内代码样式 */
  :deep(code:not(.code-block code)) {
    background: var(--color-background-secondary);
    color: var(--color-primary);
    padding: 2px 6px;
    border-radius: 3px;
    font-family: var(--font-family-mono);
    font-size: 0.9em;
    border: 1px solid var(--color-border);
  }

  /* 表格样式 */
  :deep(table) {
    margin: 12px 0;
    border-collapse: collapse;
    width: 100%;
    font-size: 14px;
  }

  :deep(th),
  :deep(td) {
    padding: 8px 12px;
    border: 1px solid var(--color-border);
    text-align: left;
  }

  :deep(th) {
    background: var(--color-background-secondary);
    font-weight: 600;
  }

  /* 分割线样式 */
  :deep(hr) {
    margin: 16px 0;
    border: none;
    border-top: 1px solid var(--color-border);
  }

  /* 链接样式 */
  :deep(a) {
    color: var(--color-primary);
    text-decoration: none;
  }

  :deep(a:hover) {
    text-decoration: underline;
  }
</style>
