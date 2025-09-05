<template>
  <div class="llm-test-container">
    <h1>🤖 LLM 接口测试工具</h1>

    <!-- 模型选择和连接测试 -->
    <div class="test-section">
      <h2>📋 模型选择</h2>

      <div class="form-group">
        <label>选择模型</label>
        <div class="model-selector">
          <select v-model="modelId" :disabled="isLoadingModels">
            <option value="">{{ isLoadingModels ? '加载中...' : '请选择模型' }}</option>
            <option v-for="model in availableModels" :key="model.id" :value="model.id">
              {{ model.name }} ({{ model.model }})
            </option>
          </select>
          <button class="btn secondary" @click="loadModels" :disabled="isLoadingModels">
            {{ isLoadingModels ? '刷新中...' : '🔄 刷新' }}
          </button>
          <button class="btn secondary" @click="testConnection" :disabled="!modelId || isTestingConnection">
            {{ isTestingConnection ? '测试中...' : '🔗 测试连接' }}
          </button>
        </div>
      </div>

      <div v-if="connectionResult" class="connection-result" :class="connectionResultType">
        {{ connectionResult }}
      </div>
    </div>

    <!-- LLM 调用测试 -->
    <div class="test-section">
      <h2>🚀 LLM 调用测试</h2>

      <div class="form-group">
        <label>测试消息</label>
        <textarea v-model="message" rows="3" placeholder="输入要发送给 AI 的消息">
