<template>
  <div v-if="taskNodes.length > 0" class="task-list">
    <div class="task-header" @click="toggleCollapsed">
      <div class="header-left">
        <svg class="chevron-icon" :class="{ expanded: !isCollapsed }" width="14" height="14" viewBox="0 0 24 24">
          <path
            d="M9 18l6-6-6-6"
            stroke="currentColor"
            stroke-width="2"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <svg class="tasks-icon" width="14" height="14" viewBox="0 0 24 24">
          <path
            d="M9 11l3 3 8-8"
            stroke="currentColor"
            stroke-width="2"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
          <path
            d="M21 12c0 4.97-4.03 9-9 9s-9-4.03-9-9 4.03-9 9-9c1.51 0 2.93.37 4.18 1.03"
            stroke="currentColor"
            stroke-width="2"
            fill="none"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <span class="header-title">TODOLIST</span>
      </div>
      <div class="header-right">
        <span class="task-counter">{{ completedCount }}/{{ taskNodes.length }}</span>
      </div>
    </div>

    <transition name="collapse">
      <div v-show="!isCollapsed" class="task-content">
        <div
          v-for="(node, index) in taskNodes"
          :key="`${taskId}-${index}`"
          class="task-item"
          :class="{
            'task-pending': node.status === 'pending',
            'task-running': node.status === 'running',
            'task-completed': node.status === 'completed',
          }"
        >
          <div class="task-status">
            <!-- 待处理状态 -->
            <svg
              v-if="node.status === 'pending'"
              class="status-icon pending"
              width="14"
              height="14"
              viewBox="0 0 16 16"
            >
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5" fill="none" />
            </svg>

            <!-- 运行中状态 -->
            <svg
              v-else-if="node.status === 'running'"
              class="status-icon running"
              width="14"
              height="14"
              viewBox="0 0 16 16"
            >
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5" fill="none" />
              <circle cx="8" cy="8" r="2" fill="currentColor" />
            </svg>

            <!-- 已完成状态 -->
            <svg
              v-else-if="node.status === 'completed'"
              class="status-icon completed"
              width="14"
              height="14"
              viewBox="0 0 16 16"
            >
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5" fill="none" />
              <path d="M5.5 8l2 2 3.5-3.5" stroke="currentColor" stroke-width="1.5" fill="none" />
            </svg>

            <!-- 默认状态 -->
            <svg v-else class="status-icon default" width="14" height="14" viewBox="0 0 16 16">
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5" fill="none" />
            </svg>
          </div>

          <div class="task-text">{{ node.text }}</div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
  import { ref, watch, computed } from 'vue'

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

  // 计算完成的任务数量
  const completedCount = computed(() => {
    return props.taskNodes.filter(node => node.status === 'completed').length
  })

  // 监听任务节点变化，如果有新任务自动展开
  watch(
    () => props.taskNodes.length,
    (newLength, oldLength) => {
      if (newLength > oldLength && newLength > 0) {
        isCollapsed.value = false
      }
    }
  )

  // 监听taskId变化，新会话时自动展开
  watch(
    () => props.taskId,
    (newTaskId, oldTaskId) => {
      if (newTaskId && newTaskId !== oldTaskId && props.taskNodes.length > 0) {
        isCollapsed.value = false
      }
    }
  )

  // 记录上一次的节点数量
  const previousNodeCount = ref(0)

  // 监听nodes数组变化，新增节点时说明上一个任务完成
  watch(
    () => props.taskNodes.length,
    newCount => {
      if (newCount > previousNodeCount.value && previousNodeCount.value > 0) {
        // 有新节点添加，说明上一个任务完成，自动收回
        isCollapsed.value = true
      }
      previousNodeCount.value = newCount
    }
  )
</script>

<style scoped>
  .task-list {
    margin: 6px 10px 0 10px;
    background-color: var(--bg-400);
    border: 1px solid var(--border-200);
    border-bottom: none;
    border-radius: var(--border-radius) var(--border-radius) 0 0;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    font-size: 13px;
    box-shadow: none;
    width: calc(90% - 20px);
    margin-left: auto;
    margin-right: auto;
  }

  .task-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background-color: var(--bg-400);
    border-bottom: none;
    border-radius: var(--border-radius) var(--border-radius) 0 0;
    cursor: pointer;
    user-select: none;
    transition: background-color 0.2s ease;
    min-height: 28px;
    position: relative;
    z-index: 1;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .header-right {
    display: flex;
    align-items: center;
  }

  .chevron-icon {
    color: var(--text-400);
    transition: transform 0.2s ease;
    flex-shrink: 0;
    width: 12px;
    height: 12px;
    display: block;
  }

  .chevron-icon.expanded {
    transform: rotate(90deg);
  }

  .tasks-icon {
    color: var(--text-300);
    flex-shrink: 0;
    width: 12px;
    height: 12px;
    display: block;
  }

  .header-title {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-300);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .task-counter {
    font-size: 11px;
    color: var(--text-400);
    background-color: var(--bg-400);
    padding: 2px 6px;
    border-radius: var(--border-radius-lg);
    font-weight: 500;
  }

  .task-content {
    background-color: var(--bg-400);
    max-height: 240px;
    overflow-y: auto;
  }

  .task-item {
    display: flex;
    align-items: center;
    padding: 4px 12px;
    border-bottom: 1px solid var(--border-200);
    transition: background-color 0.2s ease;
    min-height: 24px;
  }

  .task-item:last-child {
    border-bottom: none;
  }

  .task-item:hover {
    background-color: var(--color-hover);
  }

  .task-running {
    background-color: rgba(0, 122, 204, 0.1);
  }

  .task-running:hover {
    background-color: rgba(0, 122, 204, 0.15);
  }

  .task-status {
    margin-right: 8px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .status-icon {
    transition: all 0.2s ease;
  }

  .status-icon.pending {
    color: #6c6c6c;
  }

  .status-icon.running {
    color: #007acc;
    animation: pulse 1.5s ease-in-out infinite;
  }

  .status-icon.completed {
    color: #4caf50;
  }

  .status-icon.default {
    color: #6c6c6c;
    opacity: 0.6;
  }

  .task-text {
    flex: 1;
    font-size: 13px;
    line-height: 1.4;
    color: var(--text-300);
    word-wrap: break-word;
    transition: all 0.2s ease;
  }

  .task-pending .task-text {
    color: var(--text-400);
  }

  .task-running .task-text {
    color: var(--text-200);
    font-weight: 500;
  }

  .task-completed .task-text {
    color: var(--text-400);
    text-decoration: line-through;
    opacity: 0.8;
  }

  @keyframes pulse {
    0% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
    100% {
      opacity: 1;
    }
  }

  .collapse-enter-active,
  .collapse-leave-active {
    transition: all 0.3s ease;
    overflow: hidden;
  }

  .collapse-enter-from,
  .collapse-leave-to {
    max-height: 0;
    opacity: 0;
  }

  .collapse-enter-to,
  .collapse-leave-from {
    max-height: 240px;
    opacity: 1;
  }
</style>
