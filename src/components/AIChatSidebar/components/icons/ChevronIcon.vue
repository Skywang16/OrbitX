<template>
  <svg class="chevron-icon" :class="{ expanded }" width="14" height="14" viewBox="0 0 24 24">
    <path
      d="M9 18l6-6-6-6"
      stroke="currentColor"
      stroke-width="2"
      fill="none"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</template>

<script setup lang="ts">
  interface Props {
    expanded?: boolean
  }

  withDefaults(defineProps<Props>(), {
    expanded: false,
  })
</script>

<style scoped>
  .chevron-icon {
    transition: transform 0.2s ease;
    opacity: 0.7;
  }

  .chevron-icon.expanded {
    transform: rotate(90deg);
  }
</style>
