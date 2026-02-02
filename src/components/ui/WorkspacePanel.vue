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
  const currentFolderName = computed(() => {
    const path = currentPath.value
    if (!path || path === '~') return ''
    return path.split(getPathSeparator(path)).filter(Boolean).pop() || path
  })

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

  const rootChildren = computed(() => {
    const path = currentPath.value
    if (!path || path === '~') return []
    return childrenCache.value.get(path) || []
  })

  const getChildren = (path: string) => {
    return childrenCache.value.get(path) || []
  }

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

  const handleDragStart = (event: DragEvent, path: string) => {
    void event
    layoutStore.setDragPath(path)
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
    <!-- Header -->
    <div class="panel-header">
      <!-- Current Folder Button -->
      <button class="folder-btn" type="button" :title="currentPath">
        <svg class="folder-icon" viewBox="0 0 24 24" fill="currentColor">
          <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
        <span class="folder-name">{{ currentFolderName || 'Explorer' }}</span>
      </button>

      <!-- Breadcrumb -->
      <div v-if="breadcrumbs.length > 0" class="breadcrumb">
        <template v-for="(crumb, index) in breadcrumbs" :key="crumb.path">
          <button class="breadcrumb-item" type="button" :title="crumb.path" @click="navigateToPath(crumb.path)">
            {{ crumb.name }}
          </button>
          <span v-if="index < breadcrumbs.length - 1" class="breadcrumb-sep">›</span>
        </template>
      </div>
    </div>

    <!-- Loading State -->
    <div v-if="loading" class="panel-loading">
      <div class="loading-spinner"></div>
    </div>

    <!-- Error State -->
    <div v-else-if="errorMessage" class="panel-error">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10" />
        <path d="M12 8v4M12 16h.01" />
      </svg>
      <span>{{ errorMessage }}</span>
    </div>

    <!-- Empty State -->
    <div v-else-if="currentPath === '~'" class="panel-empty">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
      </svg>
      <span>{{ t('workspace.no_folders') }}</span>
    </div>

    <!-- File Tree -->
    <div v-else class="file-tree">
      <!-- Empty folder -->
      <div v-if="rootChildren.length === 0" class="tree-empty">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
        <span>{{ t('workspace.empty_folder') }}</span>
      </div>

      <!-- File list -->
      <template v-else>
        <template v-for="entry in rootChildren" :key="joinPath(currentPath, entry.name)">
          <!-- Directory -->
          <div
            v-if="entry.isDirectory"
            class="tree-item tree-item--dir"
            :class="{ 'tree-item--dim': entry.isIgnored }"
            :draggable="true"
            @dragstart="e => handleDragStart(e, joinPath(currentPath, entry.name))"
            @dragend="handleDragEnd"
            @click="toggleDirectory(joinPath(currentPath, entry.name))"
            @dblclick="handleDirectoryDoubleClick(joinPath(currentPath, entry.name))"
          >
            <svg
              class="tree-chevron"
              :class="{ 'tree-chevron--open': expandedDirs.has(joinPath(currentPath, entry.name)) }"
              viewBox="0 0 24 24"
              fill="currentColor"
            >
              <path d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z" />
            </svg>
            <div class="tree-icon tree-icon--folder">
              <svg viewBox="0 0 24 24" fill="currentColor">
                <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
              </svg>
            </div>
            <span class="tree-name">{{ entry.name }}</span>
            <button
              class="tree-action"
              type="button"
              :title="t('workspace.new_terminal_here')"
              @click.stop="handleDirectoryNewTerminal(joinPath(currentPath, entry.name))"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M4 17l6-6-6-6M12 19h8" />
              </svg>
            </button>
          </div>

          <!-- Nested children -->
          <template v-if="entry.isDirectory && expandedDirs.has(joinPath(currentPath, entry.name))">
            <div
              v-for="child in getChildren(joinPath(currentPath, entry.name))"
              :key="joinPath(joinPath(currentPath, entry.name), child.name)"
              class="tree-item"
              :class="{
                'tree-item--dir': child.isDirectory,
                'tree-item--nested': true,
                'tree-item--dim': child.isIgnored || entry.isIgnored,
              }"
              :draggable="true"
              @dragstart="e => handleDragStart(e, joinPath(joinPath(currentPath, entry.name), child.name))"
              @dragend="handleDragEnd"
              @click="
                child.isDirectory ? toggleDirectory(joinPath(joinPath(currentPath, entry.name), child.name)) : undefined
              "
            >
              <svg v-if="child.isDirectory" class="tree-chevron" viewBox="0 0 24 24" fill="currentColor">
                <path d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z" />
              </svg>
              <span v-else class="tree-chevron-space"></span>
              <div class="tree-icon" :class="child.isDirectory ? 'tree-icon--folder' : 'tree-icon--file'">
                <svg v-if="child.isDirectory" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                </svg>
                <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
                  <path d="M14 2v6h6" />
                </svg>
              </div>
              <span class="tree-name">{{ child.name }}</span>
            </div>
          </template>

          <!-- File -->
          <div
            v-if="entry.isFile || entry.isSymlink"
            class="tree-item"
            :class="{ 'tree-item--dim': entry.isIgnored }"
            :draggable="true"
            @dragstart="e => handleDragStart(e, joinPath(currentPath, entry.name))"
            @dragend="handleDragEnd"
          >
            <span class="tree-chevron-space"></span>
            <div class="tree-icon tree-icon--file">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
                <path d="M14 2v6h6" />
              </svg>
            </div>
            <span class="tree-name">{{ entry.name }}</span>
          </div>
        </template>
      </template>
    </div>
  </div>
