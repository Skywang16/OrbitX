<script setup lang="ts">
  import type { ChatMessage } from '@/types'
  import type { AgentMessageData, AgentToolUseMessage } from '../types'
  import { computed, ref, watch } from 'vue'
  import { marked } from 'marked'
  import ThinkingMessage from './ThinkingMessage.vue'
  import ToolUseMessage from './ToolUseMessage.vue'

  interface Props {
    message: ChatMessage
    isStreaming?: boolean
  }
  const props = defineProps<Props>()
  // 代码块管理
  const codeBlocks = ref<Array<{ id: string; code: string; lang: string }>>([])
  const copiedStates = ref<Record<string, boolean>>({})
  // 计算属性
  const isUser = computed(() => props.message.messageType === 'user')
  const isAssistant = computed(() => props.message.messageType === 'assistant')
  const formattedTime = computed(() => {
    return new Intl.DateTimeFormat('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
    }).format(props.message.timestamp)
  })

  // 检查是否为Agent消息
  const isAgentMessage = computed(() => {
    if (!isAssistant.value) return false

    // 检查metadata中是否标记为Agent消息
    if (props.message.metadata && 'isAgentMessage' in props.message.metadata) {
      return props.message.metadata.isAgentMessage
    }

    // 检查content是否为JSON格式的Agent消息
    try {
      const parsed = JSON.parse(props.message.content)
      return parsed && typeof parsed === 'object' && 'type' in parsed && 'timestamp' in parsed
    } catch {
      return false
    }
  })

  // Agent原始输出
  const agentMessageStream = ref<AgentMessageData[]>([])

  watch(
    () => props.message.content,
    newContent => {
      if (!isAgentMessage.value) return
      try {
        const parsedStream = JSON.parse(newContent)
        if (Array.isArray(parsedStream)) {
          agentMessageStream.value = parsedStream
        } else {
          agentMessageStream.value = [parsedStream]
        }
      } catch (e) {
        agentMessageStream.value = []
      }
    },
    { immediate: true, deep: true }
  )

  const agentRawOutput = computed(() => {
    if (!isAgentMessage.value) return null

    // 优先从metadata中获取
    if (props.message.metadata && 'messageData' in props.message.metadata) {
      return props.message.metadata.messageData
    }

    // 从content中解析
    try {
      return JSON.parse(props.message.content)
    } catch {
      return props.message.content
    }
  })
  // 配置marked
  marked.setOptions({
    breaks: true,
    gfm: true,
  })
  // 自定义渲染器
  const renderer = new marked.Renderer()
  // 重写代码块渲染，添加复制功能
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
      <pre><code class="language-${lang}">${escapeHtml(code)}</code></pre>
    </div>`
  }
  marked.use({ renderer })
  // HTML转义函数
  function escapeHtml(text: string): string {
    const div = document.createElement('div')
    div.textContent = text
    return div.innerHTML
  }
  // 格式化内容
  const formattedContent = computed(() => {
    if (!isAssistant.value || isAgentMessage.value) return props.message.content
    try {
      // 重置代码块记录
      codeBlocks.value = []
      // 安全地解析markdown
      const content = props.message.content || ''
      return marked.parse(content)
    } catch (error) {
      return escapeHtml(props.message.content || '')
    }
  })
  // 代码复制功能
  const copyCode = async (code: string, blockId: string) => {
    try {
      await navigator.clipboard.writeText(code)
      copiedStates.value[blockId] = true
      setTimeout(() => {
        copiedStates.value[blockId] = false
      }, 2000)
    } catch (error) {}
  }
  // 处理复制按钮点击
  const handleCopyClick = (event: Event) => {
    const target = event.target as HTMLElement
    if (target.classList.contains('copy-btn')) {
      const codeId = target.getAttribute('data-code-id')
      if (codeId) {
        const codeBlock = codeBlocks.value.find(block => block.id === codeId)
        if (codeBlock) {
          copyCode(codeBlock.code, codeId)
          target.textContent = copiedStates.value[codeId] ? '已复制' : '复制'
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
  <!-- AI消息 -->
  <div v-else class="ai-message">
    <!-- Agent 消息 -->
    <div v-if="isAgentMessage" class="agent-message-container">
      <template v-for="(item, index) in agentMessageStream" :key="index">
        <thinking-message v-if="item.type === 'thinking'" :message="item" />
        <tool-use-message v-if="item.type === 'tool_use'" :message="item as AgentToolUseMessage" />
        <!-- Render 'text' type messages as plain text, without markdown for now -->
        <div v-if="item.type === 'text'" class="agent-text-message">
          {{ item.content }}
        </div>
        <!-- tool_result is not rendered directly; it just stops the loader for tool_use -->
      </template>
    </div>
    <!-- 普通AI消息使用markdown渲染 -->
    <div v-else class="ai-content" v-html="formattedContent" @click="handleCopyClick"></div>
    <div v-if="isStreaming" class="streaming-indicator">
      <span class="typing-cursor">|</span>
      <span class="streaming-text">回复中...</span>
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

  .agent-message-container {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .agent-text-message {
    padding: 8px 12px;
    background: var(--color-bg-primary);
    border-radius: 8px;
    font-size: 14px;
    line-height: 1.6;
    color: var(--color-text-primary);
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* Agent调试样式 */
  .agent-debug {
    background: #f8f9fa;
    border: 1px solid #e9ecef;
    border-radius: 8px;
    padding: 12px;
    margin-bottom: 8px;
  }

  .debug-header {
    font-size: 12px;
    font-weight: 600;
    color: #6c757d;
    margin-bottom: 8px;
    border-bottom: 1px solid #e9ecef;
    padding-bottom: 4px;
  }

  .debug-content {
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 11px;
    color: #495057;
    background: #ffffff;
    border: 1px solid #dee2e6;
    border-radius: 4px;
    padding: 8px;
    margin: 0;
    overflow-x: auto;
    max-height: 300px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
  }
  /* 普通AI消息样式 */
  .regular-ai-message {
    width: 100%;
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
