<template>
  <div class="todo-block">
    <div class="todo-header" :class="{ clickable: todoItems.length > 0 }" @click="todoExpanded = !todoExpanded">
      <span class="todo-summary">
        <span v-if="isRunning" class="todo-summary-text todo-summary-running">
          {{ progress ? `${progress} tasks done` : 'Processing…' }}
        </span>
        <span v-else class="todo-summary-text">{{ progress }} tasks done</span>
      </span>
      <svg
        v-if="todoItems.length > 0"
        class="todo-chevron"
        :class="{ expanded: todoExpanded }"
        width="10"
        height="10"
        viewBox="0 0 10 10"
      >
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
    <transition name="todo-expand">
      <div v-if="todoExpanded && todoItems.length > 0" class="todo-list">
        <div v-for="(item, idx) in todoItems" :key="idx" class="todo-item" :class="item.status">
          <span class="todo-icon">
            <svg v-if="item.status === 'completed'" width="14" height="14" viewBox="0 0 14 14" fill="none">
              <circle
                cx="7"
                cy="7"
                r="6.5"
                stroke="var(--color-success)"
                stroke-width="1"
                fill="color-mix(in srgb, var(--color-success) 15%, transparent)"
              />
              <path
                d="M4.5 7L6.5 9L9.5 5"
                stroke="var(--color-success)"
                stroke-width="1.2"
                stroke-linecap="round"
                stroke-linejoin="round"
                fill="none"
              />
            </svg>
            <svg
              v-else-if="item.status === 'in_progress'"
              width="14"
              height="14"
              viewBox="0 0 14 14"
              fill="none"
              class="todo-spinner"
            >
              <circle
                cx="7"
                cy="7"
                r="6.5"
                stroke="var(--color-info)"
                stroke-width="1"
                stroke-dasharray="4 3"
                fill="none"
              />
              <circle cx="7" cy="7" r="2" fill="var(--color-info)" />
            </svg>
            <svg v-else width="14" height="14" viewBox="0 0 14 14" fill="none">
              <circle cx="7" cy="7" r="6.5" stroke="var(--text-500)" stroke-width="1" fill="none" />
            </svg>
          </span>
          <span class="todo-text">{{ cleanContent(item.content) }}</span>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
  import type { Block } from '@/types'
  import { computed, ref } from 'vue'

  const props = defineProps<{
    block: Extract<Block, { type: 'tool' }>
  }>()

  const todoExpanded = ref(true)

  interface TodoItem {
    content: string
    status: 'pending' | 'in_progress' | 'completed'
  }

  const isRunning = computed(() => props.block.status === 'running' || props.block.status === 'pending')

  const todoItems = computed<TodoItem[]>(() => {
    const input = props.block.input as { todos?: TodoItem[] } | undefined
    return input?.todos || []
  })

  const progress = computed(() => {
    if (todoItems.value.length === 0) return ''
    const done = todoItems.value.filter(t => t.status === 'completed').length
    return `${done}/${todoItems.value.length}`
  })

  // Strip leading status emoji that the AI sometimes adds to content (e.g. ✅ ⏳)
  const cleanContent = (text: string) => {
    let s = text.trimStart()
    const emojis = ['✅', '⏳', '🔄', '⭕', '❌', '⚠️']
    let changed = true
    while (changed) {
      changed = false
      for (const e of emojis) {
        if (s.startsWith(e)) {
          s = s.slice(e.length).trimStart()
          changed = true
        }
      }
    }
    return s
  }
</script>

<style scoped>
  .todo-block {
    margin: 6px 0;
    font-size: 13px;
    background: var(--bg-100);
    border: 1px solid var(--border-200);
    border-radius: 10px;
    overflow: hidden;
  }

  .todo-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    color: var(--text-400);
    user-select: none;
  }

  .todo-header.clickable {
    cursor: pointer;
  }
  .todo-header.clickable:hover {
    color: var(--text-300);
  }

  .todo-summary-text {
    font-size: 13px;
    color: var(--text-400);
  }

  .todo-summary-running {
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

  .todo-chevron {
    flex-shrink: 0;
    color: var(--text-500);
    transition: transform 0.2s ease;
    transform: rotate(90deg);
  }

  .todo-chevron.expanded {
    transform: rotate(270deg);
  }

  .todo-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 0 12px 8px;
  }

  .todo-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 3px 0;
    color: var(--text-400);
    font-size: 13px;
    line-height: 1.4;
  }

  .todo-item.in_progress {
    color: var(--text-300);
  }

  .todo-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 2px;
  }

  .todo-spinner {
    animation: spin 2s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .todo-text {
    flex: 1;
    min-width: 0;
  }

  .todo-expand-enter-active,
  .todo-expand-leave-active {
    transition: all 0.2s ease;
    overflow: hidden;
  }

  .todo-expand-enter-from,
  .todo-expand-leave-to {
    max-height: 0;
    opacity: 0;
    padding-top: 0;
    padding-bottom: 0;
  }

  .todo-expand-enter-to,
  .todo-expand-leave-from {
    max-height: 500px;
    opacity: 1;
  }
</style>
