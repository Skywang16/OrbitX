<template>
  <div class="empty-state">
    <div class="empty-text">No active tasks</div>
    <div class="empty-subtitle">Tasks will appear here when AI starts working</div>
  </div>
</template>

<style scoped>
  .empty-state {
    padding: 20px 16px;
    text-align: center;
    color: var(--text-400);
  }

  .empty-icon {
    font-size: 24px;
    margin-bottom: 8px;
    opacity: 0.5;
  }

  .empty-text {
    font-size: 12px;
    font-weight: 500;
    margin-bottom: 4px;
    opacity: 0.8;
  }

  .empty-subtitle {
    font-size: 10px;
    opacity: 0.6;
    line-height: 1.4;
  }
</style>
