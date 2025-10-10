import { ref, watch, onBeforeUnmount } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { nodeApi, shellIntegrationApi } from '@/api'

interface NodeVersionState {
  isNodeProject: boolean
  currentVersion: string | null
  manager: string | null
}

export function useNodeVersion(terminalId: () => number | undefined, cwd: () => string | undefined) {
  const state = ref<NodeVersionState>({
    isNodeProject: false,
    currentVersion: null,
    manager: null,
  })

  let unlisten: UnlistenFn | null = null

  // 设置 Node 版本变化监听器
  const setupListener = async () => {
    unlisten = await nodeApi.onVersionChanged(payload => {
      const currentPaneId = terminalId()
      if (payload.paneId === currentPaneId && state.value.isNodeProject) {
        state.value.currentVersion = payload.version || null
      }
    })
  }

  // 检测当前目录是否为 Node 项目
  const detectNodeProject = async (cwdPath: string) => {
    if (!cwdPath || cwdPath === '~') {
      state.value = { isNodeProject: false, currentVersion: null, manager: null }
      return
    }

    const isNodeProject = await nodeApi.checkNodeProject(cwdPath)

    if (isNodeProject) {
      const manager = await nodeApi.getVersionManager()
      state.value = {
        isNodeProject: true,
        currentVersion: null,
        manager,
      }
      syncVersion()
    } else {
      state.value = { isNodeProject: false, currentVersion: null, manager: null }
    }
  }

  // 从 pane state 同步当前 Node 版本
  const syncVersion = async () => {
    const paneId = terminalId()
    if (!paneId) return

    const paneState = await shellIntegrationApi.getPaneShellState(paneId)
    if (paneState?.node_version) {
      state.value.currentVersion = paneState.node_version
    }
  }

  // 监听工作目录变化
  watch(
    cwd,
    newCwd => {
      if (newCwd) {
        detectNodeProject(newCwd)
      }
    },
    { immediate: true }
  )

  // 监听终端切换
  watch(terminalId, (newId, oldId) => {
    if (newId !== oldId && newId) {
      const currentCwd = cwd()
      if (currentCwd) {
        detectNodeProject(currentCwd)
      }
    }
  })

  // 初始化监听器
  setupListener()

  // 清理资源
  onBeforeUnmount(() => {
    if (unlisten) {
      unlisten()
    }
  })

  return {
    state,
  }
}
