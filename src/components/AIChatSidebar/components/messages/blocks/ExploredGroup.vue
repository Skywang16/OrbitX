<template>
  <section class="explored-group step-block">
    <button type="button" class="explored-header" :aria-expanded="expanded" @click="toggle">
      <span class="explored-left">
        <span v-if="isRunning" class="explored-spinner">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <circle
              cx="7"
              cy="7"
              r="5.5"
              stroke="var(--color-primary)"
              stroke-width="1.5"
              stroke-dasharray="8 6"
              fill="none"
            />
          </svg>
        </span>
        <span v-else class="explored-dot" />
        <span class="explored-title">
          <span class="explored-title-prefix">Explored</span>
          <span v-if="titleDetail" class="explored-title-detail">{{ titleDetail }}</span>
        </span>
      </span>
      <svg class="chevron" :class="{ expanded }" width="10" height="10" viewBox="0 0 10 10">
        <path
          d="M3.5 2.5L6 5L3.5 7.5"
          stroke="currentColor"
          stroke-width="1"
          stroke-linecap="round"
          stroke-linejoin="round"
          fill="none"
        />
      </svg>
    </button>
    <transition name="expand">
      <div v-if="expanded" class="explored-scroll">
        <GenericTool v-for="block in blocks" :key="block.id" :block="block" />
      </div>
    </transition>
  </section>
</template>

<script setup lang="ts">
  import type { Block } from '@/types'
  import { computed, ref, watch } from 'vue'
  import GenericTool from './tools/GenericTool.vue'

  type ToolItem = Extract<Block, { type: 'tool' }>

  const props = defineProps<{ blocks: ToolItem[] }>()

  const manualToggle = ref<boolean | null>(null)
  const isRunning = computed(() => props.blocks.some(b => b.status === 'running' || b.status === 'pending'))
  const expanded = computed(() => manualToggle.value ?? isRunning.value)
  const toggle = () => {
    manualToggle.value = !expanded.value
  }
  watch(isRunning, (cur, prev) => {
    if (prev && !cur) manualToggle.value = null
  })

  const titleDetail = computed(() => {
    const first = props.blocks[0]
    if (!first) return ''
    const input = (first.input as Record<string, unknown> | null) ?? {}
    const str = (v: unknown) => (typeof v === 'string' ? v.trim() : '')
    return str(input.path) || str(input.query) || str(input.pattern) || ''
  })
</script>

<style scoped>
  .explored-group {
    margin: 2px 0;
  }

  .explored-header {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    width: auto;
    padding: 4px 6px;
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    gap: 6px;
    color: var(--text-400);
    transition: background 0.1s ease;
  }

  .explored-header:hover {
    background: color-mix(in srgb, var(--bg-200) 50%, transparent);
  }

  .explored-left {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: none;
  }

  .explored-spinner {
    flex-shrink: 0;
    display: flex;
    animation: spin 1.5s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .explored-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-primary);
    flex-shrink: 0;
    opacity: 0.5;
  }

  .explored-title {
    display: flex;
    align-items: baseline;
    gap: 5px;
    min-width: 0;
    font-size: 15px;
    font-weight: 500;
  }

  .explored-title-prefix {
    color: var(--color-primary);
    font-weight: 600;
  }

  .explored-title-detail {
    color: var(--text-500);
    font-size: 13px;
    font-weight: 400;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .chevron {
    flex-shrink: 0;
    color: var(--text-500);
    opacity: 0.5;
    transition: transform 0.15s ease;
  }

  .chevron.expanded {
    transform: rotate(90deg);
  }

  .explored-scroll {
    max-height: 320px;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 1px 6px 3px 18px;
    scrollbar-width: none;
  }

  .explored-scroll::-webkit-scrollbar {
    display: none;
  }

  .expand-enter-active,
  .expand-leave-active {
    transition: all 0.2s ease;
    overflow: hidden;
  }
  .expand-enter-from,
  .expand-leave-to {
    max-height: 0;
    opacity: 0;
  }
  .expand-enter-to,
  .expand-leave-from {
    max-height: 320px;
    opacity: 1;
  }
</style>
