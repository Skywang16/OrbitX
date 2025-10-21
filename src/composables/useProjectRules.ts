import { ref } from 'vue'
import { workspaceApi } from '@/api'

interface ProjectRulesState {
  hasRulesFile: boolean
  selectedRulesFile: string | null
}

export const useProjectRules = () => {
  const state = ref<ProjectRulesState>({
    hasRulesFile: false,
    selectedRulesFile: null,
  })

  const detect = async (cwd: string) => {
    const [files, selectedRule] = await Promise.all([
      workspaceApi.listAvailableRulesFiles(cwd),
      workspaceApi.getProjectRules(),
    ])

    const hasFiles = files.length > 0

    state.value = {
      hasRulesFile: hasFiles,
      selectedRulesFile: selectedRule || (hasFiles ? files[0] : null),
    }
  }

  const refresh = async () => {
    const rules = await workspaceApi.getProjectRules()
    state.value.selectedRulesFile = rules || null
  }

  return {
    state,
    detect,
    refresh,
  }
}