你好！请用中文回复一句话证明你能正常工作。</textarea
        >
      </div>

      <div class="button-group">
        <button class="btn primary" @click="testLLMCall" :disabled="!modelId || isLoading || isStreamLoading">
          {{ isLoading ? '调用中...' : '🧪 测试调用' }}
        </button>
        <button class="btn primary" @click="testLLMStreamCall" :disabled="!modelId || isLoading || isStreamLoading">
          {{ isStreamLoading ? '流式调用中...' : '🌊 流式测试' }}
        </button>
        <button class="btn secondary" @click="clearResult">清空结果</button>
        <button v-if="isStreamLoading" class="btn danger" @click="stopStream">⏹️ 停止流式</button>
      </div>

      <div v-if="result" class="result-area" :class="resultType">
        <pre>{{ result }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, onMounted, onBeforeUnmount } from 'vue'
  import { invoke, Channel } from '@tauri-apps/api/core'

  // 基础状态
  const modelId = ref('')
  const message = ref('你好！请用中文回复一句话证明你能正常工作。')
  const result = ref('')
  const resultType = ref<'success' | 'error' | 'info' | 'loading'>('info')
  const isLoading = ref(false)

  // 流式调用状态
  const isStreamLoading = ref(false)
  const streamContent = ref('')
  const streamUnlisten = ref<(() => void) | null>(null)

  // 模型相关状态
  const availableModels = ref<Array<{ id: string; name: string; model: string }>>([])
  const isLoadingModels = ref(false)
  const connectionResult = ref('')
  const connectionResultType = ref<'success' | 'error' | 'info'>('info')
  const isTestingConnection = ref(false)

  // 加载可用模型列表
  async function loadModels() {
    isLoadingModels.value = true
    connectionResult.value = ''

    try {
      // 获取完整的模型配置信息
      const models = await invoke<Array<{ id: string; name: string; model: string; provider: string }>>('get_ai_models')
      availableModels.value = models.map(m => ({
        id: m.id,
        name: m.name,
        model: m.model,
      }))

      if (models.length === 0) {
        connectionResult.value = '⚠️ 未找到可用模型，请先在设置中配置 AI 模型'
        connectionResultType.value = 'error'
      } else {
        connectionResult.value = `✅ 成功加载 ${models.length} 个模型`
        connectionResultType.value = 'success'
      }
    } catch (error: any) {
      connectionResult.value = `❌ 加载模型失败: ${error.message || error}`
      connectionResultType.value = 'error'
      availableModels.value = []
    } finally {
      isLoadingModels.value = false
    }
  }

  // 测试模型连接
  async function testConnection() {
    if (!modelId.value) {
      connectionResult.value = '❌ 请先选择一个模型'
      connectionResultType.value = 'error'
      return
    }

    isTestingConnection.value = true
    connectionResult.value = '正在测试连接...'
    connectionResultType.value = 'info'

    try {
      const isConnected = await invoke<boolean>('llm_test_model_connection', {
        modelId: modelId.value,
      })

      if (isConnected) {
        connectionResult.value = `✅ 模型 "${modelId.value}" 连接成功`
        connectionResultType.value = 'success'
      } else {
        connectionResult.value = `❌ 模型 "${modelId.value}" 连接失败`
        connectionResultType.value = 'error'
      }
    } catch (error: any) {
      connectionResult.value = `❌ 连接测试失败: ${error.message || error}`
      connectionResultType.value = 'error'
    } finally {
      isTestingConnection.value = false
    }
  }

  function clearResult() {
    result.value = ''
    streamContent.value = ''
  }

  // 构建简单的 LLM 请求对象
  function buildLLMRequest(isStream = false) {
    return {
      model: modelId.value, // 使用标准的 model 字段
      messages: [
        {
          role: 'user',
          content: message.value,
        },
      ],
      temperature: 0.7,
      max_tokens: isStream ? 500 : 150,
      stream: isStream,
    }
  }

  // 停止流式调用
  function stopStream() {
    if (streamUnlisten.value) {
      streamUnlisten.value()
      streamUnlisten.value = null
    }
    isStreamLoading.value = false

    if (streamContent.value) {
      result.value = `🌊 流式调用已停止\n\n已接收内容:\n${streamContent.value}`
      resultType.value = 'info'
    }
  }

  // 流式 LLM 调用
  async function testLLMStreamCall() {
    if (!modelId.value.trim()) {
      result.value = '请选择模型'
      resultType.value = 'error'
      return
    }

    if (!message.value.trim()) {
      result.value = '请输入测试消息'
      resultType.value = 'error'
      return
    }

    isStreamLoading.value = true
    streamContent.value = ''
    result.value = '🌊 开始流式调用...\n\n'
    resultType.value = 'loading'

    const request = buildLLMRequest(true)

    try {
      // 创建 Channel 来接收流式数据
      const onChunk = new Channel<any>()

      onChunk.onmessage = chunk => {
        if (chunk.type === 'Delta') {
          if (chunk.content) {
            streamContent.value += chunk.content
            result.value = `🌊 流式调用进行中...\n\n实时内容:\n${streamContent.value}`
          }
        } else if (chunk.type === 'Finish') {
          isStreamLoading.value = false

          const finishText = `✅ 流式调用完成！\n\n📊 响应信息:\n- 完整内容: ${streamContent.value}\n- 结束原因: ${chunk.finish_reason || '未知'}\n\n📈 使用统计:\n${
            chunk.usage
              ? `- Prompt Tokens: ${chunk.usage.prompt_tokens || 0}\n- Completion Tokens: ${chunk.usage.completion_tokens || 0}\n- Total Tokens: ${chunk.usage.total_tokens || 0}`
              : '- 无使用统计信息'
          }\n\n🔧 请求参数:\n${JSON.stringify(request, null, 2)}`

          result.value = finishText
          resultType.value = 'success'
        } else if (chunk.type === 'Error') {
          isStreamLoading.value = false
          result.value = `❌ 流式调用失败: ${chunk.error}`
          resultType.value = 'error'
        }
      }

      // 调用流式接口
      await invoke('llm_call_stream', {
        request,
        onChunk,
      })
    } catch (error: any) {
      isStreamLoading.value = false
      result.value = `❌ 流式调用失败: ${error.message || error}`
      resultType.value = 'error'
    }
  }

  // 页面加载时自动获取模型列表
  onMounted(() => {
    loadModels()
  })

  // 页面卸载时清理流式调用
  onBeforeUnmount(() => {
    stopStream()
  })

  async function testLLMCall() {
    if (!modelId.value.trim()) {
      result.value = '请输入模型 ID'
      resultType.value = 'error'
      return
    }

    if (!message.value.trim()) {
      result.value = '请输入测试消息'
      resultType.value = 'error'
      return
    }

    isLoading.value = true
    result.value = '正在调用 LLM...'
    resultType.value = 'loading'

    const request = buildLLMRequest(false)

    try {
      const response = (await invoke('llm_call', { request })) as any

      const resultText = `✅ LLM 调用成功！

📊 响应信息:
- 内容: ${response.content || '无内容'}
- 结束原因: ${response.finish_reason || '未知'}
- 工具调用: ${response.tool_calls ? JSON.stringify(response.tool_calls, null, 2) : '无'}

📈 使用统计:
${
  response.usage
    ? `- Prompt Tokens: ${response.usage.prompt_tokens || 0}
- Completion Tokens: ${response.usage.completion_tokens || 0}
- Total Tokens: ${response.usage.total_tokens || 0}`
    : '- 无使用统计信息'
}

🔧 请求参数:
${JSON.stringify(request, null, 2)}`

      result.value = resultText
      resultType.value = 'success'
    } catch (error: any) {
      result.value = `LLM 调用失败: ${error.message || error}`
      resultType.value = 'error'
    } finally {
      isLoading.value = false
    }
  }
