<template>
  <div v-if="showCompletion" class="completion-suggestion" :style="completionStyle">
    <span class="completion-text">{{ completionText }}</span>
    <span class="completion-hint">{{ shortcutHint }}</span>
  </div>
</template>

<script setup lang="ts">
  import { completionApi } from '@/api'
  import type { CompletionRequest, CompletionResponse } from '@/api'
  import { computed, ref, watch, onMounted, onUnmounted } from 'vue'
  import { debounce } from 'lodash-es'

  // Props
  interface Props {
    input: string
    workingDirectory: string
    terminalElement?: HTMLElement | null
    terminalCursorPosition?: { x: number; y: number }
    isMac?: boolean
  }

  const props = defineProps<Props>()

  // Emits
  interface Emits {
    (e: 'completion-ready', items: any[]): void
    (e: 'suggestion-change', suggestion: string): void
  }

  const emit = defineEmits<Emits>()

  const completionItems = ref<any[]>([])
  const currentSuggestion = ref('')
  const isLoading = ref(false)
  let currentRequest: AbortController | null = null

  const showCompletion = computed(() => {
    return props.input.length > 0 && currentSuggestion.value.length > 0 && completionText.value.length > 0
  })

  const shortcutHint = computed(() => {
    return props.isMac ? 'Cmd+→' : 'Ctrl+→'
  })

  const completionText = computed(() => {
    if (!currentSuggestion.value || !props.input) return ''

    const input = props.input.toLowerCase()
    const suggestion = currentSuggestion.value.toLowerCase()

    if (suggestion.startsWith(input)) {
      return currentSuggestion.value.slice(props.input.length)
    }
    return ''
  })

  const completionStyle = computed(() => {
    if (!props.terminalElement || !showCompletion.value) return {}

    // 如果有传递的终端光标位置，直接使用
    if (props.terminalCursorPosition && props.terminalCursorPosition.x > 0 && props.terminalCursorPosition.y > 0) {
      const { x, y } = props.terminalCursorPosition

      // 获取终端包装器的位置（补全组件的定位上下文）
      const wrapperElement = props.terminalElement.parentElement
      if (!wrapperElement) return {}

      const wrapperRect = wrapperElement.getBoundingClientRect()

      // 计算相对于包装器的位置
      const relativeX = x - wrapperRect.left
      const relativeY = y - wrapperRect.top

      // 确保补全提示不会超出包装器边界
      const maxX = wrapperRect.width - 200 // 预留补全提示的宽度
      const maxY = wrapperRect.height - 30 // 预留补全提示的高度

      const finalX = Math.min(Math.max(0, relativeX), maxX) + 20
      const finalY = Math.min(Math.max(0, relativeY), maxY)

      return {
        left: `${finalX}px`,
        top: `${finalY}px`,
        zIndex: '1000',
      }
    }

    return {}
  })

  // 本地补全后备方案
  const getLocalCompletions = (input: string): CompletionResponse => {
    const commands = [
      'ls',
      'ls -la',
      'ls -l',
      'cd',
      'cd ..',
      'pwd',
      'mkdir',
      'touch',
      'rm',
      'rm -rf',
      'cp',
      'cp -r',
      'mv',
      'cat',
      'grep',
      'find',
      'which',
      'history',
      'clear',
      'exit',
      'git status',
      'git add',
      'git add .',
      'git commit -m',
      'git push',
      'git pull',
      'npm install',
      'npm run dev',
      'npm run build',
      'npm start',
      'yarn install',
      'yarn dev',
    ]

    const matches = commands
      .filter(cmd => cmd.toLowerCase().startsWith(input.toLowerCase()))
      .slice(0, 10)
      .map(cmd => ({
        text: cmd,
        displayText: cmd,
        description: `Command: ${cmd}`,
        kind: 'command',
        score: 1.0,
        source: 'local',
      }))

    return {
      items: matches,
      replaceStart: 0,
      replaceEnd: input.length,
      hasMore: false,
    }
  }

  // 获取补全建议的核心逻辑
  const fetchCompletions = async (input: string) => {
    if (!input || input.length === 0) {
      completionItems.value = []
      currentSuggestion.value = ''
      emit('completion-ready', [])
      emit('suggestion-change', '')
      return
    }

    // 取消之前的请求
    if (currentRequest) {
      currentRequest.abort()
    }

    // 创建新的请求控制器
    currentRequest = new AbortController()

    try {
      isLoading.value = true

      const request: CompletionRequest = {
        input,
        cursorPosition: input.length,
        workingDirectory: props.workingDirectory,
        maxResults: 10,
      }

      let response: CompletionResponse

      try {
        // 尝试调用后端API
        response = await completionApi.getCompletions(request)
      } catch (error: unknown) {
        // 如果是取消错误，直接返回
        if (error instanceof Error && error.message === 'Request was aborted') return

        // 使用本地补全作为后备方案
        response = getLocalCompletions(input)
      }

      completionItems.value = response.items
      emit('completion-ready', response.items)

      // 设置第一个匹配项作为内联补全
      if (response.items.length > 0) {
        const firstItem = response.items[0]
        if (firstItem.text.toLowerCase().startsWith(input.toLowerCase())) {
          currentSuggestion.value = firstItem.text
        } else {
          currentSuggestion.value = ''
        }
      } else {
        currentSuggestion.value = ''
      }

      emit('suggestion-change', currentSuggestion.value)
    } catch (error) {
      // 重置状态
      currentSuggestion.value = ''
      completionItems.value = []
      emit('completion-ready', [])
      emit('suggestion-change', '')
      // 可以考虑向用户显示错误提示，但这里选择静默处理
      // 因为补全失败不应该中断用户的正常操作
    } finally {
      isLoading.value = false
    }
  }

  // 使用lodash防抖的补全函数
  const debouncedFetchCompletions = debounce(fetchCompletions, 150)

  // 监听输入变化
  watch(
    () => props.input,
    newInput => {
      debouncedFetchCompletions(newInput)
    },
    { immediate: true }
  )

  /**
   * 接受当前的补全建议
   * 清除当前的补全状态，因为补全已被接受
   */
  const acceptCompletion = () => {
    const completionToAccept = completionText.value
    if (completionToAccept && completionToAccept.trim()) {
      // 清除当前补全状态
      currentSuggestion.value = ''
      completionItems.value = []
      emit('suggestion-change', '')
      emit('completion-ready', [])
      return completionToAccept
    }
    return ''
  }

  /**
   * 检查是否有可用的补全建议
   */
  const hasCompletion = () => showCompletion.value && !!completionText.value && completionText.value.length > 0

  // 处理快捷键触发的补全接受
  const handleAcceptCompletionEvent = (event: Event) => {
    if (event.type === 'accept-completion') {
      const result = acceptCompletion()
      if (result) {
        // 触发一个自定义事件，让父组件（Terminal）知道有补全被接受
        const detailEvent = new CustomEvent('completion-accepted', {
          detail: { completion: result },
          bubbles: true,
        })
        event.target?.dispatchEvent(detailEvent)
      }
    }
  }

  // 添加事件监听
  onMounted(() => {
    if (props.terminalElement) {
      props.terminalElement.addEventListener('accept-completion', handleAcceptCompletionEvent)
    }
  })

  onUnmounted(() => {
    if (props.terminalElement) {
      props.terminalElement.removeEventListener('accept-completion', handleAcceptCompletionEvent)
    }
  })

  // 暴露方法给父组件
  defineExpose({
    getCompletionText: () => completionText.value,
    acceptCompletion,
    hasCompletion,
  })
</script>

<style scoped>
  .completion-suggestion {
    position: absolute;
    pointer-events: none;
    z-index: 1000;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .completion-text {
    color: var(--text-400);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-md);
    background: var(--bg-500);
    padding: 1px 4px;
    border-radius: var(--border-radius-xs);
    border: 1px solid var(--border-300);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 60vw;
  }

  .completion-hint {
    color: var(--text-500);
    font-family: var(--font-family-mono);
    font-size: var(--font-size-xs);
    background: var(--bg-400);
    padding: 2px 6px;
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--border-200);
    opacity: 0.7;
  }
</style>
