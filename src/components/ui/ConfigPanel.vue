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
    <!-- Header with Tabs -->
    <div class="panel-header">
      <!-- Tab Buttons - matches Git's action buttons -->
      <div class="config-tabs">
        <button
          class="config-tab"
          :class="{ 'config-tab--active': activeTab === 'global' }"
          @click="activeTab = 'global'"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <path
              d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"
            />
          </svg>
          <span>{{ t('config.global') }}</span>
        </button>
        <button
          class="config-tab"
          :class="{ 'config-tab--active': activeTab === 'workspace' }"
          @click="activeTab = 'workspace'"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
          </svg>
          <span>{{ t('config.workspace') }}</span>
        </button>
      </div>
    </div>

    <!-- Global Tab -->
    <div v-if="activeTab === 'global'" class="panel-content">
      <!-- Error -->
      <div v-if="globalError" class="config-error">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" />
          <path d="M12 8v4M12 16h.01" />
        </svg>
        <span>{{ globalError }}</span>
      </div>

      <!-- Rules Section -->
      <div class="config-section">
        <div class="section-header">
          <div class="section-header__left">
            <svg class="section-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <path d="M14 2v6h6M16 13H8M16 17H8M10 9H8" />
            </svg>
            <span class="section-title">{{ t('config.rules') }}</span>
          </div>
        </div>
        <div class="section-body">
          <textarea
            v-model="globalRules"
            class="config-textarea"
            rows="4"
            :placeholder="t('config.rules_placeholder') || 'Enter rules...'"
            :disabled="isLoadingGlobal || isSavingGlobal"
          />
        </div>
      </div>

      <!-- MCP Servers Section -->
      <div class="config-section">
        <div class="section-header">
          <div class="section-header__left">
            <svg class="section-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="2" y="3" width="20" height="7" rx="2" />
              <rect x="2" y="14" width="20" height="7" rx="2" />
              <circle cx="6" cy="6.5" r="1" fill="currentColor" />
              <circle cx="6" cy="17.5" r="1" fill="currentColor" />
            </svg>
            <span class="section-title">{{ t('config.mcp_servers') }}</span>
            <span v-if="globalMcpStatuses.length > 0" class="section-badge">{{ globalMcpStatuses.length }}</span>
          </div>
          <button
            v-if="hasWorkspace"
            class="action-btn"
            :class="{ 'action-btn--loading': isReloadingGlobalMcp }"
            :disabled="isLoadingGlobal || isSavingGlobal"
            :title="t('common.reload')"
            @click="reloadGlobalMcp"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 12a9 9 0 0 0-9-9 9 9 0 0 0-6.5 2.8L3 8" />
              <path d="M3 3v5h5" />
              <path d="M3 12a9 9 0 0 0 9 9 9 9 0 0 0 6.5-2.8l2.5-2.2" />
              <path d="M21 21v-5h-5" />
            </svg>
          </button>
        </div>
        <div class="section-body">
          <textarea
            v-model="globalMcpJson"
            class="config-textarea config-textarea--code"
            rows="6"
            :disabled="isLoadingGlobal || isSavingGlobal"
          />

          <!-- Server List -->
          <div v-if="globalMcpStatuses.length > 0" class="server-list">
            <div v-for="s in globalMcpStatuses" :key="s.name" class="server-card" @click="toggleServer(s.name)">
              <div class="server-card__main">
                <svg
                  class="server-chevron"
                  :class="{ 'server-chevron--open': isExpanded(s.name) }"
                  viewBox="0 0 24 24"
                  fill="currentColor"
                >
                  <path d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z" />
                </svg>
                <div
                  class="server-dot"
                  :class="s.status === 'connected' ? 'server-dot--ok' : s.status === 'error' ? 'server-dot--error' : ''"
                ></div>
                <span class="server-name">{{ s.name }}</span>
                <span class="server-badge">{{ s.tools.length }} tools</span>
                <span v-if="s.error" class="server-error-icon" :title="s.error">!</span>
              </div>
              <div v-if="isExpanded(s.name)" class="server-card__tools">
                <div v-if="s.tools.length > 0" class="tools-list">
                  <div v-for="tool in s.tools" :key="tool.name" class="tool-row">
                    <span class="tool-name">{{ tool.name }}</span>
                    <span v-if="tool.description" class="tool-desc">{{ tool.description }}</span>
                  </div>
                </div>
                <div v-else class="tools-empty">No tools available</div>
              </div>
            </div>
          </div>
          <div v-else-if="hasWorkspace && !isLoadingGlobal" class="section-hint">
            {{ t('config.no_mcp_servers') }}
          </div>
        </div>
      </div>

      <!-- Skills Section -->
      <div class="config-section">
        <div class="section-header">
          <div class="section-header__left">
            <svg class="section-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
            </svg>
            <span class="section-title">{{ t('config.global_skills') }}</span>
            <span v-if="globalSkills.length > 0" class="section-badge">{{ globalSkills.length }}</span>
          </div>
        </div>
        <div class="section-body">
          <div v-if="isLoadingSkills" class="section-loading">
            <div class="loading-spinner"></div>
          </div>
          <div v-else-if="globalSkills.length > 0" class="server-list">
            <div
              v-for="skill in globalSkills"
              :key="skill.name"
              class="server-card"
              @click="toggleServer(`skill:global:${skill.name}`)"
            >
              <div class="server-card__main">
                <svg
                  class="server-chevron"
                  :class="{ 'server-chevron--open': isExpanded(`skill:global:${skill.name}`) }"
                  viewBox="0 0 24 24"
                  fill="currentColor"
                >
                  <path d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z" />
                </svg>
                <span class="server-name">{{ skill.name }}</span>
              </div>
              <div v-if="isExpanded(`skill:global:${skill.name}`)" class="server-card__tools">
                <div class="tool-row">
                  <span class="tool-desc">{{ skill.description }}</span>
                </div>
              </div>
            </div>
          </div>
          <div v-else-if="hasWorkspace" class="section-hint">
            {{ t('config.no_global_skills') }}
          </div>
        </div>
      </div>

      <!-- Actions -->
      <div class="config-actions">
        <button class="config-btn" :disabled="isSavingGlobal" @click="loadGlobal">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 12a9 9 0 0 0-9-9 9 9 0 0 0-6.5 2.8L3 8" />
            <path d="M3 3v5h5" />
            <path d="M3 12a9 9 0 0 0 9 9 9 9 0 0 0 6.5-2.8l2.5-2.2" />
            <path d="M21 21v-5h-5" />
          </svg>
          {{ t('common.refresh') }}
        </button>
        <button class="config-btn config-btn--primary" :disabled="isLoadingGlobal" @click="saveGlobal">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <polyline points="20 6 9 17 4 12" />
          </svg>
          {{ t('common.save') }}
        </button>
      </div>
    </div>

    <!-- Workspace Tab -->
    <div v-else class="panel-content">
      <div v-if="!hasWorkspace" class="panel-empty">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
        </svg>
        <span>{{ t('config.no_workspace') }}</span>
      </div>

      <template v-else>
        <!-- Error -->
        <div v-if="workspaceError" class="config-error">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10" />
            <path d="M12 8v4M12 16h.01" />
          </svg>
          <span>{{ workspaceError }}</span>
        </div>

        <!-- Rules Section -->
        <div class="config-section">
          <div class="section-header">
            <div class="section-header__left">
              <svg class="section-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <path d="M14 2v6h6M16 13H8M16 17H8M10 9H8" />
              </svg>
              <span class="section-title">{{ t('config.rules') }}</span>
            </div>
          </div>
          <div class="section-body">
            <textarea
              v-model="workspaceRules"
              class="config-textarea"
              rows="4"
              :placeholder="t('config.rules_placeholder') || 'Enter rules...'"
              :disabled="isLoadingWorkspace || isSavingWorkspace"
            />
          </div>
        </div>

        <!-- MCP Servers Section -->
        <div class="config-section">
          <div class="section-header">
            <div class="section-header__left">
              <svg class="section-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="2" y="3" width="20" height="7" rx="2" />
                <rect x="2" y="14" width="20" height="7" rx="2" />
                <circle cx="6" cy="6.5" r="1" fill="currentColor" />
                <circle cx="6" cy="17.5" r="1" fill="currentColor" />
              </svg>
              <span class="section-title">{{ t('config.mcp_servers') }}</span>
              <span v-if="workspaceMcpStatuses.length > 0" class="section-badge">
                {{ workspaceMcpStatuses.length }}
              </span>
            </div>
            <button
              class="action-btn"
              :class="{ 'action-btn--loading': isReloadingWorkspaceMcp }"
              :disabled="isLoadingWorkspace || isSavingWorkspace"
              :title="t('common.reload')"
              @click="reloadWorkspaceMcp"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 12a9 9 0 0 0-9-9 9 9 0 0 0-6.5 2.8L3 8" />
                <path d="M3 3v5h5" />
                <path d="M3 12a9 9 0 0 0 9 9 9 9 0 0 0 6.5-2.8l2.5-2.2" />
                <path d="M21 21v-5h-5" />
              </svg>
            </button>
          </div>
          <div class="section-body">
            <textarea
              v-model="workspaceMcpJson"
              class="config-textarea config-textarea--code"
              rows="6"
              :disabled="isLoadingWorkspace || isSavingWorkspace"
            />

            <!-- Server List -->
            <div v-if="workspaceMcpStatuses.length > 0" class="server-list">
              <template v-if="groupedWorkspaceMcpStatuses.global.length > 0">
                <div class="server-group-label">{{ t('config.global') }}</div>
                <div
                  v-for="s in groupedWorkspaceMcpStatuses.global"
                  :key="`g:${s.name}`"
                  class="server-card"
                  @click="toggleServer(`g:${s.name}`)"
                >
                  <div class="server-card__main">
                    <svg
                      class="server-chevron"
                      :class="{ 'server-chevron--open': isExpanded(`g:${s.name}`) }"
                      viewBox="0 0 24 24"
                      fill="currentColor"
                    >
                      <path d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z" />
                    </svg>
                    <div
                      class="server-dot"
                      :class="
                        s.status === 'connected' ? 'server-dot--ok' : s.status === 'error' ? 'server-dot--error' : ''
                      "
                    ></div>
                    <span class="server-name">{{ s.name }}</span>
                    <span class="server-badge">{{ s.tools.length }} tools</span>
                  </div>
                  <div v-if="isExpanded(`g:${s.name}`)" class="server-card__tools">
                    <div v-if="s.tools.length > 0" class="tools-list">
                      <div v-for="tool in s.tools" :key="tool.name" class="tool-row">
                        <span class="tool-name">{{ tool.name }}</span>
                        <span v-if="tool.description" class="tool-desc">{{ tool.description }}</span>
                      </div>
                    </div>
                    <div v-else class="tools-empty">No tools available</div>
                  </div>
                </div>
              </template>
              <template v-if="groupedWorkspaceMcpStatuses.workspace.length > 0">
                <div class="server-group-label">{{ t('config.workspace') }}</div>
                <div
                  v-for="s in groupedWorkspaceMcpStatuses.workspace"
                  :key="`w:${s.name}`"
                  class="server-card"
                  @click="toggleServer(`w:${s.name}`)"
                >
                  <div class="server-card__main">
                    <svg
                      class="server-chevron"
                      :class="{ 'server-chevron--open': isExpanded(`w:${s.name}`) }"
                      viewBox="0 0 24 24"
                      fill="currentColor"
                    >
                      <path d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z" />
                    </svg>
                    <div
                      class="server-dot"
                      :class="
                        s.status === 'connected' ? 'server-dot--ok' : s.status === 'error' ? 'server-dot--error' : ''
                      "
                    ></div>
                    <span class="server-name">{{ s.name }}</span>
                    <span class="server-badge">{{ s.tools.length }} tools</span>
                  </div>
                  <div v-if="isExpanded(`w:${s.name}`)" class="server-card__tools">
                    <div v-if="s.tools.length > 0" class="tools-list">
                      <div v-for="tool in s.tools" :key="tool.name" class="tool-row">
                        <span class="tool-name">{{ tool.name }}</span>
                        <span v-if="tool.description" class="tool-desc">{{ tool.description }}</span>
                      </div>
                    </div>
                    <div v-else class="tools-empty">No tools available</div>
                  </div>
                </div>
              </template>
            </div>
          </div>
        </div>

        <!-- Workspace Skills Section -->
        <div class="config-section">
          <div class="section-header">
            <div class="section-header__left">
              <svg class="section-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path
                  d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"
                />
              </svg>
              <span class="section-title">{{ t('config.workspace_skills') }}</span>
              <span v-if="workspaceSkills.length > 0" class="section-badge">{{ workspaceSkills.length }}</span>
            </div>
          </div>
          <div class="section-body">
            <div v-if="isLoadingSkills" class="section-loading">
              <div class="loading-spinner"></div>
            </div>
            <div v-else-if="workspaceSkills.length > 0" class="server-list">
              <div
                v-for="skill in workspaceSkills"
                :key="skill.name"
                class="server-card"
                @click="toggleServer(`skill:workspace:${skill.name}`)"
              >
                <div class="server-card__main">
                  <svg
                    class="server-chevron"
                    :class="{ 'server-chevron--open': isExpanded(`skill:workspace:${skill.name}`) }"
                    viewBox="0 0 24 24"
                    fill="currentColor"
                  >
                    <path d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z" />
                  </svg>
                  <span class="server-name">{{ skill.name }}</span>
                </div>
                <div v-if="isExpanded(`skill:workspace:${skill.name}`)" class="server-card__tools">
                  <div class="tool-row">
                    <span class="tool-desc">{{ skill.description }}</span>
                  </div>
                </div>
              </div>
            </div>
            <div v-else class="section-hint">
              {{ t('config.no_workspace_skills') }}
            </div>
          </div>
        </div>

        <!-- Actions -->
        <div class="config-actions">
          <button class="config-btn" :disabled="isSavingWorkspace" @click="loadWorkspace(currentWorkspacePath)">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 12a9 9 0 0 0-9-9 9 9 0 0 0-6.5 2.8L3 8" />
              <path d="M3 3v5h5" />
              <path d="M3 12a9 9 0 0 0 9 9 9 9 0 0 0 6.5-2.8l2.5-2.2" />
              <path d="M21 21v-5h-5" />
            </svg>
            {{ t('common.refresh') }}
          </button>
          <button class="config-btn config-btn--primary" :disabled="isLoadingWorkspace" @click="saveWorkspace">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <polyline points="20 6 9 17 4 12" />
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
    background: var(--bg-200);
    overflow: hidden;
  }

  /* Header */
  .panel-header {
    flex-shrink: 0;
    padding: 16px;
    background: var(--bg-50);
    border-bottom: 1px solid var(--border-100);
  }

  /* Config Tabs */
  .config-tabs {
    display: flex;
    gap: 8px;
  }

  .config-tab {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 10px 12px;
    background: var(--bg-400);
    border: 1px solid transparent;
    border-radius: var(--border-radius-md);
    color: var(--text-400);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .config-tab svg {
    width: 15px;
    height: 15px;
  }

  .config-tab:hover {
    background: var(--bg-500);
    color: var(--text-300);
  }

  .config-tab--active {
    background: var(--color-primary-alpha);
    color: var(--text-100);
  }

  /* Content */
  .panel-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 8px;
  }

  /* Sections */
  .config-section {
    margin-bottom: 12px;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 10px 12px;
    background: var(--bg-100);
    border: 1px solid var(--border-100);
    border-bottom: none;
    border-radius: 10px 10px 0 0;
  }

  .section-header__left {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .section-icon {
    width: 16px;
    height: 16px;
    color: var(--text-400);
    flex-shrink: 0;
  }

  .section-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-200);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }

  .section-badge {
    font-size: 11px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 10px;
    background: var(--bg-200);
    color: var(--text-400);
  }

  .section-body {
    background: var(--bg-50);
    border: 1px solid var(--border-100);
    border-top: none;
    border-radius: 0 0 10px 10px;
    padding: 12px;
  }

  .section-hint {
    font-size: 12px;
    color: var(--text-500);
    text-align: center;
    padding: 16px;
  }

  .section-loading {
    display: flex;
    justify-content: center;
    padding: 16px;
  }

  /* Action Button */
  .action-btn {
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 6px;
    background: var(--bg-200);
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover {
    background: var(--bg-300);
    color: var(--text-100);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-btn svg {
    width: 14px;
    height: 14px;
  }

  .action-btn--loading svg {
    animation: spin 1s linear infinite;
  }

  /* Textarea */
  .config-textarea {
    width: 100%;
    padding: 12px 14px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-200);
    background: var(--bg-100);
    border: 1px solid var(--border-100);
    border-radius: 8px;
    resize: vertical;
    outline: none;
    transition: all 0.15s ease;
  }

  .config-textarea--code {
    font-family: var(--font-family-mono);
    font-size: 12px;
  }

  .config-textarea:focus {
    border-color: var(--color-primary);
  }

  .config-textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  /* Server List */
  .server-list {
    margin-top: 12px;
  }

  .server-group-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-500);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 8px 10px 4px;
  }

  .server-card {
    border-radius: var(--border-radius-lg);
    margin-bottom: 2px;
    cursor: pointer;
    transition: background 0.12s ease;
  }

  .server-card:hover {
    background: var(--color-hover);
  }

  .server-card__main {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
  }

  .server-chevron {
    width: 16px;
    height: 16px;
    color: var(--text-500);
    flex-shrink: 0;
    transition: transform 0.15s ease;
  }

  .server-chevron--open {
    transform: rotate(90deg);
  }

  .server-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-500);
    flex-shrink: 0;
  }

  .server-dot--ok {
    background: var(--color-success);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-success) 20%, transparent);
  }

  .server-dot--error {
    background: var(--color-error);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-error) 20%, transparent);
  }

  .server-name {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-100);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .server-badge {
    font-size: 11px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: 10px;
    background: var(--bg-200);
    color: var(--text-400);
    flex-shrink: 0;
  }

  .server-error-icon {
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-error);
    background: color-mix(in srgb, var(--color-error) 15%, transparent);
    border-radius: 50%;
    cursor: help;
    flex-shrink: 0;
  }

  .server-card__tools {
    padding: 0 12px 12px 38px;
  }

  .tools-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .tool-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 10px;
    border-radius: var(--border-radius);
    transition: background 0.12s ease;
  }

  .tool-row:hover {
    background: var(--color-hover);
  }

  .tool-name {
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-family-mono);
    color: var(--text-200);
  }

  .tool-desc {
    font-size: 11px;
    color: var(--text-500);
    line-height: 1.4;
  }

  .tools-empty {
    font-size: 12px;
    color: var(--text-500);
    padding: 8px;
  }

  /* Actions */
  .config-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 8px 8px;
    margin-top: auto;
  }

  .config-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    font-size: 12px;
    font-weight: 500;
    border: 1px solid var(--border-100);
    border-radius: 10px;
    cursor: pointer;
    transition: all 0.15s ease;
    background: var(--bg-100);
    color: var(--text-300);
  }

  .config-btn svg {
    width: 15px;
    height: 15px;
  }

  .config-btn:hover:not(:disabled) {
    background: var(--bg-200);
    border-color: var(--border-200);
    color: var(--text-100);
  }

  .config-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .config-btn--primary {
    background: var(--color-primary);
    border-color: var(--color-primary);
    color: white;
  }

  .config-btn--primary:hover:not(:disabled) {
    background: var(--color-primary-hover);
    border-color: var(--color-primary-hover);
    transform: translateY(-1px);
  }

  /* Error */
  .config-error {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    margin: 8px;
    font-size: 12px;
    color: var(--color-error);
    background: color-mix(in srgb, var(--color-error) 8%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-error) 20%, transparent);
    border-radius: var(--border-radius-lg);
  }

  .config-error svg {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  /* Empty */
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
    width: 18px;
    height: 18px;
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
