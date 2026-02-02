<script setup lang="ts">
  import { computed, ref, watch } from 'vue'
  import { useWorkspaceStore } from '@/stores/workspace'
  import type { Block } from '@/types'
  import AIMessage from '../AIMessage.vue'

  type SubtaskBlock = Extract<Block, { type: 'subtask' }>

  interface Props {
    block: SubtaskBlock
  }

  const props = defineProps<Props>()
  const expanded = ref(false)
  const userToggled = ref(false)
  const workspaceStore = useWorkspaceStore()
  const loadError = ref<string | null>(null)

  const isRunning = computed(() => props.block.status === 'running' || props.block.status === 'pending')
  const isError = computed(() => props.block.status === 'error')
  const isCancelled = computed(() => props.block.status === 'cancelled')

  // 只获取 assistant 消息，过滤掉 user 消息（提示词）
  const aiMessages = computed(() => {
    const messages = workspaceStore.getCachedMessages(props.block.childSessionId)
    return messages.filter(msg => msg.role === 'assistant')
  })

  // 是否有内容可以展开查看
  const hasContent = computed(() => {
    return aiMessages.value.length > 0 || isRunning.value
  })

  // 是否正在等待首条消息（运行中但还没有 AI 消息）
  const isWaitingForContent = computed(() => {
    return isRunning.value && aiMessages.value.length === 0
  })

  const toggleDetails = async () => {
    userToggled.value = true
    expanded.value = !expanded.value
    if (!expanded.value) return

    // 展开时尝试加载历史消息（如果缓存为空）
    const cachedMessages = workspaceStore.getCachedMessages(props.block.childSessionId)
    if (cachedMessages.length === 0) {
      loadError.value = null
      try {
        await workspaceStore.fetchMessages(props.block.childSessionId)
      } catch (err) {
        loadError.value = err instanceof Error ? err.message : String(err)
      }
    }
  }

  // 运行中时自动展开，完成后保持展开状态
  watch(
    () => props.block.status,
    status => {
      if (userToggled.value) return
      if (status === 'running' || status === 'pending') {
        expanded.value = true
      }
    },
    { immediate: true }
  )
</script>

<template>
  <div class="task-block" :data-status="block.status">
    <!-- 工具样式的主行 -->
    <div
      class="task-line"
      :class="{ running: isRunning, error: isError, cancelled: isCancelled }"
      @click="toggleDetails"
    >
      <span class="task-text" :class="{ running: isRunning }">
        <span class="task-prefix">{{ block.agentType }}</span>
        <span class="task-desc">{{ block.description }}</span>
      </span>
      <svg v-if="hasContent" class="chevron" :class="{ expanded }" width="10" height="10" viewBox="0 0 10 10">
        <path
          d="M3.5 2.5L6 5L3.5 7.5"
          stroke="currentColor"
          stroke-width="1"
          stroke-linecap="round"
          stroke-linejoin="round"
          fill="none"
        />
      </svg>
    </div>

    <!-- 展开的内容区 -->
    <transition name="expand">
      <div v-if="expanded" class="task-content" @click.stop>
        <!-- 错误状态 -->
        <div v-if="loadError" class="task-error">{{ loadError }}</div>

        <!-- 有 AI 消息时显示流式内容 -->
        <template v-else-if="aiMessages.length > 0">
          <div class="task-messages">
            <template v-for="msg in aiMessages" :key="msg.id">
              <AIMessage :message="msg" />
            </template>
          </div>
        </template>

        <!-- 运行中但还没有 AI 消息 -->
        <div v-else-if="isWaitingForContent" class="task-streaming">
          <span class="loading-dot"></span>
          <span>Running…</span>
        </div>

        <!-- 完成但没有输出 -->
        <div v-else class="task-empty">No output</div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
  .task-block {
    margin: 6px 0;
    font-size: 14px;
    line-height: 1.8;
  }

  .task-line {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 0;
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s ease;
    font-size: 14px;
  }

  .task-line:hover {
    color: var(--text-300);
  }

  .task-line:hover .chevron {
    opacity: 1;
  }

  .task-line.error {
    color: var(--color-error);
  }

  .task-line.cancelled {
    color: var(--text-500);
    opacity: 0.85;
  }

  .task-text {
    font-size: 14px;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .task-text.running {
    background: linear-gradient(
      90deg,
      var(--text-500) 0%,
      var(--text-500) 25%,
      var(--text-200) 50%,
      var(--text-500) 75%,
      var(--text-500) 100%
    );
    background-size: 300% 100%;
    background-clip: text;
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    animation: scan 2s linear infinite;
  }

  @keyframes scan {
    0% {
      background-position: 100% 0;
    }
    100% {
      background-position: -200% 0;
    }
  }

  .task-prefix {
    color: var(--text-400);
    font-weight: 400;
  }

  .task-desc {
    color: var(--text-500);
    font-weight: 400;
  }

  .chevron {
    flex-shrink: 0;
    color: var(--text-500);
    transition: transform 0.2s ease;
    opacity: 0.5;
  }

  .task-line:hover .chevron {
    opacity: 1;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  /* 内容区样式 */
  .task-content {
    margin-top: 8px;
    padding-left: 12px;
    border-left: 2px solid var(--border-200);
    margin-left: 2px;
  }

  .task-streaming {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 0;
    color: var(--text-400);
    font-size: 13px;
  }

  .loading-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-primary);
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  .task-error {
    padding: 8px 0;
    color: var(--color-error);
    font-size: 13px;
    white-space: pre-wrap;
  }

  .task-empty {
    padding: 8px 0;
    color: var(--text-500);
    font-size: 13px;
  }

  .task-messages {
    padding-top: 4px;
  }

  /* 过渡动画 */
  .expand-enter-active,
  .expand-leave-active {
    transition: all 0.15s ease;
  }

  .expand-enter-from,
  .expand-leave-to {
    opacity: 0;
    transform: translateY(-4px);
  }

  /* 隐藏子消息的 footer */
  .task-messages :deep(.ai-message) {
    margin-bottom: var(--spacing-sm);
  }

  .task-messages :deep(.ai-message-footer) {
    display: none;
  }
</style>
