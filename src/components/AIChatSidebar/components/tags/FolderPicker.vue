<script setup lang="ts">
  import { ref, onMounted, computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { filesystemApi } from '@/api'
  import { useTerminalStore } from '@/stores/Terminal'

  interface Props {
    currentPath: string
    terminalId: number
  }

  interface Emits {
    (e: 'close'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const terminalStore = useTerminalStore()

  const folders = ref<Array<{ name: string; isDirectory: boolean }>>([])
  const loading = ref(false)
  const errorMessage = ref('')

  // 判断是否为根目录
  const isRootPath = (path: string): boolean => {
    // Linux/Mac: "/"
    // Windows: "C:\", "D:\", etc
    return path === '/' || /^[A-Za-z]:[/\\]?$/.test(path)
  }

  const showParentDirectory = computed(() => !isRootPath(props.currentPath))

  // 读取目录内容
  const loadFolders = async () => {
    loading.value = true
    errorMessage.value = ''
    folders.value = []

    try {
      const entries = await filesystemApi.readDir(props.currentPath)
      // 只保留文件夹，过滤隐藏文件夹（以.开头），并按名称排序
      folders.value = entries
        .filter(entry => entry.isDirectory && !entry.name.startsWith('.'))
        .sort((a, b) => a.name.localeCompare(b.name))
    } catch (error: unknown) {
      console.error('Failed to read directory:', error)
      errorMessage.value = t('workspace.read_dir_error')
    } finally {
      loading.value = false
    }
  }

  const buildTargetPath = (folderName: string): string => {
    const separator = props.currentPath.includes('\\') ? '\\' : '/'
    const basePath = props.currentPath.endsWith(separator) ? props.currentPath : props.currentPath + separator
    return basePath + folderName
  }

  // 获取父目录路径
  const getParentPath = (): string => {
    const separator = props.currentPath.includes('\\') ? '\\' : '/'
    const parts = props.currentPath.split(separator).filter(Boolean)

    if (parts.length === 0) return '/'

    // Windows盘符处理
    if (parts.length === 1 && /^[A-Za-z]:$/.test(parts[0])) {
      return parts[0] + separator
    }

    parts.pop()

    // Unix根目录
    if (parts.length === 0) return '/'

    // Windows盘符
    if (parts.length === 1 && /^[A-Za-z]:$/.test(parts[0])) {
      return parts[0] + separator
    }

    return (props.currentPath.startsWith(separator) ? separator : '') + parts.join(separator)
  }

  // 处理文件夹点击
  const handleFolderClick = (folderName: string) => {
    const targetPath = buildTargetPath(folderName)
    terminalStore.writeToTerminal(props.terminalId, `cd "${targetPath}"\n`)
    emit('close')
  }

  // 处理返回上级目录
  const handleParentClick = () => {
    const parentPath = getParentPath()
    terminalStore.writeToTerminal(props.terminalId, `cd "${parentPath}"\n`)
    emit('close')
  }

  onMounted(() => {
    loadFolders()
  })
</script>

<template>
  <div class="folder-picker">
    <div class="body">
      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <p>{{ t('common.loading') || '加载中...' }}</p>
      </div>

      <div v-else-if="errorMessage" class="error-state">
        <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
        <p>{{ errorMessage }}</p>
      </div>

      <div v-else class="folder-list">
        <!-- 返回上级目录 -->
        <div v-if="showParentDirectory" class="folder-item parent" @click="handleParentClick">
          <svg
            class="folder-icon"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
          </svg>
          <span class="folder-name">.. ({{ t('workspace.parent_directory') }})</span>
        </div>

        <!-- 文件夹列表 -->
        <div v-for="folder in folders" :key="folder.name" class="folder-item" @click="handleFolderClick(folder.name)">
          <svg
            class="folder-icon"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
          </svg>
          <span class="folder-name">{{ folder.name }}</span>
        </div>

        <!-- 空状态 -->
        <div v-if="folders.length === 0" class="empty-state">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
          </svg>
          <p>{{ t('workspace.no_folders') }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .folder-picker {
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

  .error-state svg,
  .empty-state svg {
    color: var(--text-400);
    margin-bottom: var(--spacing-sm);
  }

  .error-state {
    color: var(--color-error);
  }

  .folder-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .folder-item {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
    color: var(--text-200);
  }

  .folder-item:hover {
    background: var(--bg-300);
    color: var(--text-100);
  }

  .folder-item.parent {
    color: var(--color-primary);
    font-weight: 500;
  }

  .folder-item.parent:hover {
    background: var(--bg-300);
  }

  .folder-icon {
    flex-shrink: 0;
    color: var(--accent-500);
  }

  .folder-item.parent .folder-icon {
    color: var(--color-primary);
  }

  .folder-name {
    font-size: var(--font-size-sm);
    font-family: var(--font-family-mono);
    word-break: break-all;
  }

  /* 滚动条样式 */
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
