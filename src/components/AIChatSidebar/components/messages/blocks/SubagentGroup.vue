<script setup lang="ts">
  import type { SubagentRecord } from '@/types'
  import SubagentCard from './SubagentCard.vue'

  interface Props {
    subagents: SubagentRecord[]
  }

  defineProps<Props>()
</script>

<template>
  <section class="subagent-group step-block">
    <div class="subagent-grid">
      <SubagentCard v-for="subagent in subagents" :key="subagent.id" :subagent="subagent" />
    </div>
  </section>
</template>

<style scoped>
  .subagent-group {
    margin-bottom: var(--spacing-md);
  }

  .subagent-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    align-items: start;
    gap: 12px;
  }

  @media (max-width: 720px) {
    .subagent-grid {
      grid-template-columns: 1fr;
      gap: 10px;
    }
  }
</style>
