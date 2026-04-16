<template>
  <section class="research-group step-block">
    <button type="button" class="research-header" :aria-expanded="expanded" @click="toggle">
      <span class="research-left">
        <span v-if="isRunning" class="research-spinner">
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
        <span v-else class="research-dot" />
        <span class="research-title">
          <span class="research-title-prefix">Research</span>
          <span v-if="titleDetail" class="research-title-detail">{{ titleDetail }}</span>
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
      <div v-if="expanded" class="research-scroll">
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
    const url = str(input.url)
    if (url) {
      try {
        return new URL(url).hostname
      } catch {
        return url
      }
    }
    return str(input.query) || str(input.q) || ''
  })
</script>

<style scoped>
  .research-group {
    margin: 2px 0;
  }

  .research-header {
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

  .research-header:hover {
    background: color-mix(in srgb, var(--bg-200) 50%, transparent);
  }

  .research-left {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: none;
  }

  .research-spinner {
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

  .research-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-primary);
    flex-shrink: 0;
    opacity: 0.5;
  }

  .research-title {
    display: flex;
    align-items: baseline;
    gap: 5px;
    min-width: 0;
    font-size: 15px;
    font-weight: 500;
  }

  .research-title-prefix {
    color: var(--color-primary);
    font-weight: 600;
  }

  .research-title-detail {
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

  .research-scroll {
    max-height: 320px;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 1px 6px 3px 18px;
    scrollbar-width: none;
  }

  .research-scroll::-webkit-scrollbar {
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
