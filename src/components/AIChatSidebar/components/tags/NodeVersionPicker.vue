<script setup lang="ts">
  import { ref, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { nodeApi, type NodeVersionInfo } from '@/api'

  interface Props {
    currentVersion: string
    manager: string
    cwd?: string
  }

  interface Emits {
    (e: 'select', version: string): void
    (e: 'close'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const versions = ref<NodeVersionInfo[]>([])
  const loading = ref(false)

  // 版本号规范化，移除 v 前缀
  const normalizeVersion = (version: string) => version.trim().replace(/^v/, '')

  // 加载已安装的版本列表
  const loadVersions = async () => {
    loading.value = true
    const versionList = await nodeApi.listVersions()
    // 重新标记当前版本，以防包括前缀差异
    versions.value = versionList.map(v => ({
      ...v,
      is_current: normalizeVersion(v.version) === normalizeVersion(props.currentVersion),
    }))
    loading.value = false
  }

  const handleVersionClick = (version: string) => {
    emit('select', version)
    emit('close')
  }

  onMounted(() => {
    loadVersions()
  })
</script>

<template>
  <div class="node-version-picker">
    <div class="body">
      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <p>{{ t('common.loading') }}</p>
      </div>

      <div v-else class="version-list">
        <div
          v-for="ver in versions"
          :key="ver.version"
          class="version-item"
          :class="{ current: ver.is_current }"
          @click="handleVersionClick(ver.version)"
        >
          <svg
            v-if="ver.is_current"
            class="check-icon"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="3"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
          <span class="version-name">{{ ver.version }}</span>
        </div>

        <div v-if="versions.length === 0" class="empty-state">
          <svg width="32" height="32" viewBox="0 0 256 289" fill="none">
            <path
              d="M128 288.464c-3.975 0-7.685-1.06-11.13-2.915l-35.247-20.936c-5.3-2.915-2.65-3.975-1.06-4.505 7.155-2.385 8.48-2.915 15.9-7.156.795-.53 1.856-.265 2.65.265l27.032 16.166c1.06.53 2.385.53 3.18 0l105.74-61.217c1.06-.53 1.59-1.59 1.59-2.915V83.08c0-1.325-.53-2.385-1.59-2.915l-105.74-60.953c-1.06-.53-2.385-.53-3.18 0L20.405 80.166c-1.06.53-1.59 1.855-1.59 2.915v122.17c0 1.06.53 2.385 1.59 2.915l28.887 16.695c15.636 7.95 25.44-1.325 25.44-10.6V93.68c0-1.59 1.326-3.18 3.181-3.18h13.516c1.59 0 3.18 1.325 3.18 3.18v120.58c0 20.936-11.396 33.126-31.272 33.126-6.095 0-10.865 0-24.38-6.625l-27.827-15.9C4.24 220.885 0 213.465 0 205.515V83.346C0 75.396 4.24 67.976 11.13 64L116.87 2.783c6.625-3.71 15.635-3.71 22.26 0L244.87 64C251.76 67.975 256 75.395 256 83.346v122.17c0 7.95-4.24 15.37-11.13 19.345L139.13 286.08c-3.445 1.59-7.42 2.385-11.13 2.385z"
              fill="currentColor"
              stroke="currentColor"
            />
          </svg>
          <p>{{ t('node.no_versions') }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .node-version-picker {
    overflow: hidden;
  }

  .body {
    padding: var(--spacing-lg);
    max-height: 220px;
    overflow-y: auto;
  }

  .loading-state,
  .error-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-xl) var(--spacing-lg);
    color: var(--text-300);
    text-align: center;
  }

  .loading-state .spinner {
    width: 24px;
    height: 24px;
    border: 2px solid var(--border-300);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-bottom: var(--spacing-sm);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .empty-state svg {
    color: #5fa04e;
    opacity: 0.5;
    margin-bottom: var(--spacing-sm);
  }

  .version-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .version-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
    color: var(--text-200);
    min-height: 32px;
  }

  .version-item:hover {
    background: var(--bg-300);
    color: var(--text-100);
  }

  .version-item.current {
    color: var(--color-primary);
    font-weight: 500;
  }

  .version-item.current:hover {
    background: var(--bg-300);
  }

  .check-icon {
    flex-shrink: 0;
    color: var(--color-primary);
  }

  .version-name {
    font-size: var(--font-size-sm);
    font-family: var(--font-family-mono);
    word-break: break-all;
  }

  .body::-webkit-scrollbar {
    width: 6px;
  }

  .body::-webkit-scrollbar-track {
    background: transparent;
  }

  .body::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 3px;
  }

  .body::-webkit-scrollbar-thumb:hover {
    background: var(--border-400);
  }
</style>