</template>

<style scoped>
  .workspace-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-200);
    overflow: hidden;
  }

  /* Header */
  .panel-header {
    flex-shrink: 0;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--bg-50);
    border-bottom: 1px solid var(--border-100);
  }

  /* Folder Button - matches Git's branch-btn */
  .folder-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    background: var(--bg-100);
    border: 1px solid var(--border-100);
    border-radius: 10px;
    color: var(--text-100);
    font-size: 13px;
    font-weight: 500;
    cursor: default;
    transition: all 0.15s ease;
  }

  .folder-icon {
    width: 16px;
    height: 16px;
    color: var(--color-warning);
    flex-shrink: 0;
  }

  .folder-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Breadcrumb */
  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 2px;
    overflow-x: auto;
  }

  .breadcrumb::-webkit-scrollbar {
    height: 0;
  }

  .breadcrumb-item {
    flex-shrink: 0;
    padding: 4px 8px;
    border: none;
    background: transparent;
    color: var(--text-400);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    border-radius: 6px;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .breadcrumb-item:hover {
    background: var(--bg-100);
    color: var(--text-200);
  }

  .breadcrumb-sep {
    color: var(--text-500);
    font-size: 11px;
  }

  /* File Tree */
  .file-tree {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 8px;
  }

  .tree-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--border-radius-lg);
    cursor: pointer;
    transition: background 0.12s ease;
  }

  .tree-item:hover {
    background: var(--color-hover);
  }

  .tree-item:hover .tree-action {
    opacity: 1;
  }

  .tree-item--nested {
    padding-left: 32px;
  }

  .tree-item--dim {
    opacity: 0.5;
  }

  .tree-chevron {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    color: var(--text-500);
    transition: transform 0.15s ease;
  }

  .tree-chevron--open {
    transform: rotate(90deg);
  }

  .tree-chevron-space {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  .tree-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .tree-icon svg {
    width: 16px;
    height: 16px;
  }

  .tree-icon--folder {
    color: var(--color-warning);
  }

  .tree-icon--file {
    color: var(--text-400);
  }

  .tree-name {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-200);
    font-family: var(--font-family-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tree-action {
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    padding: 0;
    border: none;
    background: var(--bg-200);
    color: var(--text-400);
    cursor: pointer;
    border-radius: 6px;
    opacity: 0;
    transition: all 0.15s ease;
  }

  .tree-action:hover {
    background: color-mix(in srgb, var(--color-primary) 20%, transparent);
    color: var(--color-primary);
  }

  .tree-action svg {
    width: 14px;
    height: 14px;
  }

  .tree-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 40px 20px;
    color: var(--text-500);
    font-size: 13px;
  }

  .tree-empty svg {
    width: 40px;
    height: 40px;
    opacity: 0.4;
  }

  /* States */
  .panel-loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 40px;
  }

  .panel-error {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 8px;
    padding: 12px 16px;
    font-size: 12px;
    color: var(--color-error);
    background: color-mix(in srgb, var(--color-error) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-error) 20%, transparent);
    border-radius: var(--border-radius-lg);
  }

  .panel-error svg {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  .panel-empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 40px 20px;
    color: var(--text-500);
    font-size: 13px;
  }

  .panel-empty svg {
    width: 40px;
    height: 40px;
    opacity: 0.4;
  }

  .loading-spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-200);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
