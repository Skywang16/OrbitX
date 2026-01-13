<script setup lang="ts">
  import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { debounce } from 'lodash-es'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useEditorStore } from '@/stores/Editor'
  import { useLayoutStore } from '@/stores/layout'
  import { useFileWatcherStore } from '@/stores/fileWatcher'
  import { filesystemApi } from '@/api'

  const { t } = useI18n()
  const terminalStore = useTerminalStore()
  const editorStore = useEditorStore()
  const layoutStore = useLayoutStore()
  const fileWatcherStore = useFileWatcherStore()

  type FsEntry = {
    name: string
    isDirectory: boolean
    isFile: boolean
    isSymlink: boolean
    isIgnored: boolean
  }
  type TreeItemKind = 'dir' | 'file' | 'symlink'
  type TreeItem = { name: string; path: string; kind: TreeItemKind; depth: number; isIgnored: boolean }
  type Breadcrumb = { name: string; path: string }

  const sidebarPath = ref<string>('')
  const terminalCwd = computed(() => terminalStore.activeTerminal?.cwd || '~')
  const currentPath = computed(() => sidebarPath.value || terminalCwd.value)

  const loading = ref(false)
  const errorMessage = ref('')

  const expandedDirs = ref<Set<string>>(new Set())
  const childrenCache = ref<Map<string, FsEntry[]>>(new Map())
  const loadingDirs = ref<Set<string>>(new Set())

  const breadcrumbs = computed(() => {
    const path = currentPath.value
    if (!path || path === '~') return []

    const separator = getPathSeparator(path)
    const normalizedPath = path.replace(/\\/g, '/')
    const parts = normalizedPath.split('/').filter(Boolean)

    if (parts.length <= 1) return []

    const crumbs: Breadcrumb[] = []
    let buildPath = ''
    const isUnixPath = normalizedPath.startsWith('/')

    for (let i = 0; i < parts.length - 1; i++) {
      if (i === 0 && /^[A-Za-z]:$/.test(parts[0])) {
        buildPath = parts[0] + separator
        crumbs.push({ name: parts[0], path: buildPath })
      } else {
        if (i === 0 && isUnixPath) {
          buildPath = '/' + parts[i]
        } else {
          buildPath += separator + parts[i]
        }
        crumbs.push({ name: parts[i], path: buildPath })
      }
    }

    return crumbs
  })

  const isRootPath = (path: string): boolean => {
    return path === '/' || /^[A-Za-z]:[/\\]?$/.test(path)
  }

  const getPathSeparator = (path: string): string => {
    return path.includes('\\') ? '\\' : '/'
  }

  const joinPath = (parent: string, name: string): string => {
    const separator = getPathSeparator(parent)
    const basePath = parent.endsWith(separator) ? parent : parent + separator
    return basePath + name
  }

  const getParentPath = (path: string): string | null => {
    const separator = getPathSeparator(path)
    if (isRootPath(path)) return null
    const normalized = path.endsWith(separator) ? path.slice(0, -1) : path
    const idx = normalized.lastIndexOf(separator)
    if (idx <= 0) return normalized.startsWith(separator) ? separator : null
    return normalized.slice(0, idx)
  }

  const sortEntries = (entries: FsEntry[]): FsEntry[] => {
    return [...entries].sort((a, b) => {
      if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1
      return a.name.localeCompare(b.name)
    })
  }

  const loadChildren = async (path: string) => {
    if (!path || path === '~') return
    if (childrenCache.value.has(path)) return
    if (loadingDirs.value.has(path)) return

    loadingDirs.value.add(path)
    try {
      const entries = await filesystemApi.readDir(path)
      childrenCache.value.set(path, sortEntries(entries as FsEntry[]))
      childrenCache.value = new Map(childrenCache.value)
    } catch (error: unknown) {
      console.error('Failed to read directory:', error)
      childrenCache.value.set(path, [])
      childrenCache.value = new Map(childrenCache.value)
      errorMessage.value = t('workspace.read_dir_error')
    } finally {
      loadingDirs.value.delete(path)
    }
  }

  const reloadChildren = async (path: string) => {
    if (!path || path === '~') return
    if (loadingDirs.value.has(path)) return
    childrenCache.value.delete(path)
    childrenCache.value = new Map(childrenCache.value)
    await loadChildren(path)
  }

  const resetTreeState = () => {
    expandedDirs.value = new Set()
    childrenCache.value = new Map()
    loadingDirs.value = new Set()
  }

  const ensureRootLoaded = async () => {
    const path = currentPath.value
    if (!path || path === '~') {
      resetTreeState()
      return
    }

    loading.value = true
    errorMessage.value = ''
    resetTreeState()

    try {
      await loadChildren(path)
      expandedDirs.value.add(path)
      expandedDirs.value = new Set(expandedDirs.value)
    } catch (error: unknown) {
      errorMessage.value = t('workspace.read_dir_error')
    } finally {
      loading.value = false
    }
  }

  const buildTreeItems = (rootPath: string): TreeItem[] => {
    const rootLabel = isRootPath(rootPath)
      ? rootPath
      : rootPath.split(getPathSeparator(rootPath)).filter(Boolean).pop() || rootPath
    const items: TreeItem[] = [{ name: rootLabel, path: rootPath, kind: 'dir', depth: 0, isIgnored: false }]

    const visit = (dirPath: string, depth: number, parentIgnored: boolean) => {
      const children = childrenCache.value.get(dirPath) || []
      for (const entry of children) {
        const childPath = joinPath(dirPath, entry.name)
        const kind: TreeItemKind = entry.isDirectory ? 'dir' : entry.isSymlink ? 'symlink' : 'file'
        const isIgnored = parentIgnored || entry.isIgnored || false
        items.push({ name: entry.name, path: childPath, kind, depth, isIgnored })
        if (entry.isDirectory && expandedDirs.value.has(childPath)) {
          visit(childPath, depth + 1, isIgnored)
        }
      }
    }

    if (expandedDirs.value.has(rootPath)) {
      visit(rootPath, 1, false)
    }

    return items
  }

  const treeItems = computed(() => {
    const path = currentPath.value
    if (!path || path === '~') return []
    return buildTreeItems(path)
  })

  const toggleDirectory = async (path: string) => {
    if (expandedDirs.value.has(path)) {
      expandedDirs.value.delete(path)
      expandedDirs.value = new Set(expandedDirs.value)
      return
    }

    expandedDirs.value.add(path)
    expandedDirs.value = new Set(expandedDirs.value)
    await loadChildren(path)
  }

  const navigateToPath = async (path: string) => {
    sidebarPath.value = path
    resetTreeState()
    loading.value = true
    errorMessage.value = ''

    try {
      await loadChildren(path)
      expandedDirs.value.add(path)
      expandedDirs.value = new Set(expandedDirs.value)
    } catch (error: unknown) {
      errorMessage.value = t('workspace.read_dir_error')
    } finally {
      loading.value = false
    }
  }

  const handleDirectoryNewTerminal = async (path: string) => {
    await editorStore.createTerminalTab({ directory: path, activate: true })
  }

  const handleDirectoryDoubleClick = async (path: string) => {
    await navigateToPath(path)
  }

  const isTreeEmpty = computed(() => {
    const path = currentPath.value
    if (!path || path === '~') return true
    if (!expandedDirs.value.has(path)) return false
    const children = childrenCache.value.get(path)
    return Array.isArray(children) && children.length === 0
  })

  const handleDragStart = (event: DragEvent, item: TreeItem) => {
    void event
    layoutStore.setDragPath(item.path)
  }

  const handleDragEnd = () => {
    setTimeout(() => layoutStore.setDragPath(null), 100)
  }

  let unsubscribeWatcher: (() => void) | null = null
  const pendingReloadDirs = new Set<string>()
  const flushReloadDirs = debounce(async () => {
    const dirs = Array.from(pendingReloadDirs)
    pendingReloadDirs.clear()

    await Promise.allSettled(
      dirs.map(async dir => {
        if (!expandedDirs.value.has(dir)) {
          childrenCache.value.delete(dir)
          childrenCache.value = new Map(childrenCache.value)
          return
        }
        await reloadChildren(dir)
      })
    )
  }, 200)

  watch(terminalCwd, () => {
    if (!sidebarPath.value) {
      ensureRootLoaded()
    }
  })

  onMounted(() => {
    ensureRootLoaded()

    unsubscribeWatcher = fileWatcherStore.subscribe(batch => {
      for (const evt of batch.events) {
        if (evt.type !== 'fs_changed') continue
        const paths = [evt.path]
        if (evt.oldPath) paths.push(evt.oldPath)

        for (const p of paths) {
          const parent = getParentPath(p)
          if (!parent) continue
          if (!childrenCache.value.has(parent) && !expandedDirs.value.has(parent)) continue
          pendingReloadDirs.add(parent)
        }
      }

      flushReloadDirs()
    })
  })

  onUnmounted(() => {
    unsubscribeWatcher?.()
    flushReloadDirs.cancel()
  })
