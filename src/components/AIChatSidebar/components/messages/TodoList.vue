<script setup lang="ts">
  import { computed } from 'vue'

  interface TodoItem {
    content: string
    activeForm: string
    status: 'pending' | 'in_progress' | 'completed'
  }

  interface Props {
    todos: TodoItem[]
  }

  const props = defineProps<Props>()

  const statusIcon = (status: TodoItem['status']) => {
    switch (status) {
      case 'pending':
        return '⏳'
      case 'in_progress':
        return '🔄'
      case 'completed':
        return '✅'
    }
  }

  const statusClass = (status: TodoItem['status']) => {
    switch (status) {
      case 'pending':
        return 'todo-pending'
      case 'in_progress':
        return 'todo-in-progress'
      case 'completed':
        return 'todo-completed'
    }
  }

  const displayText = (todo: TodoItem) => {
    return todo.status === 'in_progress' ? todo.activeForm : todo.content
  }

  const stats = computed(() => {
    const total = props.todos.length
    const completed = props.todos.filter(t => t.status === 'completed').length
    const inProgress = props.todos.filter(t => t.status === 'in_progress').length
    const pending = props.todos.filter(t => t.status === 'pending').length
    return { total, completed, inProgress, pending }
  })
</script>

<template>
  <div class="todo-list-container">
    <!-- Header -->
    <div class="todo-header">
      <div class="todo-title">
        <span class="icon">📋</span>
        <span>Task Plan</span>
      </div>
      <div class="todo-stats">
        <span class="stat-item">{{ stats.completed }}/{{ stats.total }}</span>
      </div>
    </div>

    <!-- Todo Items -->
    <div class="todo-items">
      <div v-for="(todo, index) in todos" :key="index" :class="['todo-item', statusClass(todo.status)]">
        <span class="todo-icon">{{ statusIcon(todo.status) }}</span>
        <span class="todo-text">{{ displayText(todo) }}</span>
      </div>
    </div>

    <!-- Progress Bar -->
    <div v-if="stats.total > 0" class="progress-bar-container">
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: `${(stats.completed / stats.total) * 100}%` }"></div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .todo-list-container {
    background: rgba(var(--v-theme-surface), 0.6);
    border-radius: 8px;
    padding: 12px;
    margin: 8px 0;
    border-left: 3px solid rgb(var(--v-theme-primary));
  }

  .todo-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .todo-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
    font-size: 14px;
    color: rgb(var(--v-theme-on-surface));
  }

  .todo-title .icon {
    font-size: 16px;
  }

  .todo-stats {
    font-size: 12px;
    color: rgba(var(--v-theme-on-surface), 0.6);
    font-weight: 500;
  }

  .todo-items {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .todo-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 13px;
    transition: all 0.2s ease;
  }

  .todo-item.todo-pending {
    background: rgba(var(--v-theme-warning), 0.1);
    color: rgb(var(--v-theme-on-surface));
  }

  .todo-item.todo-in-progress {
    background: rgba(var(--v-theme-info), 0.15);
    color: rgb(var(--v-theme-on-surface));
    font-weight: 500;
    animation: pulse 2s infinite;
  }

  .todo-item.todo-completed {
    background: rgba(var(--v-theme-success), 0.1);
    color: rgba(var(--v-theme-on-surface), 0.6);
    text-decoration: line-through;
  }

  .todo-icon {
    font-size: 16px;
    flex-shrink: 0;
  }

  .todo-text {
    flex: 1;
    line-height: 1.4;
  }

  .progress-bar-container {
    margin-top: 12px;
  }

  .progress-bar {
    height: 4px;
    background: rgba(var(--v-theme-surface-variant), 0.3);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, rgb(var(--v-theme-primary)), rgb(var(--v-theme-secondary)));
    transition: width 0.3s ease;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.8;
    }
  }
</style>
