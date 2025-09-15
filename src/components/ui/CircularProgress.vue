<template>
  <div class="circular-progress">
    <svg width="28" height="28" viewBox="0 0 28 28" class="progress-ring">
      <circle cx="14" cy="14" r="12" stroke="var(--border-300)" stroke-width="2" fill="none" opacity="0.3" />
      <circle
        cx="14"
        cy="14"
        r="12"
        stroke="var(--color-primary)"
        stroke-width="2"
        fill="none"
        stroke-linecap="round"
        :stroke-dasharray="75.4"
        :stroke-dashoffset="75.4 - (percentage / 100) * 75.4"
        transform="rotate(-90 14 14)"
      />
    </svg>
    <div class="progress-center">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
  interface Props {
    percentage: number
  }

  defineProps<Props>()
</script>

<style scoped>
  .circular-progress {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
  }

  .progress-ring {
    position: absolute;
    top: 0;
    left: 0;
  }

  .progress-center {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
