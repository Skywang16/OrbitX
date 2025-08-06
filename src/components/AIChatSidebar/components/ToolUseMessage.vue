<template>
  <div class="tool-use-message">
    <div class="tool-header">
      <span class="tool-name">{{ message.toolName }}</span>
    </div>
    <div class="tool-params">
      <pre><code>{{ JSON.stringify(message.params, null, 2) }}</code></pre>
    </div>
  </div>
</template>

<script setup lang="ts">
  import type { AgentToolUseMessage } from '../types'

  interface Props {
    message: AgentToolUseMessage
  }

  defineProps<Props>()
</script>

<style scoped>
  .tool-use-message {
    background: var(--color-bg-secondary);
    border-radius: 8px;
    padding: 10px 14px;
    font-size: 13px;
    border: 1px solid var(--color-border);
    margin: 8px 0;
  }

  .tool-header {
    display: flex;
    align-items: center;
    font-weight: 500;
    color: var(--color-text-secondary);
  }

  .tool-params {
    margin-top: 8px;
    background: var(--color-bg-tertiary);
    border-radius: 4px;
    padding: 8px;
    border: 1px solid var(--color-border-secondary);
  }

  .tool-params pre {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-all;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 12px;
    color: var(--color-text-primary);
  }
</style>
