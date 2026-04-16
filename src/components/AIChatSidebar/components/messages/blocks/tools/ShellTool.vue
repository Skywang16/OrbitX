<template>
  <div class="tool-block-shell">
    <div class="shell-header">
      <div class="shell-info">
        <svg
          class="shell-icon"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <polyline points="4 17 10 11 4 5"></polyline>
          <line x1="12" y1="19" x2="20" y2="19"></line>
        </svg>
        <span class="shell-command" :class="{ running: isRunning }" :title="commandDisplay">
          {{ commandDisplay }}
        </span>
      </div>
    </div>
    <div v-if="paneId !== null" class="shell-terminal" :class="{ 'disabled-interaction': !isRunning }">
      <Terminal :terminal-id="paneId" :is-active="false" :disable-stdin="!isRunning" />
    </div>
    <div v-else-if="!isRunning" class="shell-content">
      <pre class="shell-output">{{ outputSnapshot }}</pre>
    </div>
    <div v-else class="shell-content">
      <pre class="shell-output">Waiting for terminal…</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
  import Terminal from '@/components/terminal/Terminal.vue'
  import type { Block } from '@/types'
  import stripAnsi from 'strip-ansi'
  import { computed } from 'vue'

  const props = defineProps<{
    block: Extract<Block, { type: 'tool' }>
  }>()

  const isRunning = computed(() => props.block.status === 'running' || props.block.status === 'pending')
  const metadata = computed(() => props.block.output?.metadata as Record<string, unknown> | undefined)
  const params = computed(() => (props.block.input as Record<string, unknown>) || {})

  const paneId = computed<number | null>(() => {
    const id = metadata.value?.paneId
    return typeof id === 'number' ? id : null
  })

  const commandDisplay = computed(() => {
    const metaCmd = metadata.value?.command
    if (typeof metaCmd === 'string' && metaCmd) return metaCmd
    const inputCmd = params.value?.command
    return typeof inputCmd === 'string' ? inputCmd : ''
  })

  const outputSnapshot = computed(() => {
    const r = props.block.output?.content
    if (typeof r === 'string' && r) return stripAnsi(r)
    return 'No output'
  })
</script>

<style scoped>
  .tool-block-shell {
    background: var(--bg-100);
    border: 1px solid var(--border-200);
    border-radius: 10px;
    overflow: hidden;
    margin: 6px 0;
  }

  .shell-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    background: var(--color-primary-alpha);
    border-bottom: 1px solid var(--border-200);
  }

  .shell-info {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .shell-icon {
    color: var(--color-primary);
    flex-shrink: 0;
  }

  .shell-command {
    font-family: var(--font-family-mono), 'Apple Color Emoji', 'Segoe UI Emoji', emoji;
    font-size: 13px;
    color: var(--text-300);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .shell-command.running {
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

  .shell-terminal {
    height: 180px;
    overflow: hidden;
  }

  .shell-terminal :deep(.terminal-wrapper) {
    height: 100%;
    background: transparent;
    padding: 8px;
  }

  .shell-terminal :deep(.terminal-container) {
    background: transparent;
  }

  .shell-terminal.disabled-interaction {
    pointer-events: none;
    user-select: none;
    opacity: 0.55;
  }

  .shell-terminal.disabled-interaction :deep(.xterm-cursor-layer) {
    display: none !important;
  }

  .shell-content {
    max-height: 300px;
    overflow-y: auto;
    background: var(--bg-100);
  }

  .shell-output {
    margin: 0;
    padding: 12px;
    font-family: var(--font-family-mono), 'Apple Color Emoji', 'Segoe UI Emoji', emoji;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-400);
    white-space: pre-wrap;
    word-break: break-word;
    width: 100%;
  }
</style>
