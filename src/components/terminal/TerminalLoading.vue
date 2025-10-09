<template>
  <div class="terminal-loading-overlay">
    <div class="spinner"></div>
  </div>
</template>

<style scoped>
  .terminal-loading-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--bg-100);
    z-index: 1000;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 3px solid var(--bg-300);
    border-top-color: var(--accent-500);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
