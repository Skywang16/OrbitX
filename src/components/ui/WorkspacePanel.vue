<script setup lang="ts">
  import { ref, computed, watch, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { useTerminalStore } from '@/stores/Terminal'
  import { useEditorStore } from '@/stores/Editor'
  import { useLayoutStore } from '@/stores/layout'
  import { filesystemApi } from '@/api'

  const { t } = useI18n()
  const terminalStore = useTerminalStore()
  const editorStore = useEditorStore()
  const layoutStore = useLayoutStore()

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

  // 侧边栏独立的路径状态
  const sidebarPath = ref<string>('')
  const terminalCwd = computed(() => terminalStore.activeTerminal?.cwd || '~')

  // 当前显示的路径
  const currentPath = computed(() => sidebarPath.value || terminalCwd.value)

  const loading = ref(false)
  const errorMessage = ref('')

  const expandedDirs = ref<Set<string>>(new Set())
  const childrenCache = ref<Map<string, FsEntry[]>>(new Map())
  const loadingDirs = ref<Set<string>>(new Set())

  // 构建面包屑导航（不包含当前目录）
  const breadcrumbs = computed(() => {
    const path = currentPath.value
    if (!path || path === '~') return []

    const separator = getPathSeparator(path)
    const normalizedPath = path.replace(/\\/g, '/')
    const parts = normalizedPath.split('/').filter(Boolean)

    if (parts.length <= 1) return [] // 只有一个部分时（根目录或单级目录），不显示面包屑

    const crumbs: Breadcrumb[] = []
    let buildPath = ''
    const isUnixPath = normalizedPath.startsWith('/')

    // 只遍历到倒数第二个部分（排除当前目录）
    for (let i = 0; i < parts.length - 1; i++) {
      if (i === 0 && /^[A-Za-z]:$/.test(parts[0])) {
        // Windows 驱动器
        buildPath = parts[0] + separator
        crumbs.push({ name: parts[0], path: buildPath })
      } else {
        // Unix 路径以 / 开头
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

  const getRootLabel = (path: string): string => {
    if (isRootPath(path)) return path
    const separator = getPathSeparator(path)
    const parts = path.split(separator).filter(Boolean)
    if (parts.length === 0) return path
    return parts[parts.length - 1]
  }

  const buildTreeItems = (rootPath: string): TreeItem[] => {
    const items: TreeItem[] = [
      { name: getRootLabel(rootPath), path: rootPath, kind: 'dir', depth: 0, isIgnored: false },
    ]

    const visit = (dirPath: string, depth: number, parentIgnored: boolean) => {
      const children = childrenCache.value.get(dirPath) || []
      for (const entry of children) {
        const childPath = joinPath(dirPath, entry.name)
        const kind: TreeItemKind = entry.isDirectory ? 'dir' : entry.isSymlink ? 'symlink' : 'file'
        // 如果父目录被忽略，或者当前条目被忽略，则标记为 ignored
        const isIgnored = parentIgnored || entry.isIgnored || false
        items.push({
          name: entry.name,
          path: childPath,
          kind,
          depth,
          isIgnored,
        })
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

  // 处理侧边栏路径导航
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

  // 处理文件夹双击（跳转到该目录）
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

  // 当终端 cwd 改变时，如果侧边栏没有独立路径，则更新侧边栏
  watch(terminalCwd, () => {
    if (!sidebarPath.value) {
      ensureRootLoaded()
    }
  })

  onMounted(() => {
    ensureRootLoaded()
  })
</script>

<template>
  <div class="workspace-panel">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
    </div>

    <div v-else-if="errorMessage" class="error-state">
      <span>{{ errorMessage }}</span>
    </div>

    <div v-else-if="currentPath === '~'" class="empty-state">
      <span>{{ t('workspace.no_folders') }}</span>
    </div>

    <template v-else>
      <!-- 面包屑导航栏 -->
      <div v-if="breadcrumbs.length > 0" class="breadcrumb-bar">
        <div class="breadcrumbs">
          <button
            v-for="(crumb, index) in breadcrumbs"
            :key="crumb.path"
            class="breadcrumb-item"
            type="button"
            :title="crumb.path"
            @click="navigateToPath(crumb.path)"
          >
            <span class="breadcrumb-name">{{ crumb.name }}</span>
            <svg
              v-if="index < breadcrumbs.length - 1"
              class="breadcrumb-separator"
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

      <!-- 目录树 -->
      <div class="tree-list">
        <div
          v-for="item in treeItems"
          :key="item.path"
          class="tree-row"
          :class="{ dir: item.kind === 'dir', ignored: item.isIgnored }"
          :draggable="true"
          @dragstart="(e: DragEvent) => handleDragStart(e, item)"
          @dragend="handleDragEnd"
          @click="item.kind === 'dir' ? toggleDirectory(item.path) : undefined"
          @dblclick="item.kind === 'dir' ? handleDirectoryDoubleClick(item.path) : undefined"
        >
          <span class="indent-spacer" :style="{ width: `${item.depth * 14}px` }"></span>

          <button
            v-if="item.kind === 'dir'"
            class="caret-button"
            type="button"
            :title="expandedDirs.has(item.path) ? 'Collapse' : 'Expand'"
            @click.stop="toggleDirectory(item.path)"
          >
            <svg
              class="caret-icon"
              :class="{ expanded: expandedDirs.has(item.path) }"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M9 18l6-6-6-6" />
            </svg>
          </button>
          <span v-else class="caret-placeholder"></span>

          <svg
            v-if="item.kind === 'dir'"
            class="folder-icon"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
          </svg>
          <svg v-else class="file-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" />
            <path d="M14 2v6h6" />
          </svg>

          <span class="tree-name" :title="item.path">{{ item.name }}</span>

          <button
            v-if="item.kind === 'dir'"
            class="new-terminal-button"
            type="button"
            title="New terminal here"
            @click.stop="handleDirectoryNewTerminal(item.path)"
          >
            <svg class="new-terminal-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 5v14" />
              <path d="M5 12h14" />
            </svg>
          </button>
        </div>

        <div v-if="isTreeEmpty" class="empty-state">
          <span>{{ t('workspace.no_folders') }}</span>
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
    overflow-y: auto;
    padding: 8px;
  }

  /* 面包屑导航栏 */
  .breadcrumb-bar {
    padding: 6px 8px;
    margin-bottom: 6px;
    background: rgba(255, 255, 255, 0.03);
    border-radius: var(--border-radius-md);
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .breadcrumbs {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 1px;
  }

  .breadcrumb-item {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 3px 6px;
    border: none;
    background: transparent;
    color: var(--text-300);
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    cursor: pointer;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .breadcrumb-item:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-100);
  }

  .breadcrumb-name {
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .breadcrumb-separator {
    flex-shrink: 0;
    width: 12px;
    height: 12px;
    color: var(--text-500);
    opacity: 0.6;
  }

  .tree-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .tree-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-radius: var(--border-radius-sm);
    cursor: pointer;
    transition: all 0.15s ease;
    color: var(--text-200);
    min-width: 0;
  }

  .tree-row:hover {
    background: var(--color-hover);
    color: var(--text-100);
  }

  .tree-row.ignored {
    color: #666 !important;
    opacity: 0.5 !important;
  }

  .tree-row.ignored .tree-name {
    color: #666 !important;
  }

  .tree-row.ignored .folder-icon,
  .tree-row.ignored .file-icon {
    opacity: 0.5 !important;
    color: #666 !important;
  }

  .tree-row.ignored:hover {
    color: #888 !important;
    opacity: 0.7 !important;
  }

  .tree-row.ignored:hover .tree-name {
    color: #888 !important;
  }

  .tree-row.ignored:hover .folder-icon,
  .tree-row.ignored:hover .file-icon {
    opacity: 0.7 !important;
    color: #888 !important;
  }

  .folder-icon {
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    color: var(--accent-500);
  }

  .file-icon {
    flex-shrink: 0;
    width: 14px;
    height: 14px;
    color: var(--text-400);
  }

  .tree-name {
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  .indent-spacer {
    flex-shrink: 0;
  }

  .caret-button {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-300);
    cursor: pointer;
    border-radius: 4px;
  }

  .caret-button:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-100);
  }

  .caret-placeholder {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
  }

  .caret-icon {
    width: 14px;
    height: 14px;
    transition: transform 0.12s ease;
  }

  .caret-icon.expanded {
    transform: rotate(90deg);
  }

  .new-terminal-button {
    flex-shrink: 0;
    width: 20px;
    height: 20px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-400);
    cursor: pointer;
    border-radius: 4px;
    opacity: 0;
    transition:
      opacity 0.12s ease,
      background 0.12s ease,
      color 0.12s ease;
  }

  .tree-row:hover .new-terminal-button {
    opacity: 1;
  }

  .new-terminal-button:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-100);
  }

  .new-terminal-icon {
    width: 14px;
    height: 14px;
  }

  .loading-state {
    display: flex;
    justify-content: center;
    padding: 20px;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--border-300);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-state {
    padding: 12px;
    font-size: 12px;
    color: var(--color-error);
    text-align: center;
  }

  .empty-state {
    padding: 12px;
    font-size: 12px;
    color: var(--text-400);
    text-align: center;
  }

  .workspace-panel::-webkit-scrollbar {
    width: 6px;
  }

  .workspace-panel::-webkit-scrollbar-track {
    background: transparent;
  }

  .workspace-panel::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 3px;
  }

  .workspace-panel::-webkit-scrollbar-thumb:hover {
    background: var(--border-400);
  }
</style>