</script>

<template>
  <div class="workspace-panel">
    <div v-if="loading" class="workspace-panel__loading">
      <div class="workspace-panel__spinner"></div>
    </div>

    <div v-else-if="errorMessage" class="workspace-panel__error">
      <span>{{ errorMessage }}</span>
    </div>

    <div v-else-if="currentPath === '~'" class="workspace-panel__empty">
      <svg class="workspace-panel__empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
      </svg>
      <span class="workspace-panel__empty-text">{{ t('workspace.no_folders') }}</span>
    </div>

    <template v-else>
      <div class="workspace-panel__header">
        <div class="workspace-panel__path">
          <button
            v-for="(crumb, index) in breadcrumbs"
            :key="crumb.path"
            class="workspace-panel__path-item"
            type="button"
            :title="crumb.path"
            @click="navigateToPath(crumb.path)"
          >
            <span>{{ crumb.name }}</span>
            <svg
              v-if="index < breadcrumbs.length - 1"
              class="workspace-panel__path-sep"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="M9 18l6-6-6-6" />
            </svg>
          </button>
        </div>
      </div>

      <div class="workspace-panel__tree">
        <div
          v-for="item in treeItems"
          :key="item.path"
          class="workspace-panel__item"
          :class="{ 'workspace-panel__item--ignored': item.isIgnored }"
          :draggable="true"
          @dragstart="(e: DragEvent) => handleDragStart(e, item)"
          @dragend="handleDragEnd"
          @click="item.kind === 'dir' ? toggleDirectory(item.path) : undefined"
          @dblclick="item.kind === 'dir' ? handleDirectoryDoubleClick(item.path) : undefined"
        >
          <span class="workspace-panel__item-indent" :style="{ width: `${item.depth * 14}px` }"></span>

          <button
            v-if="item.kind === 'dir'"
            class="workspace-panel__item-caret"
            type="button"
            @click.stop="toggleDirectory(item.path)"
          >
            <svg
              :class="{ 'workspace-panel__item-caret--open': expandedDirs.has(item.path) }"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="M9 18l6-6-6-6" />
            </svg>
          </button>
          <span v-else class="workspace-panel__item-caret-placeholder"></span>

          <svg
            v-if="item.kind === 'dir'"
            class="workspace-panel__item-icon workspace-panel__item-icon--folder"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
          </svg>
          <svg
            v-else
            class="workspace-panel__item-icon workspace-panel__item-icon--file"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
            <path d="M14 2v6h6" />
          </svg>

          <span class="workspace-panel__item-name" :title="item.path">{{ item.name }}</span>

          <button
            v-if="item.kind === 'dir'"
            class="workspace-panel__item-action"
            type="button"
            title="New terminal here"
            @click.stop="handleDirectoryNewTerminal(item.path)"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 5v14" />
              <path d="M5 12h14" />
            </svg>
          </button>
        </div>

        <div v-if="isTreeEmpty" class="workspace-panel__empty workspace-panel__empty--inline">
          <span class="workspace-panel__empty-text">{{ t('workspace.no_folders') }}</span>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
  .workspace-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-50);
    overflow: hidden;
  }

  /* Header */
  .workspace-panel__header {
    flex-shrink: 0;
    padding: 12px;
    border-bottom: 1px solid var(--border-200);
    background: var(--bg-100);
  }

  .workspace-panel__path {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 2px;
  }

  .workspace-panel__path-item {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px 4px;
    border: none;
    background: transparent;
    color: var(--text-400);
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .workspace-panel__path-item:hover {
    background: var(--bg-200);
    color: var(--text-200);
  }

  .workspace-panel__path-sep {
    width: 12px;
    height: 12px;
    color: var(--text-500);
  }

  /* Tree */
  .workspace-panel__tree {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 8px 0;
  }

  .workspace-panel__tree::-webkit-scrollbar {
    width: 6px;
  }

  .workspace-panel__tree::-webkit-scrollbar-track {
    background: transparent;
  }

  .workspace-panel__tree::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 3px;
  }

  .workspace-panel__tree::-webkit-scrollbar-thumb:hover {
    background: var(--border-400);
  }

  /* Tree Item */
  .workspace-panel__item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px;
    cursor: pointer;
    transition: background 0.15s ease;
    color: var(--text-100);
    min-width: 0;
  }

  .workspace-panel__item:hover {
    background: var(--color-hover);
  }

  .workspace-panel__item--ignored {
    opacity: 0.5;
  }

  .workspace-panel__item-indent {
    flex-shrink: 0;
  }

  .workspace-panel__item-caret {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-400);
    cursor: pointer;
    border-radius: 4px;
  }

  .workspace-panel__item-caret:hover {
    background: var(--bg-200);
    color: var(--text-200);
  }

  .workspace-panel__item-caret svg {
    width: 14px;
    height: 14px;
    transition: transform 0.15s ease;
  }

  .workspace-panel__item-caret--open {
    transform: rotate(90deg);
  }

  .workspace-panel__item-caret-placeholder {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
  }

  .workspace-panel__item-icon {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
  }

  .workspace-panel__item-icon--folder {
    color: var(--accent-500);
  }

  .workspace-panel__item-icon--file {
    color: var(--text-300);
  }

  .workspace-panel__item-name {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .workspace-panel__item-action {
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-400);
    cursor: pointer;
    border-radius: 4px;
    opacity: 0;
    transition: all 0.15s ease;
  }

  .workspace-panel__item:hover .workspace-panel__item-action {
    opacity: 1;
  }

  .workspace-panel__item-action:hover {
    background: var(--bg-200);
    color: var(--text-200);
  }

  .workspace-panel__item-action svg {
    width: 14px;
    height: 14px;
  }

  /* States */
  .workspace-panel__loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }

  .workspace-panel__spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-300);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: workspace-spin 0.8s linear infinite;
  }

  @keyframes workspace-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .workspace-panel__error {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 24px;
    font-size: 12px;
    color: #ef4444;
  }

  .workspace-panel__empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 24px;
    color: var(--text-400);
  }

  .workspace-panel__empty--inline {
    padding: 16px;
  }

  .workspace-panel__empty-icon {
    width: 48px;
    height: 48px;
    opacity: 0.4;
  }

  .workspace-panel__empty-text {
    font-size: 13px;
  }
</style>
