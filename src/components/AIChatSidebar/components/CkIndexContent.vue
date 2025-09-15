<script setup lang="ts">
  import { computed, ref, watch, onMounted } from 'vue'
  import terminalContextApi from '@/api/terminal-context'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { useTerminalStore } from '@/stores/Terminal'
  import { TabType } from '@/types'
  import { homeDir } from '@tauri-apps/api/path'
  import { useI18n } from 'vue-i18n'
  const { t } = useI18n()

  interface Props {
    indexStatus: {
      hasIndex: boolean
      path: string
      size?: string
    }
  }

  interface Emits {
    (e: 'build'): void
    (e: 'delete'): void
    (e: 'refresh'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  const handleBuild = () => {
    emit('build')
  }

  const handleDelete = () => {
    emit('delete')
  }

  const handleRefresh = () => {
    emit('refresh')
  }

  const statusText = computed(() => {
    if (props.indexStatus.hasIndex) {
      return t('ck.index_ready')
    }
    return t('ck.index_not_ready')
  })

  const tabManagerStore = useTabManagerStore()
  const terminalStore = useTerminalStore()

  const displayPath = ref(props.indexStatus.path)
  const resolvedPath = ref<string>(props.indexStatus.path || '.')
  const homePath = ref<string>('')
  const indexSize = ref<string>('')

  const simplify = (p: string) => {
    const parts = p.replace(/\/$/, '').split(/[/\\]/).filter(Boolean)
    if (parts.length === 0) return '~'
    const last = parts[parts.length - 1]
    return last.length > 15 ? last.slice(0, 12) + '...' : last
  }

  const normalize = (p: string) => p.replace(/\\/g, '/').replace(/\/$/, '')

  const refreshDisplayPath = async () => {
    let p = props.indexStatus.path
    if (!p || p === '.') {
      const activeTab = tabManagerStore.activeTab
      if (activeTab && activeTab.type === TabType.TERMINAL) {
        if (activeTab.path && activeTab.path !== '~') {
          p = activeTab.path
        } else {
          const terminal = terminalStore.terminals.find(t => t.id === activeTab.id)
          if (terminal?.cwd) {
            p = terminal.cwd
          }
        }
      }
      if (!p || p === '.') {
        try {
          const ctx = await terminalContextApi.getActiveTerminalContext()
          const cwd = ctx?.current_working_directory || ctx?.currentWorkingDirectory
          if (cwd) p = cwd
        } catch (e) {
          console.error('Failed to get active terminal context', e)
        }
      }
    }
    resolvedPath.value = p || '.'
    displayPath.value = p && p !== '.' ? simplify(p) : '.'
  }

  const canBuild = computed(() => {
    const pRaw = resolvedPath.value
    if (!pRaw) return false
    const p = normalize(pRaw)
    if (p === '.' || p === '~' || p === '/' || /^[A-Za-z]:$/.test(p)) return false
    if (homePath.value) {
      const h = normalize(homePath.value)
      if (p === h) return false
    }
    return true
  })

  watch(
    () => props.indexStatus,
    newStatus => {
      refreshDisplayPath()
      // 直接使用 indexStatus 中的 size 字段
      if (newStatus.hasIndex && newStatus.path) {
        indexSize.value = newStatus.size || ''
      } else {
        indexSize.value = ''
      }
    },
    { deep: true, immediate: true }
  )

  onMounted(() => {
    refreshDisplayPath()
    homeDir()
      .then(path => (homePath.value = path))
      .catch(() => {})
  })
</script>

<template>
  <div class="ck-index-content">
    <div class="header">
      <div class="title-section">
        <h3 class="title">{{ t('ck.title') }}</h3>
        <p class="subtitle">{{ t('ck.subtitle') }}</p>
      </div>
    </div>

    <div class="body">
      <div v-if="!indexStatus.hasIndex" class="workspace-section">
        <div class="workspace-info">
          <div class="workspace-label">{{ t('ck.current_workspace') }}</div>
          <div class="workspace-path">{{ displayPath }}</div>
        </div>
        <x-button
          variant="primary"
          :disabled="!canBuild"
          :title="!canBuild ? t('ck.build_index_tooltip_disabled') : t('ck.build_index_tooltip_enabled')"
          @click="handleBuild"
        >
          {{ t('ck.build_index_button') }}
        </x-button>
      </div>

      <div v-if="indexStatus.hasIndex" class="indexed-section">
        <div class="status-row">
          <div class="status-info">
            <span class="status-text">{{ statusText }}</span>
            <div class="index-size-info" v-if="indexSize">
              <span class="size-value">{{ indexSize }}</span>
            </div>
          </div>
          <div class="action-buttons">
            <x-button size="small" variant="primary" @click="handleRefresh">
              <template #icon>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                  <path d="M3 3v5h5" />
                  <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
                  <path d="M21 21v-5h-5" />
                </svg>
              </template>
              {{ t('ck.refresh') }}
            </x-button>
            <x-button size="small" variant="danger" @click="handleDelete">
              <template #icon>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="3,6 5,6 21,6"></polyline>
                  <path d="m19,6v14a2,2 0 0,1 -2,2H7a2,2 0 0,1 -2,-2V6m3,0V4a2,2 0 0,1 2,-2h4a2,2 0 0,1 2,2v2"></path>
                  <line x1="10" x2="10" y1="11" y2="17"></line>
                  <line x1="14" x2="14" y1="11" y2="17"></line>
                </svg>
              </template>
              {{ t('ck.delete') }}
            </x-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .ck-index-content {
    overflow: hidden;
  }

  .header {
    padding: var(--spacing-lg) var(--spacing-lg) var(--spacing-md) var(--spacing-lg);
    border-bottom: 1px solid var(--border-200);
  }

  .title-section {
    text-align: left;
  }

  .title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 var(--spacing-xs) 0;
  }

  .subtitle {
    font-size: var(--font-size-sm);
    color: var(--text-300);
    margin: 0;
    line-height: 1.4;
  }

  .body {
    padding: var(--spacing-lg);
  }

  .workspace-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .workspace-info {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .workspace-label {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-200);
  }

  .workspace-path {
    font-size: var(--font-size-sm);
    color: var(--text-100);
    font-family: var(--font-family-mono);
    background: var(--bg-300);
    padding: var(--spacing-sm);
    border-radius: var(--border-radius-sm);
    border: 1px solid var(--border-200);
    word-break: break-all;
  }

  .build-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--color-primary);
    color: white;
    border: none;
    border-radius: var(--border-radius-sm);
    font-size: var(--font-size-sm);
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }

  .build-button:hover {
    background: var(--color-primary-dark);
  }

  .indexed-section {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-md);
  }

  .status-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--spacing-md);
  }

  .status-info {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-xs);
  }

  .status-text {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-100);
  }

  .index-size-info {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    font-size: var(--font-size-xs);
  }

  .size-value {
    color: var(--text-200);
    font-family: var(--font-family-mono);
    background: var(--bg-300);
    padding: 2px var(--spacing-xs);
    border-radius: var(--border-radius-xs);
    border: 1px solid var(--border-200);
  }

  .action-buttons {
    display: flex;
    gap: var(--spacing-sm);
  }

  /* 增加系统 x-button 内部图标与文字的间距，仅作用于本组件按钮区 */
  .action-buttons :deep(.x-button) {
    gap: 6px;
  }

  .action-button {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-xs) var(--spacing-sm);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-sm);
    font-size: var(--font-size-xs);
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s ease;
  }

  .action-button.secondary {
    background: var(--bg-200);
    color: var(--text-200);
  }

  .action-button.secondary:hover {
    background: var(--bg-300);
    border-color: var(--border-300);
    color: var(--text-100);
  }

  .action-button.danger {
    background: var(--bg-200);
    color: var(--color-danger);
  }

  .action-button.danger:hover {
    background: var(--color-danger);
    color: white;
    border-color: var(--color-danger);
  }
</style>
