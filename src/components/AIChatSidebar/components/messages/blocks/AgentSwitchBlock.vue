<script setup lang="ts">
  import type { Block } from '@/types'

  type AgentSwitchBlock = Extract<Block, { type: 'agent_switch' }>

  interface Props {
    block: AgentSwitchBlock
  }

  defineProps<Props>()
</script>

<template>
  <div class="agent-switch step-block">
    <div class="agent-switch-title">Agent Switch</div>
    <div class="agent-switch-body">
      <span class="agent-chip">{{ block.fromAgent }}</span>
      <span class="agent-arrow">→</span>
      <span class="agent-chip">{{ block.toAgent }}</span>
    </div>
    <div v-if="block.reason" class="agent-switch-reason">{{ block.reason }}</div>
  </div>
</template>

<style scoped>
  .agent-switch {
    border: 1px dashed var(--border-200);
    border-radius: var(--border-radius);
    background: var(--bg-200);
    padding: var(--spacing-sm) var(--spacing-md);
    color: var(--text-200);
  }

  .agent-switch-title {
    font-size: var(--font-size-xs);
    color: var(--text-500);
    margin-bottom: 6px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .agent-switch-body {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-mono);
  }

  .agent-chip {
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--bg-300);
    border: 1px solid var(--border-200);
  }

  .agent-arrow {
    color: var(--text-500);
  }

  .agent-switch-reason {
    margin-top: 6px;
    font-size: var(--font-size-sm);
    color: var(--text-400);
  }
</style>
