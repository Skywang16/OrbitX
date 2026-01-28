<script setup lang="ts">
  import { computed, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { mcpApi, settingsApi, agentApi } from '@/api'
  import type { Settings } from '@/api/settings'
  import type { McpServerStatus } from '@/api/mcp'
  import type { SkillSummary } from '@/api/agent'
  import { UNGROUPED_WORKSPACE_PATH, useWorkspaceStore } from '@/stores/workspace'

  const { t } = useI18n()
  const workspaceStore = useWorkspaceStore()

  type ConfigTab = 'global' | 'workspace'
  const activeTab = ref<ConfigTab>('global')

  const currentWorkspacePath = computed(() => workspaceStore.currentWorkspacePath)
  const hasWorkspace = computed(() => currentWorkspacePath.value !== UNGROUPED_WORKSPACE_PATH)

  const isLoadingGlobal = ref(false)
  const isSavingGlobal = ref(false)
  const globalError = ref<string | null>(null)
  const globalSettings = ref<Settings | null>(null)
  const globalRules = ref('')
  const globalMcpJson = ref('{}')
  const isReloadingGlobalMcp = ref(false)
  const globalMcpStatuses = ref<McpServerStatus[]>([])

  const isLoadingWorkspace = ref(false)
  const isSavingWorkspace = ref(false)
  const workspaceError = ref<string | null>(null)
  const workspaceSettings = ref<Settings | null>(null)
  const workspaceRules = ref('')
  const workspaceMcpJson = ref('{}')

  const isReloadingWorkspaceMcp = ref(false)
  const workspaceMcpStatuses = ref<McpServerStatus[]>([])

  // Skills 相关状态
  const isLoadingSkills = ref(false)
  const globalSkills = ref<SkillSummary[]>([])
  const workspaceSkills = ref<SkillSummary[]>([])

  const createEmptySettings = (): Settings => ({
    permissions: { allow: [], deny: [], ask: [] },
    mcpServers: {},
    rules: { content: '', rulesFiles: [] },
    agent: {},
  })

  const loadSkills = async () => {
    if (!hasWorkspace.value) return

    isLoadingSkills.value = true
    try {
      const all = await agentApi.listSkills(currentWorkspacePath.value)
      globalSkills.value = all.filter(s => s.source === 'global')
      workspaceSkills.value = all.filter(s => s.source === 'workspace')
    } catch (e) {
      console.error('Failed to load skills:', e)
    } finally {
      isLoadingSkills.value = false
    }
  }

  const loadGlobal = async () => {
    isLoadingGlobal.value = true
    globalError.value = null
    try {
      const settings = await settingsApi.getGlobal()
      globalSettings.value = settings
      globalRules.value = settings.rules?.content || ''
      globalMcpJson.value = JSON.stringify({ mcpServers: settings.mcpServers || {} }, null, 2)
      if (hasWorkspace.value) {
        globalMcpStatuses.value = await mcpApi.listServers(currentWorkspacePath.value).catch(() => [])
        loadSkills()
      }
    } catch (e) {
      globalError.value = e instanceof Error ? e.message : String(e)
    } finally {
      isLoadingGlobal.value = false
    }
  }

  const loadWorkspace = async (workspacePath: string) => {
    if (!workspacePath || workspacePath === UNGROUPED_WORKSPACE_PATH) {
      workspaceSettings.value = null
      workspaceRules.value = ''
      workspaceMcpJson.value = '{}'
      workspaceMcpStatuses.value = []
      globalSkills.value = []
      workspaceSkills.value = []
      return
    }

    isLoadingWorkspace.value = true
    workspaceError.value = null
    try {
      const settings = await settingsApi.getWorkspace(workspacePath)
      const effective = settings ?? createEmptySettings()
      workspaceSettings.value = effective
      workspaceRules.value = effective.rules?.content || ''
      workspaceMcpJson.value = JSON.stringify({ mcpServers: effective.mcpServers || {} }, null, 2)
      workspaceMcpStatuses.value = await mcpApi.listServers(workspacePath).catch(() => [])
      loadSkills()
    } catch (e) {
      workspaceError.value = e instanceof Error ? e.message : String(e)
    } finally {
      isLoadingWorkspace.value = false
    }
  }

  const saveGlobal = async () => {
    isSavingGlobal.value = true
    globalError.value = null
    try {
      const settings = globalSettings.value || (await settingsApi.getGlobal())
      settings.rules = settings.rules || { content: '', rulesFiles: [] }
      settings.rules.content = globalRules.value

      const parsed = JSON.parse(globalMcpJson.value || '{}')
      settings.mcpServers = parsed.mcpServers ?? parsed

      await settingsApi.updateGlobal(settings)
      globalSettings.value = settings
    } catch (e) {
      globalError.value = e instanceof Error ? e.message : String(e)
    } finally {
      isSavingGlobal.value = false
    }
  }

  const saveWorkspace = async () => {
    const workspacePath = currentWorkspacePath.value
    if (!workspacePath || workspacePath === UNGROUPED_WORKSPACE_PATH) return

    isSavingWorkspace.value = true
    workspaceError.value = null
    try {
      const settings = workspaceSettings.value || createEmptySettings()
      settings.rules = settings.rules || { content: '', rulesFiles: [] }
      settings.rules.content = workspaceRules.value

      const parsed = JSON.parse(workspaceMcpJson.value || '{}')
      settings.mcpServers = parsed.mcpServers ?? parsed

      await settingsApi.updateWorkspace(workspacePath, settings)
      workspaceSettings.value = settings
    } catch (e) {
      workspaceError.value = e instanceof Error ? e.message : String(e)
    } finally {
      isSavingWorkspace.value = false
    }
  }

  const reloadGlobalMcp = async () => {
    if (!hasWorkspace.value) return

    isReloadingGlobalMcp.value = true
    try {
      globalMcpStatuses.value = await mcpApi.reloadServers(currentWorkspacePath.value)
    } finally {
      isReloadingGlobalMcp.value = false
    }
  }

  const reloadWorkspaceMcp = async () => {
    const workspacePath = currentWorkspacePath.value
    if (!workspacePath || workspacePath === UNGROUPED_WORKSPACE_PATH) return

    isReloadingWorkspaceMcp.value = true
    try {
      workspaceMcpStatuses.value = await mcpApi.reloadServers(workspacePath)
    } finally {
      isReloadingWorkspaceMcp.value = false
    }
  }

  const groupedWorkspaceMcpStatuses = computed(() => {
    const global = workspaceMcpStatuses.value.filter(s => s.source === 'global')
    const workspace = workspaceMcpStatuses.value.filter(s => s.source === 'workspace')
    return { global, workspace }
  })

  const statusDotClass = (status: McpServerStatus['status']): string => {
    if (status === 'connected') return 'config-panel__dot config-panel__dot--ok'
    if (status === 'error') return 'config-panel__dot config-panel__dot--error'
    return 'config-panel__dot config-panel__dot--off'
  }

  // 展开的服务器列表
  const expandedServers = ref<Set<string>>(new Set())

  const toggleServer = (name: string) => {
    if (expandedServers.value.has(name)) {
      expandedServers.value.delete(name)
    } else {
      expandedServers.value.add(name)
    }
  }

  const isExpanded = (name: string) => expandedServers.value.has(name)

  watch(
    currentWorkspacePath,
    async newPath => {
      await loadWorkspace(newPath)
    },
    { immediate: true }
  )

  onMounted(async () => {
    await loadGlobal()
  })
</script>

<template>
  <div class="config-panel">
    <div class="config-panel__header">
      <div class="config-panel__tabs">
        <button
          class="config-panel__tab"
          :class="{ 'config-panel__tab--active': activeTab === 'global' }"
          @click="activeTab = 'global'"
        >
          {{ t('config.global') }}
        </button>
        <button
          class="config-panel__tab"
          :class="{ 'config-panel__tab--active': activeTab === 'workspace' }"
          @click="activeTab = 'workspace'"
        >
          {{ t('config.workspace') }}
        </button>
      </div>
    </div>

    <!-- Global Tab -->
    <div v-if="activeTab === 'global'" class="config-panel__content">
      <div v-if="globalError" class="config-panel__error">{{ globalError }}</div>

      <div class="config-panel__section">
        <div class="config-panel__section-header">
          <span class="config-panel__section-title">{{ t('config.rules') }}</span>
        </div>
        <textarea
          v-model="globalRules"
          class="config-panel__textarea"
          rows="4"
          :placeholder="t('config.rules')"
          :disabled="isLoadingGlobal || isSavingGlobal"
        />
      </div>

      <div class="config-panel__section">
        <div class="config-panel__section-header">
          <span class="config-panel__section-title">{{ t('config.mcp_servers') }}</span>
          <button
            v-if="hasWorkspace"
            class="config-panel__icon-btn"
            :class="{ 'config-panel__icon-btn--loading': isReloadingGlobalMcp }"
            :disabled="isLoadingGlobal || isSavingGlobal"
            :title="t('common.reload')"
            @click="reloadGlobalMcp"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M1 4v6h6" />
              <path d="M23 20v-6h-6" />
              <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15" />
            </svg>
          </button>
        </div>
        <textarea
          v-model="globalMcpJson"
          class="config-panel__textarea"
          rows="8"
          :disabled="isLoadingGlobal || isSavingGlobal"
        />

        <div v-if="globalMcpStatuses.length > 0" class="config-panel__status-list">
          <div
            v-for="s in globalMcpStatuses"
            :key="s.name"
            class="config-panel__server-block"
            @click="toggleServer(s.name)"
          >
            <div class="config-panel__status-item">
              <svg
                class="config-panel__chevron"
                :class="{ 'config-panel__chevron--expanded': isExpanded(s.name) }"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <polyline points="9 18 15 12 9 6" />
              </svg>
              <svg
                class="config-panel__server-icon"
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
              >
                <rect x="2" y="3" width="12" height="4" rx="1" />
                <rect x="2" y="9" width="12" height="4" rx="1" />
                <circle cx="4.5" cy="5" r="0.5" fill="currentColor" />
                <circle cx="4.5" cy="11" r="0.5" fill="currentColor" />
              </svg>
              <span :class="statusDotClass(s.status)"></span>
              <span class="config-panel__status-name">{{ s.name }}</span>
              <span class="config-panel__status-meta">{{ s.tools.length }} tools</span>
              <span v-if="s.error" class="config-panel__status-error" :title="s.error">!</span>
            </div>
            <div v-if="isExpanded(s.name) && s.tools.length > 0" class="config-panel__tools-drawer">
              <div v-for="tool in s.tools" :key="tool.name" class="config-panel__tool-item">
                <div class="config-panel__tool-info">
                  <span class="config-panel__tool-name">{{ tool.name }}</span>
                  <span v-if="tool.description" class="config-panel__tool-desc">{{ tool.description }}</span>
                </div>
              </div>
            </div>
            <div v-else-if="isExpanded(s.name) && s.tools.length === 0" class="config-panel__tools-empty">
              No tools available
            </div>
          </div>
        </div>
        <div v-else-if="hasWorkspace && !isLoadingGlobal" class="config-panel__hint">
          {{ t('config.no_mcp_servers') }}
        </div>
      </div>

      <!-- Skills Section -->
      <div class="config-panel__section">
        <div class="config-panel__section-header">
          <span class="config-panel__section-title">{{ t('config.global_skills') }}</span>
        </div>

        <div v-if="isLoadingSkills" class="config-panel__hint">{{ t('common.loading') }}</div>

        <div v-else-if="globalSkills.length > 0" class="config-panel__status-list">
          <div
            v-for="skill in globalSkills"
            :key="skill.name"
            class="config-panel__server-block"
            @click="toggleServer(`skill:global:${skill.name}`)"
          >
            <div class="config-panel__status-item">
              <svg
                class="config-panel__chevron"
                :class="{ 'config-panel__chevron--expanded': isExpanded(`skill:global:${skill.name}`) }"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
              >
                <polyline points="9 18 15 12 9 6" />
              </svg>
              <svg
                class="config-panel__server-icon"
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
              >
                <path d="M2 3h12v10H2z" />
                <path d="M5 1v2M11 1v2" />
                <path d="M2 6h12" />
              </svg>
              <span class="config-panel__status-name">{{ skill.name }}</span>
            </div>
            <div v-if="isExpanded(`skill:global:${skill.name}`)" class="config-panel__tools-drawer">
              <div class="config-panel__tool-item">
                <div class="config-panel__tool-info">
                  <span class="config-panel__tool-desc">{{ skill.description }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div v-else-if="hasWorkspace && !isLoadingSkills" class="config-panel__hint">
          {{ t('config.no_global_skills') }}
        </div>
      </div>

      <div class="config-panel__actions">
        <button class="config-panel__btn config-panel__btn--ghost" :disabled="isSavingGlobal" @click="loadGlobal">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M1 4v6h6" />
            <path d="M23 20v-6h-6" />
            <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15" />
          </svg>
          {{ t('common.refresh') }}
        </button>
        <button class="config-panel__btn config-panel__btn--primary" :disabled="isLoadingGlobal" @click="saveGlobal">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
            <polyline points="17,21 17,13 7,13 7,21" />
            <polyline points="7,3 7,8 15,8" />
          </svg>
          {{ t('common.save') }}
        </button>
      </div>
    </div>

    <!-- Workspace Tab -->
    <div v-else class="config-panel__content">
      <div v-if="!hasWorkspace" class="config-panel__empty">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
        <span class="config-panel__empty-text">{{ t('config.no_workspace') }}</span>
      </div>

      <template v-else>
        <div v-if="workspaceError" class="config-panel__error">{{ workspaceError }}</div>

        <div class="config-panel__section">
          <div class="config-panel__section-header">
            <span class="config-panel__section-title">{{ t('config.rules') }}</span>
          </div>
          <textarea
            v-model="workspaceRules"
            class="config-panel__textarea"
            rows="4"
            :placeholder="t('config.rules')"
            :disabled="isLoadingWorkspace || isSavingWorkspace"
          />
        </div>

        <div class="config-panel__section">
          <div class="config-panel__section-header">
            <span class="config-panel__section-title">{{ t('config.mcp_servers') }}</span>
            <button
              class="config-panel__icon-btn"
              :class="{ 'config-panel__icon-btn--loading': isReloadingWorkspaceMcp }"
              :disabled="isLoadingWorkspace || isSavingWorkspace"
              :title="t('common.reload')"
              @click="reloadWorkspaceMcp"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M1 4v6h6" />
                <path d="M23 20v-6h-6" />
                <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15" />
              </svg>
            </button>
          </div>
          <textarea
            v-model="workspaceMcpJson"
            class="config-panel__textarea"
            rows="8"
            :disabled="isLoadingWorkspace || isSavingWorkspace"
          />

          <div v-if="workspaceMcpStatuses.length > 0" class="config-panel__status-list">
            <template v-if="groupedWorkspaceMcpStatuses.global.length > 0">
              <div class="config-panel__status-group">{{ t('config.global') }}</div>
              <div
                v-for="s in groupedWorkspaceMcpStatuses.global"
                :key="`g:${s.name}`"
                class="config-panel__server-block"
                @click="toggleServer(`g:${s.name}`)"
              >
                <div class="config-panel__status-item">
                  <svg
                    class="config-panel__chevron"
                    :class="{ 'config-panel__chevron--expanded': isExpanded(`g:${s.name}`) }"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                  >
                    <polyline points="9 18 15 12 9 6" />
                  </svg>
                  <svg
                    class="config-panel__server-icon"
                    viewBox="0 0 16 16"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                  >
                    <rect x="2" y="3" width="12" height="4" rx="1" />
                    <rect x="2" y="9" width="12" height="4" rx="1" />
                    <circle cx="4.5" cy="5" r="0.5" fill="currentColor" />
                    <circle cx="4.5" cy="11" r="0.5" fill="currentColor" />
                  </svg>
                  <span :class="statusDotClass(s.status)"></span>
                  <span class="config-panel__status-name">{{ s.name }}</span>
                  <span class="config-panel__status-meta">{{ s.tools.length }} tools</span>
                  <span v-if="s.error" class="config-panel__status-error" :title="s.error">!</span>
                </div>
                <div v-if="isExpanded(`g:${s.name}`) && s.tools.length > 0" class="config-panel__tools-drawer">
                  <div v-for="tool in s.tools" :key="tool.name" class="config-panel__tool-item">
                    <div class="config-panel__tool-info">
                      <span class="config-panel__tool-name">{{ tool.name }}</span>
                      <span v-if="tool.description" class="config-panel__tool-desc">{{ tool.description }}</span>
                    </div>
                  </div>
                </div>
                <div v-else-if="isExpanded(`g:${s.name}`) && s.tools.length === 0" class="config-panel__tools-empty">
                  No tools available
                </div>
              </div>
            </template>
            <template v-if="groupedWorkspaceMcpStatuses.workspace.length > 0">
              <div class="config-panel__status-group">{{ t('config.workspace') }}</div>
              <div
                v-for="s in groupedWorkspaceMcpStatuses.workspace"
                :key="`w:${s.name}`"
                class="config-panel__server-block"
                @click="toggleServer(`w:${s.name}`)"
              >
                <div class="config-panel__status-item">
                  <svg
                    class="config-panel__chevron"
                    :class="{ 'config-panel__chevron--expanded': isExpanded(`w:${s.name}`) }"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                  >
                    <polyline points="9 18 15 12 9 6" />
                  </svg>
                  <svg
                    class="config-panel__server-icon"
                    viewBox="0 0 16 16"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                  >
                    <rect x="2" y="3" width="12" height="4" rx="1" />
                    <rect x="2" y="9" width="12" height="4" rx="1" />
                    <circle cx="4.5" cy="5" r="0.5" fill="currentColor" />
                    <circle cx="4.5" cy="11" r="0.5" fill="currentColor" />
                  </svg>
                  <span :class="statusDotClass(s.status)"></span>
                  <span class="config-panel__status-name">{{ s.name }}</span>
                  <span class="config-panel__status-meta">{{ s.tools.length }} tools</span>
                  <span v-if="s.error" class="config-panel__status-error" :title="s.error">!</span>
                </div>
                <div v-if="isExpanded(`w:${s.name}`) && s.tools.length > 0" class="config-panel__tools-drawer">
                  <div v-for="tool in s.tools" :key="tool.name" class="config-panel__tool-item">
                    <div class="config-panel__tool-info">
                      <span class="config-panel__tool-name">{{ tool.name }}</span>
                      <span v-if="tool.description" class="config-panel__tool-desc">{{ tool.description }}</span>
                    </div>
                  </div>
                </div>
                <div v-else-if="isExpanded(`w:${s.name}`) && s.tools.length === 0" class="config-panel__tools-empty">
                  No tools available
                </div>
              </div>
            </template>
          </div>
        </div>

        <!-- Workspace Skills Section -->
        <div class="config-panel__section">
          <div class="config-panel__section-header">
            <span class="config-panel__section-title">{{ t('config.workspace_skills') }}</span>
          </div>

          <div v-if="isLoadingSkills" class="config-panel__hint">{{ t('common.loading') }}</div>

          <div v-else-if="workspaceSkills.length > 0" class="config-panel__status-list">
            <div
              v-for="skill in workspaceSkills"
              :key="skill.name"
              class="config-panel__server-block"
              @click="toggleServer(`skill:workspace:${skill.name}`)"
            >
              <div class="config-panel__status-item">
                <svg
                  class="config-panel__chevron"
                  :class="{ 'config-panel__chevron--expanded': isExpanded(`skill:workspace:${skill.name}`) }"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <polyline points="9 18 15 12 9 6" />
                </svg>
                <svg
                  class="config-panel__server-icon"
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.5"
                >
                  <path d="M2 3h12v10H2z" />
                  <path d="M5 1v2M11 1v2" />
                  <path d="M2 6h12" />
                </svg>
                <span class="config-panel__status-name">{{ skill.name }}</span>
              </div>
              <div v-if="isExpanded(`skill:workspace:${skill.name}`)" class="config-panel__tools-drawer">
                <div class="config-panel__tool-item">
                  <div class="config-panel__tool-info">
                    <span class="config-panel__tool-desc">{{ skill.description }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div v-else class="config-panel__hint">
            {{ t('config.no_workspace_skills') }}
          </div>
        </div>

        <div class="config-panel__actions">
          <button
            class="config-panel__btn config-panel__btn--ghost"
            :disabled="isSavingWorkspace"
            @click="loadWorkspace(currentWorkspacePath)"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M1 4v6h6" />
              <path d="M23 20v-6h-6" />
              <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15" />
            </svg>
            {{ t('common.refresh') }}
          </button>
          <button
            class="config-panel__btn config-panel__btn--primary"
            :disabled="isLoadingWorkspace"
            @click="saveWorkspace"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
              <polyline points="17,21 17,13 7,13 7,21" />
              <polyline points="7,3 7,8 15,8" />
            </svg>
            {{ t('common.save') }}
          </button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
  .config-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-50);
    overflow: hidden;
  }

  /* Header */
  .config-panel__header {
    flex-shrink: 0;
    padding: 12px;
    border-bottom: 1px solid var(--border-200);
    background: var(--bg-100);
  }

  .config-panel__tabs {
    display: flex;
    gap: 8px;
  }

  .config-panel__tab {
    flex: 1;
    height: 28px;
    padding: 0 12px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-300);
    background: var(--bg-50);
    border: 1px solid var(--border-200);
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .config-panel__tab:hover {
    background: var(--bg-200);
    color: var(--text-100);
  }

  .config-panel__tab--active {
    background: var(--bg-200);
    color: var(--text-100);
    border-color: var(--border-300);
  }

  /* Content */
  .config-panel__content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .config-panel__content::-webkit-scrollbar {
    width: 6px;
  }

  .config-panel__content::-webkit-scrollbar-track {
    background: transparent;
  }

  .config-panel__content::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: 3px;
  }

  .config-panel__content::-webkit-scrollbar-thumb:hover {
    background: var(--border-400);
  }

  /* Section */
  .config-panel__section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .config-panel__section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .config-panel__section-title {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-200);
  }

  /* Icon Button */
  .config-panel__icon-btn {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-400);
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .config-panel__icon-btn:hover {
    background: var(--bg-200);
    color: var(--text-200);
  }

  .config-panel__icon-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .config-panel__icon-btn svg {
    width: 14px;
    height: 14px;
  }

  .config-panel__icon-btn--loading svg {
    animation: config-spin 1s linear infinite;
  }

  @keyframes config-spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Textarea */
  .config-panel__textarea {
    width: 100%;
    padding: 10px;
    font-family: var(--font-mono, monospace);
    font-size: 12px;
    line-height: 1.5;
    color: var(--text-200);
    background: var(--bg-100);
    border: 1px solid var(--border-200);
    border-radius: 6px;
    resize: vertical;
    outline: none;
    transition: border-color 0.15s ease;
  }

  .config-panel__textarea:focus {
    border-color: var(--border-300);
  }

  .config-panel__textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* Status List */
  .config-panel__status-list {
    display: flex;
    flex-direction: column;
    background: var(--bg-100);
    border: 1px solid var(--border-200);
    border-radius: 6px;
    padding: 8px;
  }

  .config-panel__status-group {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-500);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin: 8px 0 4px 0;
  }

  .config-panel__status-group:first-child {
    margin-top: 0;
  }

  .config-panel__server-block {
    display: flex;
    flex-direction: column;
    cursor: pointer;
    border-radius: 6px;
    transition: background 0.15s ease;
    margin-bottom: 4px;
  }

  .config-panel__server-block:last-child {
    margin-bottom: 0;
  }

  .config-panel__server-block:hover {
    background: var(--bg-200);
  }

  .config-panel__status-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    font-size: 12px;
  }

  .config-panel__chevron {
    width: 12px;
    height: 12px;
    flex-shrink: 0;
    color: var(--text-500);
    transition: transform 0.2s ease;
  }

  .config-panel__chevron--expanded {
    transform: rotate(90deg);
  }

  .config-panel__server-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    color: var(--text-400);
  }

  .config-panel__tools-drawer {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 4px;
    margin-left: 8px;
    border-left: 1px solid var(--border-200);
    animation: drawer-slide 0.2s ease;
  }

  @keyframes drawer-slide {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .config-panel__tool-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 4px;
    transition: background 0.15s ease;
  }

  .config-panel__tool-item:hover {
    background: var(--bg-200);
  }

  .config-panel__tool-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
    margin-top: 1px;
    color: var(--text-400);
  }

  .config-panel__tool-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .config-panel__tool-name {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: var(--text-200);
    word-break: break-all;
  }

  .config-panel__tool-desc {
    font-size: 10px;
    color: var(--text-500);
    line-height: 1.4;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
  }

  .config-panel__tools-empty {
    padding: 8px 8px 8px 30px;
    font-size: 11px;
    color: var(--text-500);
    font-style: italic;
  }

  .config-panel__status-name {
    flex: 1;
    min-width: 0;
    color: var(--text-200);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .config-panel__status-meta {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--text-500);
  }

  .config-panel__status-error {
    flex-shrink: 0;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 600;
    color: #ef4444;
    background: rgba(239, 68, 68, 0.15);
    border-radius: 50%;
    cursor: help;
  }

  .config-panel__dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
    background: var(--text-500);
  }

  .config-panel__dot--ok {
    background: #22c55e;
  }

  .config-panel__dot--error {
    background: #ef4444;
  }

  .config-panel__dot--off {
    background: var(--text-500);
  }

  /* Actions */
  .config-panel__actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: auto;
    padding-top: 8px;
  }

  .config-panel__btn {
    height: 28px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    font-size: 12px;
    font-weight: 500;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .config-panel__btn svg {
    width: 14px;
    height: 14px;
  }

  .config-panel__btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .config-panel__btn--ghost {
    color: var(--text-300);
    background: transparent;
    border: 1px solid var(--border-200);
  }

  .config-panel__btn--ghost:hover:not(:disabled) {
    background: var(--bg-200);
    color: var(--text-200);
  }

  .config-panel__btn--primary {
    color: white;
    background: var(--color-primary);
  }

  .config-panel__btn--primary:hover:not(:disabled) {
    background: var(--color-primary-hover);
  }

  /* Error */
  .config-panel__error {
    padding: 8px 12px;
    font-size: 12px;
    color: #ef4444;
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 6px;
  }

  /* Empty State */
  .config-panel__empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 24px;
    color: var(--text-400);
  }

  .config-panel__empty svg {
    width: 40px;
    height: 40px;
    opacity: 0.4;
  }

  .config-panel__empty-text {
    font-size: 13px;
  }

  .config-panel__hint {
    font-size: 11px;
    color: var(--text-500);
    text-align: center;
    padding: 8px;
  }
</style>
