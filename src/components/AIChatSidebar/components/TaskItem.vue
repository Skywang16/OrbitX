<template>
  <div
    class="task-node"
    :class="{
      'is-current': isCurrent,
      'is-active': task.status === 'running',
      'is-paused': task.status === 'paused',
      'is-completed': task.status === 'completed',
      'is-error': task.status === 'error',
    }"
    @click="$emit('click')"
  >
    <!-- Status indicator -->
    <div class="status-indicator">
      <div class="status-dot" :class="getStatusClass(task.status)"></div>
    </div>

    <!-- Task information -->
    <div class="task-info">
      <div class="task-title">{{ displayName }}</div>
      <div class="task-meta">
        <span class="task-status" :class="getStatusClass(task.status)">{{ getStatusText(task.status) }}</span>
        <span class="node-count">iter: {{ task.current_iteration }}</span>
        <span v-if="task.error" class="task-type">error: {{ task.error }}</span>
      </div>
    </div>

    <!-- Progress indicator for active tasks (keep original style) -->
    <div v-if="task.status === 'running' && hasProgress" class="progress-indicator">
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: `${progressPercentage}%` }"></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed } from 'vue'
  import type { AgentTaskState } from '@/api/agent'

  interface Props {
    task: AgentTaskState
    isCurrent?: boolean
  }

  const props = withDefaults(defineProps<Props>(), {
    isCurrent: false,
  })

  defineEmits<{
    click: []
  }>()

  // Derive a display name similar to previous UI's title
  const displayName = computed(() => {
    // 尝试使用用户提示作为标题的第一行，否则回退到task_id后缀
    const prompt = props.task.user_prompt
    if (prompt && prompt.trim()) {
      return (prompt.split('\n')[0] || '').slice(0, 60) || props.task.task_id
    }
    return `Task-${props.task.task_id.slice(-8)}`
  })

  // Keep progress bar UI: approximate progress by iteration (purely visual)
  const hasProgress = computed(() => true)
  const progressPercentage = computed(() => {
    const iter = props.task.current_iteration || 0
    // 显示一个0-95%的进度，避免满格直到完成事件
    return Math.min(95, Math.max(0, (iter % 10) * 10))
  })

  const getStatusClass = (status: string) => {
    switch (status) {
      case 'running':
        return 'status-active'
      case 'paused':
        return 'status-paused'
      case 'completed':
        return 'status-completed'
      case 'error':
        return 'status-error'
      case 'cancelled':
        return 'status-init'
      default:
        return 'status-init'
    }
  }

  const getStatusText = (status: string) => {
    switch (status) {
      case 'running':
        return 'Running'
      case 'paused':
        return 'Paused'
      case 'completed':
        return 'Completed'
      case 'error':
        return 'Error'
      case 'created':
        return 'Created'
      case 'cancelled':
        return 'Cancelled'
      default:
        return 'Init'
    }
  }
</script>

<style scoped>
  .task-node {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-100);
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    transition: all 0.15s ease;
    position: relative;
  }

  .task-node:last-child {
    border-bottom: none;
  }

  .task-node:hover {
    background: var(--bg-200);
  }

  .task-node.is-current {
    background: var(--bg-300);
    border-left: 2px solid #007acc;
    padding-left: 8px;
  }

  .task-node.is-active {
    background: rgba(0, 122, 204, 0.05);
  }

  .task-node.is-paused {
    background: rgba(255, 193, 7, 0.05);
  }

  .task-node.is-completed {
    opacity: 0.7;
  }

  .task-node.is-error {
    background: rgba(244, 67, 54, 0.05);
  }

  /* Status indicator */
  .status-indicator {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #9aa0a6;
  }

  .status-dot.status-active {
    background: #007acc;
    animation: pulse 2s ease-in-out infinite;
  }

  .status-dot.status-paused {
    background: #ffc107;
  }

  .status-dot.status-completed {
    background: #4caf50;
  }

  .status-dot.status-error {
    background: #f44336;
  }

  .status-dot.status-init {
    background: #9aa0a6;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }

  .task-info {
    flex: 1;
    min-width: 0;
  }

  .task-title {
    font-size: 12px;
    line-height: 1.3;
    color: var(--text-200);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 1px;
  }

  .task-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-400);
    font-size: 10px;
  }

  .task-status {
    text-transform: capitalize;
  }

  .node-count {
    opacity: 0.7;
  }

  .task-type {
    opacity: 0.6;
    font-style: italic;
  }

  /* Progress indicator */
  .progress-indicator {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 2px;
  }

  .progress-bar {
    width: 100%;
    height: 100%;
    background: var(--border-100);
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: #007acc;
    transition: width 0.3s ease;
  }
</style>
