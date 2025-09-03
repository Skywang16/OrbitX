<template>
  <div v-if="taskNodes.length > 0" class="task-list">
    <div class="task-list-header" @click="toggleCollapsed">
      <div class="task-list-title">
        <span class="task-title-text">TodoList</span>
        <span class="task-count">({{ taskNodes.length }})</span>
      </div>
      <div class="collapse-button" :title="isCollapsed ? '展开' : '收起'">
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          :class="{ rotated: isCollapsed }"
        >
          <polyline points="6,9 12,15 18,9"></polyline>
        </svg>
      </div>
    </div>

    <div v-if="!isCollapsed" class="task-list-content">
      <div v-for="(node, index) in taskNodes" :key="`${taskId}-${index}`" class="task-item">
        <div class="task-item-content">
          <div class="task-item-indicator">
            <div
              class="task-dot"
              :class="{
                'task-dot-pending': node.status === 'pending',
                'task-dot-running': node.status === 'running',
                'task-dot-completed': node.status === 'completed',
              }"
            ></div>
          </div>
          <div
            class="task-item-text"
            :class="{
              'task-text-pending': node.status === 'pending',
              'task-text-running': node.status === 'running',
              'task-text-completed': node.status === 'completed',
            }"
          >
            {{ node.text }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, watch } from 'vue'

  interface TaskNode {
    type: string
    text: string
    status?: 'pending' | 'running' | 'completed'
  }

  interface Props {
    taskNodes: TaskNode[]
    taskId?: string
  }

  const props = withDefaults(defineProps<Props>(), {
    taskNodes: () => [],
    taskId: '',
  })

  const isCollapsed = ref(true)

  const toggleCollapsed = () => {
    isCollapsed.value = !isCollapsed.value
  }

  // 监听任务节点变化，如果有新任务自动展开
  watch(
    () => props.taskNodes.length,
    (newLength, oldLength) => {
      if (newLength > oldLength && newLength > 0) {
        isCollapsed.value = false
      }
    }
  )
</script>

<style scoped>
  .task-list {
    margin: 0 10px 0 10px;
    border-radius: 6px 6px 0 0;
    background-color: rgba(37, 37, 38, 0.8); /* 半透明背景 */
    border: 1px solid var(--border-200);
    border-bottom: none;
    overflow: hidden;
    backdrop-filter: blur(2px); /* 轻微模糊效果 */
  }

  .task-list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    background-color: rgba(37, 37, 38, 0.9); /* 与task-list背景一致但稍微不透明 */
    border-bottom: 1px solid var(--border-200);
    cursor: pointer;
    user-select: none;
  }

  .task-list-header:hover {
    background-color: rgba(255, 255, 255, 0.08); /* 更明显的悬停效果 */
  }

  .task-list-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--font-size-xs);
    font-weight: 500;
    color: var(--text-300);
  }

  .task-title-text {
    color: var(--text-200);
  }

  .task-count {
    color: var(--text-400);
    font-size: 11px;
  }

  .collapse-button {
    color: var(--text-400);
    padding: 2px;
    transition: all 0.2s ease;
  }

  .collapse-button:hover {
    background-color: var(--color-hover);
    color: var(--text-300);
  }

  .collapse-button svg {
    transition: transform 0.2s ease;
  }

  .collapse-button svg.rotated {
    transform: rotate(-90deg);
  }

  .task-list-content {
    padding: 4px 0;
    background-color: transparent;
  }

  .task-item {
    padding: 4px 10px;
    transition: background-color 0.2s ease;
  }

  .task-item:hover {
    background-color: rgba(255, 255, 255, 0.08); /* 更明显的悬停效果 */
  }

  .task-item-content {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .task-item-indicator {
    padding-top: 2px;
    flex-shrink: 0;
  }

  .task-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--color-primary);
    transition: all 0.3s ease;
  }

  .task-dot-pending {
    background-color: var(--text-400);
    opacity: 0.5;
  }

  .task-dot-running {
    background-color: var(--color-primary);
    animation: pulse 1.5s ease-in-out infinite;
  }

  .task-dot-completed {
    background-color: var(--color-success, #10b981);
  }

  .task-item-text {
    flex: 1;
    font-size: var(--font-size-sm);
    line-height: 1.4;
    color: var(--text-300);
    word-wrap: break-word;
    transition: all 0.3s ease;
  }

  .task-text-pending {
    color: var(--text-400);
    opacity: 0.7;
  }

  .task-text-running {
    color: var(--text-200);
    font-weight: 500;
  }

  .task-text-completed {
    color: var(--text-400);
    text-decoration: line-through;
    opacity: 0.8;
  }

  @keyframes pulse {
    0% {
      transform: scale(1);
      opacity: 1;
    }
    50% {
      transform: scale(1.2);
      opacity: 0.7;
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }
</style>
