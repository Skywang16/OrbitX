<template>
  <div class="streaming-test">
    <h3>流式显示测试</h3>

    <div class="test-controls">
      <button @click="startMockStream" :disabled="isStreaming">
        {{ isStreaming ? '流式进行中...' : '开始模拟流式' }}
      </button>
      <button @click="stopMockStream" :disabled="!isStreaming">停止流式</button>
    </div>

    <div class="test-message">
      <div class="message-content">{{ streamContent }}</div>
      <div v-if="isStreaming" class="streaming-indicator">
        <span class="typing-cursor">|</span>
        <span class="streaming-text">模拟流式输出中...</span>
      </div>
    </div>

    <div class="test-stats">
      <p>已接收字符数: {{ streamContent.length }}</p>
      <p>流式状态: {{ isStreaming ? '进行中' : '已停止' }}</p>
      <p>更新频率: {{ updateCount }} 次</p>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref } from 'vue'

  const isStreaming = ref(false)
  const streamContent = ref('')
  const updateCount = ref(0)
  let streamInterval: NodeJS.Timeout | null = null

  const mockText = `这是一个模拟的AI回复，用来测试流式显示效果。

## 功能特点

1. **实时更新**: 内容会逐字符显示
2. **流畅滚动**: 自动滚动到底部
3. **性能优化**: 避免频繁的DOM操作

### 代码示例

\`\`\`javascript
// 这是一个示例代码块
function streamResponse(content) {
  return new Promise((resolve) => {
    let index = 0;
    const interval = setInterval(() => {
      if (index < content.length) {
        // 逐字符输出
        process.stdout.write(content[index]);
        index++;
      } else {
        clearInterval(interval);
        resolve();
      }
    }, 50);
  });
}
\`\`\`

这样就可以实现真正的流式显示效果了！`

  const startMockStream = () => {
    if (isStreaming.value) return

    isStreaming.value = true
    streamContent.value = ''
    updateCount.value = 0

    let index = 0
    streamInterval = setInterval(() => {
      if (index < mockText.length) {
        streamContent.value += mockText[index]
        updateCount.value++
        index++
      } else {
        stopMockStream()
      }
    }, 50) // 每50ms添加一个字符
  }

  const stopMockStream = () => {
    if (streamInterval) {
      clearInterval(streamInterval)
      streamInterval = null
    }
    isStreaming.value = false
  }
</script>

<style scoped>
  .streaming-test {
    padding: 20px;
    border: 1px solid var(--color-border);
    border-radius: 8px;
    margin: 20px;
  }

  .test-controls {
    display: flex;
    gap: 10px;
    margin-bottom: 20px;
  }

  .test-controls button {
    padding: 8px 16px;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    background: var(--color-background);
    color: var(--color-text);
    cursor: pointer;
  }

  .test-controls button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .test-message {
    min-height: 200px;
    padding: 16px;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    background: var(--color-background-secondary);
    white-space: pre-wrap;
    font-family: inherit;
    line-height: 1.5;
    margin-bottom: 20px;
  }

  .message-content {
    word-wrap: break-word;
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

  .test-stats {
    padding: 12px;
    background: var(--color-background-hover);
    border-radius: 4px;
    font-size: 14px;
  }

  .test-stats p {
    margin: 4px 0;
  }
</style>
