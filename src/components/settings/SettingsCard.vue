<script setup lang="ts">
  interface Props {
    title?: string
    description?: string
    collapsible?: boolean
    defaultCollapsed?: boolean
  }

  import { ref, computed } from 'vue'

  const props = withDefaults(defineProps<Props>(), {
    collapsible: false,
    defaultCollapsed: false,
  })

  const isCollapsed = ref(props.defaultCollapsed)

  const toggleCollapse = () => {
    if (props.collapsible) {
      isCollapsed.value = !isCollapsed.value
    }
  }

  const hasHeader = computed(() => props.title || props.description)
</script>

<template>
  <div class="settings-card" :class="{ 'is-collapsible': collapsible, 'is-collapsed': isCollapsed }">
    <div v-if="hasHeader" class="settings-card-header" :class="{ clickable: collapsible }" @click="toggleCollapse">
      <div class="settings-card-header-content">
        <h4 v-if="title" class="settings-card-title">{{ title }}</h4>
        <p v-if="description" class="settings-card-description">{{ description }}</p>
      </div>
      <div v-if="collapsible" class="settings-card-toggle">
        <svg
          class="settings-card-toggle-icon"
          :class="{ expanded: !isCollapsed }"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </div>
    </div>
    <div v-show="!isCollapsed || !collapsible" class="settings-card-body">
      <slot />
    </div>
  </div>
</template>

<style scoped>
  .settings-card {
    background: var(--bg-200);
    border: 1px solid var(--border-100);
    border-radius: 12px;
    overflow: hidden;
    transition:
      border-color 0.2s ease,
      box-shadow 0.2s ease;
  }

  .settings-card:hover {
    border-color: var(--border-200);
  }

  .settings-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border-100);
    background: var(--bg-250, color-mix(in srgb, var(--bg-300) 30%, var(--bg-200)));
  }

  .settings-card-header.clickable {
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .settings-card-header.clickable:hover {
    background: var(--bg-300);
  }

  .settings-card.is-collapsed .settings-card-header {
    border-bottom: none;
  }

  .settings-card-header-content {
    flex: 1;
    min-width: 0;
  }

  .settings-card-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0;
    line-height: 1.4;
  }

  .settings-card-description {
    font-size: 12px;
    color: var(--text-400);
    margin: 4px 0 0 0;
    line-height: 1.4;
  }

  .settings-card-toggle {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    margin-left: 12px;
  }

  .settings-card-toggle-icon {
    width: 16px;
    height: 16px;
    color: var(--text-400);
    transition: transform 0.2s ease;
  }

  .settings-card-toggle-icon.expanded {
    transform: rotate(180deg);
  }

  .settings-card-body {
    /* Body styles are inherited from global settings.css */
  }

  /* Remove inner item margins for cleaner look */
  .settings-card-body :deep(.settings-item) {
    background: transparent;
    border-radius: 0;
    margin-bottom: 0;
    position: relative;
  }

  .settings-card-body :deep(.settings-item:not(:last-child))::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 20px;
    right: 20px;
    height: 1px;
    background: var(--border-100);
  }
</style>
