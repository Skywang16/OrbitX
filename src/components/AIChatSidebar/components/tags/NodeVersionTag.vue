<script setup lang="ts">
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface Props {
    version?: string | null
    visible?: boolean
  }

  interface Emits {
    (e: 'click'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    version: null,
    visible: false,
  })

  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const displayVersion = computed(() => {
    if (props.version) {
      return props.version
    }
    return '...'
  })

  const tooltipText = computed(() => {
    if (props.version) {
      return `Node.js ${props.version} · ${t('node.click_to_switch') || '点击切换版本'}`
    }
    return 'Detecting Node.js version...'
  })

  const handleClick = () => {
    emit('click')
  }
</script>

<template>
  <div v-if="visible" class="node-version-tag">
    <div class="tag-content" @click="handleClick">
      <div class="tag-icon">
        <svg width="14" height="14" viewBox="0 0 256 289" fill="none">
          <path
            d="M128 288.464c-3.975 0-7.685-1.06-11.13-2.915l-35.247-20.936c-5.3-2.915-2.65-3.975-1.06-4.505 7.155-2.385 8.48-2.915 15.9-7.156.795-.53 1.856-.265 2.65.265l27.032 16.166c1.06.53 2.385.53 3.18 0l105.74-61.217c1.06-.53 1.59-1.59 1.59-2.915V83.08c0-1.325-.53-2.385-1.59-2.915l-105.74-60.953c-1.06-.53-2.385-.53-3.18 0L20.405 80.166c-1.06.53-1.59 1.855-1.59 2.915v122.17c0 1.06.53 2.385 1.59 2.915l28.887 16.695c15.636 7.95 25.44-1.325 25.44-10.6V93.68c0-1.59 1.326-3.18 3.181-3.18h13.516c1.59 0 3.18 1.325 3.18 3.18v120.58c0 20.936-11.396 33.126-31.272 33.126-6.095 0-10.865 0-24.38-6.625l-27.827-15.9C4.24 220.885 0 213.465 0 205.515V83.346C0 75.396 4.24 67.976 11.13 64L116.87 2.783c6.625-3.71 15.635-3.71 22.26 0L244.87 64C251.76 67.975 256 75.395 256 83.346v122.17c0 7.95-4.24 15.37-11.13 19.345L139.13 286.08c-3.445 1.59-7.42 2.385-11.13 2.385zm32.596-84.008c-46.377 0-55.917-21.2-55.917-39.221 0-1.59 1.325-3.18 3.18-3.18h13.78c1.59 0 2.916 1.06 2.916 2.65 2.12 14.045 8.215 20.936 36.306 20.936 22.26 0 31.802-5.035 31.802-16.96 0-6.891-2.65-11.926-37.367-15.371-28.886-2.915-46.907-9.275-46.907-32.33 0-21.467 18.021-34.187 48.232-34.187 33.921 0 50.617 11.66 52.737 37.101 0 .795-.265 1.59-.795 2.385-.53.53-1.325 1.06-2.12 1.06h-13.78c-1.326 0-2.65-1.06-2.916-2.385-3.18-14.575-11.395-19.345-33.126-19.345-24.38 0-27.296 8.48-27.296 14.84 0 7.686 3.445 10.07 36.306 14.31 32.597 4.24 47.967 10.336 47.967 33.127-.265 23.321-19.345 36.571-53.002 36.571z"
            fill="currentColor"
          />
        </svg>
      </div>
      <span class="tag-text" :title="tooltipText">
        {{ displayVersion }}
      </span>
    </div>
  </div>
</template>

<style scoped>
  .node-version-tag {
    display: inline-block;
    vertical-align: middle;
    margin-right: 8px;
    margin-bottom: 8px;
  }

  .tag-content {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background-color: var(--bg-500);
    border: 1px solid var(--border-400);
    border-radius: var(--border-radius-sm);
    padding: 4px 8px;
    font-size: 12px;
    color: var(--text-300);
    max-width: 100%;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tag-content:hover {
    background-color: var(--bg-400);
    border-color: var(--border-300);
    color: var(--text-200);
  }

  .tag-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    color: #5fa04e;
  }

  .tag-text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    pointer-events: none;
    transition: color 0.1s ease;
  }
</style>