</script>

<style scoped>
  .llm-test-container {
    max-width: 800px;
    margin: 0 auto;
    padding: 20px;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  }

  h1 {
    color: var(--text-100);
    margin-bottom: 32px;
    font-size: 28px;
    font-weight: 600;
  }

  .test-section {
    margin-bottom: 40px;
    padding: 24px;
    background: var(--bg-300);
    border-radius: 8px;
    border: 1px solid var(--border-300);
  }

  .test-section h2 {
    color: var(--text-200);
    margin: 0 0 20px 0;
    font-size: 20px;
    font-weight: 600;
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-200);
    margin-bottom: 6px;
  }

  .form-group input,
  .form-group textarea,
  .form-group select {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--border-300);
    border-radius: 4px;
    background-color: var(--bg-400);
    color: var(--text-200);
    font-size: 14px;
    transition: border-color 0.2s ease;
    box-sizing: border-box;
    font-family: inherit;
  }

  .model-selector {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .model-selector select {
    flex: 1;
    min-width: 0;
  }

  .connection-result {
    margin-top: 12px;
    padding: 12px;
    border-radius: 4px;
    font-size: 14px;
    font-weight: 500;
  }

  .connection-result.success {
    background: rgba(78, 201, 176, 0.1);
    color: #4ec9b0;
    border: 1px solid rgba(78, 201, 176, 0.3);
  }

  .connection-result.error {
    background: rgba(244, 71, 71, 0.1);
    color: #f44747;
    border: 1px solid rgba(244, 71, 71, 0.3);
  }

  .connection-result.info {
    background: rgba(86, 156, 214, 0.1);
    color: #569cd6;
    border: 1px solid rgba(86, 156, 214, 0.3);
  }

  .form-group textarea {
    resize: vertical;
    font-family: 'Consolas', 'Monaco', monospace;
  }

  .form-group input:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: var(--color-primary);
  }

  .button-group {
    display: flex;
    gap: 12px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }

  .btn {
    padding: 10px 20px;
    border: 1px solid var(--border-300);
    border-radius: 4px;
    font-size: 14px;
    cursor: pointer;
    transition: all 0.2s ease;
    background: var(--bg-500);
    color: var(--text-200);
  }

  .btn:hover {
    background: var(--bg-400);
  }

  .btn:disabled {
    background: var(--bg-600);
    color: var(--text-400);
    cursor: not-allowed;
  }

  .btn.primary {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
  }

  .btn.primary:hover:not(:disabled) {
    background: var(--color-primary-hover);
  }

  .btn.secondary {
    background: var(--bg-500);
    color: var(--text-300);
  }

  .btn.danger {
    background: #f44747;
    color: white;
    border-color: #f44747;
  }

  .btn.danger:hover:not(:disabled) {
    background: #d73a49;
    border-color: #d73a49;
  }

  .result-area {
    margin-top: 16px;
    padding: 16px;
    background: var(--bg-500);
    border: 1px solid var(--border-300);
    border-radius: 4px;
    font-family: 'Consolas', 'Monaco', monospace;
    font-size: 13px;
    white-space: pre-wrap;
    max-height: 400px;
    overflow-y: auto;
  }

  .result-area.success {
    border-color: #4ec9b0;
    background: rgba(78, 201, 176, 0.1);
    color: #4ec9b0;
  }

  .result-area.error {
    border-color: #f44747;
    background: rgba(244, 71, 71, 0.1);
    color: #f44747;
  }

  .result-area.info {
    border-color: #569cd6;
    background: rgba(86, 156, 214, 0.1);
    color: #569cd6;
  }

  .result-area.loading {
    border-color: #ffcc02;
    background: rgba(255, 204, 2, 0.1);
    color: #ffcc02;
  }

  pre {
    margin: 0;
    white-space: pre-wrap;
    word-wrap: break-word;
  }
</style>
