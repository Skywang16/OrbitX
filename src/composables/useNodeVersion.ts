import { ref, onBeforeUnmount } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { nodeApi } from '@/api'

interface NodeVersionState {
  isNodeProject: boolean
  currentVersion: string | null
  manager: string | null
}

export const useNodeVersion = () => {
  const state = ref<NodeVersionState>({
    isNodeProject: false,
    currentVersion: null,
    manager: null,
  })

  let unlisten: UnlistenFn | null = null

  // Detect whether the workspace is a Node project and which version manager is in use.
  // Terminal state is not involved — current version is updated separately via events.
  const detect = async (workspacePath: string) => {
    const isNodeProject = await nodeApi.checkNodeProject(workspacePath)
    if (!isNodeProject) {
      state.value = { isNodeProject: false, currentVersion: null, manager: null }
      return
    }
    const [manager, currentVersion] = await Promise.all([nodeApi.getVersionManager(), nodeApi.getCurrentVersion()])
    state.value = { isNodeProject: true, currentVersion, manager }
  }

  const setupListener = async (getCurrentTerminalId: () => number) => {
    unlisten = await nodeApi.onVersionChanged(payload => {
      const currentId = getCurrentTerminalId()
      if (payload.paneId === currentId && state.value.isNodeProject) {
        state.value.currentVersion = payload.version || null
      }
    })
  }

  onBeforeUnmount(() => {
    if (unlisten) {
      unlisten()
    }
  })

  return {
    state,
    detect,
    setupListener,
  }
}
