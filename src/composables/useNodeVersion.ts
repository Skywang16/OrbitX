import { ref, onBeforeUnmount } from 'vue'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { nodeApi, shellIntegrationApi } from '@/api'

interface NodeVersionState {
  isNodeProject: boolean
  currentVersion: string | null
  manager: string | null
}

export function useNodeVersion() {
  const state = ref<NodeVersionState>({
    isNodeProject: false,
    currentVersion: null,
    manager: null,
  })

  let unlisten: UnlistenFn | null = null

  const detect = async (cwd: string, terminalId: number) => {
    const isNodeProject = await nodeApi.checkNodeProject(cwd)

    if (!isNodeProject) {
      state.value = { isNodeProject: false, currentVersion: null, manager: null }
      return
    }

    const [manager, paneState] = await Promise.all([
      nodeApi.getVersionManager(),
      shellIntegrationApi.getPaneShellState(terminalId),
    ])

    state.value = {
      isNodeProject: true,
      currentVersion: paneState?.node_version || null,
      manager,
    }
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
