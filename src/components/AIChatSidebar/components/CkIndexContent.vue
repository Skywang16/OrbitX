<script setup lang="ts">
  import { computed, ref, watch, onMounted } from 'vue'
  import terminalContextApi from '@/api/terminal-context'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { useTerminalStore } from '@/stores/Terminal'
  import { TabType } from '@/types'
  import { homeDir } from '@tauri-apps/api/path'
  import { useI18n } from '@/i18n'

  const { t } = useI18n()

  interface Props {
    indexStatus: {
      hasIndex: boolean
      path: string
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
    () => {
      refreshDisplayPath()
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
        <h3 class="title">代码库索引</h3>
        <p class="subtitle">配置代码库索引设置以启用项目的语义搜索。</p>
      </div>
    </div>

    <div class="body">
      <div v-if="!indexStatus.hasIndex" class="workspace-section">
        <div class="workspace-info">
          <div class="workspace-label">当前工作区</div>
          <div class="workspace-path">{{ displayPath }}</div>
        </div>
        <x-button
          variant="primary"
          :disabled="!canBuild"
          :title="!canBuild ? '请选择非初始目录后再构建' : '构建索引'"
          @click="handleBuild"
        >
          构建索引
        </x-button>
      </div>

      <div v-if="indexStatus.hasIndex" class="indexed-section">
        <div class="status-row">
          <div class="status-info">
            <span class="status-text">{{ statusText }}</span>
          </div>
          <div class="action-buttons">
            <x-button variant="secondary" @click="handleRefresh">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
                <path d="M3 3v5h5" />
                <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
                <path d="M21 21v-5h-5" />
              </svg>
              刷新
            </x-button>
            <x-button variant="danger" @click="handleDelete">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="m3 6 3 12c0 .6.4 1 1 1h8c.6 0 1-.4 1-1l3-12" />
                <path d="M8 6V4c0-.6.4-1 1-1h4c.6 0 1 .4 1 1v2" />
                <line x1="10" y1="11" x2="10" y2="17" />
                <line x1="14" y1="11" x2="14" y2="17" />
              </svg>
              删除
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
    align-items: center;
    gap: var(--spacing-sm);
  }

  .status-text {
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--text-100);
  }

  .action-buttons {
    display: flex;
    gap: var(--spacing-sm);
  }

  .action-button {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
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
