<script setup lang="ts">
  import { mcpApi, settingsApi } from '@/api'
  import type { McpToolInfo } from '@/api/mcp/types'
  import type { McpServerConfig } from '@/api/settings/types'
  import type { McpServerStatus } from '@/api/mcp/types'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { XButton, XFormGroup, XInput, XModal, XSwitch, XTextarea, createMessage } from '@/ui'
  import { debounce } from 'lodash-es'
  import { computed, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface SmitheryServer {
    id: string
    qualifiedName: string
    namespace: string
    slug: string
    displayName: string
    description: string
    iconUrl: string
    verified: boolean
    useCount: number
    remote: boolean
    isDeployed: boolean
    homepage: string
  }

  interface SmitheryResponse {
    servers: SmitheryServer[]
    pagination: {
      currentPage: number
      pageSize: number
      totalPages: number
      totalCount: number
    }
  }

  const { t } = useI18n()
  const workspaceStore = useWorkspaceStore()
  const SMITHERY_API = 'https://registry.smithery.ai/servers'

  const isLoading = ref(false)
  const currentWorkspace = computed(() => workspaceStore.currentWorkspacePath)

  const reloadMcpRegistry = async () => {
    if (currentWorkspace.value) {
      await mcpApi.reloadServers(currentWorkspace.value)
    }
  }
  const mcpServers = ref<Record<string, McpServerConfig>>({})
  const serverStatuses = ref<McpServerStatus[]>([])
  const showAddModal = ref(false)
  const showEditModal = ref(false)
  const editingServerName = ref<string | null>(null)
  const expandedServers = ref<Record<string, boolean>>({})

  const newServerName = ref('')
  const newServerType = ref<'stdio' | 'sse' | 'streamable_http'>('stdio')
  const newServerCommand = ref('')
  const newServerArgs = ref('')
  const newServerUrl = ref('')
  const newServerEnv = ref('')

  // Form validation errors
  const formErrors = ref({
    name: '',
    command: '',
    url: '',
    env: '',
  })

  const clearErrors = () => {
    formErrors.value = { name: '', command: '', url: '', env: '' }
  }

  const validateAddForm = (): boolean => {
    clearErrors()
    let isValid = true

    if (!newServerName.value.trim()) {
      formErrors.value.name = t('mcp_settings.name_required')
      isValid = false
    }

    if (newServerType.value === 'stdio') {
      if (!newServerCommand.value.trim()) {
        formErrors.value.command = t('mcp_settings.command_required')
        isValid = false
      }
      if (newServerEnv.value.trim()) {
        try {
          JSON.parse(newServerEnv.value)
        } catch (error) {
          console.warn('Invalid MCP server env JSON in create form:', error)
          formErrors.value.env = t('mcp_settings.invalid_json')
          isValid = false
        }
      }
    } else {
      if (!newServerUrl.value.trim()) {
        formErrors.value.url = t('mcp_settings.url_required')
        isValid = false
      }
    }

    return isValid
  }

  const validateEditForm = (): boolean => {
    clearErrors()
    let isValid = true

    if (!newServerName.value.trim()) {
      formErrors.value.name = t('mcp_settings.name_required')
      isValid = false
    }

    if (newServerType.value === 'stdio') {
      if (!newServerCommand.value.trim()) {
        formErrors.value.command = t('mcp_settings.command_required')
        isValid = false
      }
      if (newServerEnv.value.trim()) {
        try {
          JSON.parse(newServerEnv.value)
        } catch (error) {
          console.warn('Invalid MCP server env JSON in edit form:', error)
          formErrors.value.env = t('mcp_settings.invalid_json')
          isValid = false
        }
      }
    } else {
      if (!newServerUrl.value.trim()) {
        formErrors.value.url = t('mcp_settings.url_required')
        isValid = false
      }
    }

    return isValid
  }

  const isJsonMode = ref(false)
  const jsonContent = ref('')
  const jsonError = ref('')

  const registryServers = ref<SmitheryServer[]>([])
  const registryLoading = ref(false)
  const registrySearch = ref('')

  const loadRegistryServers = async (search = '') => {
    registryLoading.value = true
    try {
      const url = search ? `${SMITHERY_API}?q=${encodeURIComponent(search)}&pageSize=12` : `${SMITHERY_API}?pageSize=12`
      const response = await fetch(url)
      const data: SmitheryResponse = await response.json()
      registryServers.value = data.servers
    } catch (error) {
      console.warn('Failed to load registry servers:', error)
      registryServers.value = []
    } finally {
      registryLoading.value = false
    }
  }

  const debouncedSearch = debounce((search: string) => {
    loadRegistryServers(search)
  }, 300)

  watch(registrySearch, val => {
    debouncedSearch(val)
  })

  const formatUseCount = (count: number) => {
    if (count >= 1000000) return `${(count / 1000000).toFixed(1)}M`
    if (count >= 1000) return `${(count / 1000).toFixed(1)}K`
    return count.toString()
  }

  const installServer = async (server: SmitheryServer) => {
    const serverName = server.slug || server.qualifiedName.split('/').pop() || server.displayName

    const config: McpServerConfig = server.remote
      ? {
          type: 'sse' as const,
          url: `https://server.smithery.ai/${server.qualifiedName}/sse`,
        }
      : {
          type: 'stdio' as const,
          command: 'npx',
          args: ['-y', `@smithery/${server.qualifiedName}`],
        }

    mcpServers.value[serverName] = config
    const settings = await settingsApi.getGlobal()
    settings.mcpServers = mcpServers.value
    await settingsApi.updateGlobal(settings)
    createMessage.success(t('mcp_settings.server_added'))
    await loadServers()
    await reloadMcpRegistry()
  }

  const loadServers = async () => {
    isLoading.value = true
    const settings = await settingsApi.getGlobal()
    mcpServers.value = settings.mcpServers || {}
    jsonContent.value = JSON.stringify(mcpServers.value, null, 2)

    // 获取服务器状态
    if (currentWorkspace.value) {
      try {
        serverStatuses.value = await mcpApi.listServers(currentWorkspace.value)
      } catch (e) {
        console.error('Failed to load server statuses:', e)
        serverStatuses.value = []
      }
    }
    isLoading.value = false
  }

  // 获取服务器原始状态
  const getServerRawStatus = (name: string) => {
    return serverStatuses.value.find(s => s.name === name)?.status || 'disconnected'
  }

  const getDisabledTools = (config: McpServerConfig): string[] => {
    return config.disabledTools || []
  }

  const withDisabledTools = (config: McpServerConfig, disabledTools: string[]): McpServerConfig => {
    const nextDisabledTools = Array.from(new Set(disabledTools))

    if (config.type === 'stdio') {
      return {
        ...config,
        disabledTools: nextDisabledTools.length > 0 ? nextDisabledTools : undefined,
      }
    }

    return {
      ...config,
      disabledTools: nextDisabledTools.length > 0 ? nextDisabledTools : undefined,
    }
  }

  const getServerTools = (name: string): McpToolInfo[] => {
    return serverStatuses.value.find(s => s.name === name)?.tools || []
  }

  const isToolEnabled = (config: McpServerConfig, toolName: string) => {
    return !getDisabledTools(config).includes(toolName)
  }

  const isServerExpanded = (name: string) => {
    return !!expandedServers.value[name]
  }

  const toggleServerExpanded = (name: string) => {
    expandedServers.value = {
      ...expandedServers.value,
      [name]: !expandedServers.value[name],
    }
  }

  // 获取服务器状态文本（翻译后）
  const getServerStatusText = (name: string) => {
    const status = getServerRawStatus(name)
    switch (status) {
      case 'connected':
        return t('mcp_dialog.status_connected')
      case 'error':
        return t('mcp_dialog.status_error')
      default:
        return t('mcp_dialog.status_disconnected')
    }
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'connected':
        return 'var(--color-success)'
      case 'error':
        return 'var(--color-error)'
      default:
        return 'var(--text-400)'
    }
  }

  const toggleJsonMode = () => {
    if (!isJsonMode.value) {
      jsonContent.value = JSON.stringify(mcpServers.value, null, 2)
      jsonError.value = ''
    }
    isJsonMode.value = !isJsonMode.value
  }

  const saveJsonConfig = async () => {
    try {
      const parsed = JSON.parse(jsonContent.value)
      jsonError.value = ''
      mcpServers.value = parsed
      const settings = await settingsApi.getGlobal()
      settings.mcpServers = parsed
      await settingsApi.updateGlobal(settings)
      createMessage.success(t('mcp_settings.config_saved'))
      await loadServers()
      await reloadMcpRegistry()
      isJsonMode.value = false
    } catch (e) {
      jsonError.value =
        e instanceof SyntaxError
          ? t('mcp_settings.invalid_json')
          : e instanceof Error
            ? e.message
            : t('mcp_settings.invalid_json')
    }
  }

  const toggleServer = async (name: string, enabled: boolean) => {
    const config = mcpServers.value[name]
    if (config) {
      config.disabled = !enabled
      const settings = await settingsApi.getGlobal()
      settings.mcpServers = mcpServers.value
      await settingsApi.updateGlobal(settings)
      createMessage.success(enabled ? t('mcp_settings.server_enabled') : t('mcp_settings.server_disabled'))
      await reloadMcpRegistry()
    }
  }

  const toggleServerTool = async (serverName: string, toolName: string, enabled: boolean) => {
    const config = mcpServers.value[serverName]
    if (!config) return

    const disabledTools = getDisabledTools(config)
    const nextDisabledTools = enabled ? disabledTools.filter(name => name !== toolName) : [...disabledTools, toolName]

    mcpServers.value[serverName] = withDisabledTools(config, nextDisabledTools)
    const settings = await settingsApi.getGlobal()
    settings.mcpServers = mcpServers.value
    await settingsApi.updateGlobal(settings)
    await reloadMcpRegistry()
    await loadServers()
  }

  const resetForm = () => {
    newServerName.value = ''
    newServerType.value = 'stdio'
    newServerCommand.value = ''
    newServerArgs.value = ''
    newServerUrl.value = ''
    newServerEnv.value = ''
    clearErrors()
  }

  const openAddModal = () => {
    resetForm()
    showAddModal.value = true
  }

  const openEditModal = (name: string) => {
    clearErrors()
    const config = mcpServers.value[name]
    if (config) {
      editingServerName.value = name
      newServerName.value = name
      newServerType.value = config.type
      if (config.type === 'stdio') {
        newServerCommand.value = config.command
        newServerArgs.value = config.args?.join('\n') || ''
        newServerEnv.value = config.env ? JSON.stringify(config.env, null, 2) : ''
      } else {
        newServerUrl.value = config.url
      }
      showEditModal.value = true
    }
  }

  const saveNewServer = async () => {
    if (!validateAddForm()) return

    let config: McpServerConfig
    if (newServerType.value === 'stdio') {
      config = {
        type: 'stdio',
        command: newServerCommand.value.trim(),
        args: newServerArgs.value.trim() ? newServerArgs.value.trim().split('\n').filter(Boolean) : undefined,
        env: newServerEnv.value.trim() ? JSON.parse(newServerEnv.value) : undefined,
      }
    } else {
      config = {
        type: newServerType.value,
        url: newServerUrl.value.trim(),
      }
    }

    mcpServers.value[newServerName.value.trim()] = config
    const settings = await settingsApi.getGlobal()
    settings.mcpServers = mcpServers.value
    await settingsApi.updateGlobal(settings)
    showAddModal.value = false
    resetForm()
    createMessage.success(t('mcp_settings.server_added'))
    await loadServers()
    await reloadMcpRegistry()
  }

  const updateServer = async () => {
    if (!validateEditForm()) return
    if (!editingServerName.value) return

    const existingConfig = mcpServers.value[editingServerName.value]
    const existingDisabledTools = existingConfig ? getDisabledTools(existingConfig) : []

    let config: McpServerConfig
    if (newServerType.value === 'stdio') {
      config = {
        type: 'stdio',
        command: newServerCommand.value.trim(),
        args: newServerArgs.value.trim() ? newServerArgs.value.trim().split('\n').filter(Boolean) : undefined,
        env: newServerEnv.value.trim() ? JSON.parse(newServerEnv.value) : undefined,
        disabledTools: existingDisabledTools.length > 0 ? existingDisabledTools : undefined,
      }
    } else {
      config = {
        type: newServerType.value,
        url: newServerUrl.value.trim(),
        disabledTools: existingDisabledTools.length > 0 ? existingDisabledTools : undefined,
      }
    }

    if (editingServerName.value !== newServerName.value.trim()) {
      delete mcpServers.value[editingServerName.value]
    }
    mcpServers.value[newServerName.value.trim()] = config

    const settings = await settingsApi.getGlobal()
    settings.mcpServers = mcpServers.value
    await settingsApi.updateGlobal(settings)
    showEditModal.value = false
    editingServerName.value = null
    resetForm()
    createMessage.success(t('mcp_settings.server_updated'))
    await loadServers()
    await reloadMcpRegistry()
  }

  const deleteServer = async (name: string) => {
    delete mcpServers.value[name]
    const settings = await settingsApi.getGlobal()
    settings.mcpServers = mcpServers.value
    await settingsApi.updateGlobal(settings)
    showEditModal.value = false
    editingServerName.value = null
    createMessage.success(t('mcp_settings.server_deleted'))
    await loadServers()
    await reloadMcpRegistry()
  }

  const openDocs = () => {
    window.open('https://modelcontextprotocol.io/docs', '_blank')
  }

  const init = async () => {
    await loadServers()
    await loadRegistryServers()
  }

  onMounted(init)
  defineExpose({ init })
</script>

<template>
  <div class="settings-section">
    <h1 class="section-title">{{ t('mcp_settings.title') }}</h1>
    <p class="section-description">
      {{ t('mcp_settings.description') }}
      <a class="docs-link" @click="openDocs">
        {{ t('mcp_settings.docs') }}
        <svg class="external-link-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
          <polyline points="15 3 21 3 21 9" />
          <line x1="10" y1="14" x2="21" y2="3" />
        </svg>
      </a>
    </p>

    <!-- Custom Servers -->
    <div class="settings-group">
      <div class="group-header">
        <h2 class="group-title">{{ t('mcp_settings.custom_servers') }}</h2>
        <div class="header-actions">
          <button class="mode-btn" @click="toggleJsonMode">
            <svg v-if="!isJsonMode" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="16 18 22 12 16 6" />
              <polyline points="8 6 2 12 8 18" />
            </svg>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="7" height="7" />
              <rect x="14" y="3" width="7" height="7" />
              <rect x="14" y="14" width="7" height="7" />
              <rect x="3" y="14" width="7" height="7" />
            </svg>
            {{ isJsonMode ? t('mcp_settings.form_mode') : t('mcp_settings.json_mode') }}
          </button>
          <button v-if="!isJsonMode" class="add-btn" @click="openAddModal">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="12" y1="5" x2="12" y2="19" />
              <line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            {{ t('mcp_settings.add_server') }}
          </button>
        </div>
      </div>

      <!-- JSON Mode -->
      <div v-if="isJsonMode" class="json-editor-container">
        <div class="settings-card json-card">
          <textarea
            v-model="jsonContent"
            class="json-editor"
            spellcheck="false"
            :placeholder="t('mcp_settings.json_placeholder')"
          />
        </div>
        <div v-if="jsonError" class="json-error">{{ jsonError }}</div>
        <button class="save-json-btn" @click="saveJsonConfig">
          {{ t('common.save') }}
        </button>
      </div>

      <!-- Form Mode -->
      <div v-else class="settings-card">
        <div v-if="Object.keys(mcpServers).length === 0" class="empty-state">
          <span>{{ t('mcp_settings.no_servers') }}</span>
        </div>

        <div v-for="(config, name) in mcpServers" :key="name" class="server-item">
          <div class="server-row custom">
            <div class="server-info">
              <button
                v-if="getServerTools(name as string).length > 0"
                class="expand-btn"
                type="button"
                :title="isServerExpanded(name as string) ? t('common.collapse') : t('common.expand')"
                @click="toggleServerExpanded(name as string)"
              >
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  :class="{ expanded: isServerExpanded(name as string) }"
                >
                  <polyline points="9 18 15 12 9 6" />
                </svg>
              </button>
              <span class="server-name">{{ name }}</span>
              <span class="server-status" :style="{ color: getStatusColor(getServerRawStatus(name as string)) }">
                {{ getServerStatusText(name as string) }}
              </span>
              <span v-if="getServerTools(name as string).length > 0" class="server-tools-count">
                {{ t('mcp_dialog.tools_count', { count: getServerTools(name as string).length }) }}
              </span>
            </div>
            <div class="server-actions">
              <button class="icon-btn" :title="t('common.edit')" @click="openEditModal(name as string)">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="1.5" fill="currentColor" />
                  <circle cx="19" cy="12" r="1.5" fill="currentColor" />
                  <circle cx="5" cy="12" r="1.5" fill="currentColor" />
                </svg>
              </button>
              <XSwitch :model-value="!config.disabled" @update:model-value="toggleServer(name as string, $event)" />
            </div>
          </div>

          <div
            v-if="getServerTools(name as string).length > 0 && isServerExpanded(name as string)"
            class="server-tools"
          >
            <div v-for="tool in getServerTools(name as string)" :key="`${name}-${tool.name}`" class="tool-row">
              <div class="tool-info">
                <span class="tool-name">{{ tool.name }}</span>
                <span v-if="tool.description" class="tool-description">{{ tool.description }}</span>
              </div>
              <div class="tool-switch">
                <XSwitch
                  :model-value="isToolEnabled(config, tool.name)"
                  :disabled="!!config.disabled"
                  @update:model-value="toggleServerTool(name as string, tool.name, $event)"
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Registry Servers -->
    <div class="settings-group">
      <div class="group-header">
        <h2 class="group-title">{{ t('mcp_settings.registry_servers') }}</h2>
        <a class="registry-link" href="https://smithery.ai/servers" target="_blank">
          {{ t('mcp_settings.browse_registry') }}
          <svg class="external-link-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
            <polyline points="15 3 21 3 21 9" />
            <line x1="10" y1="14" x2="21" y2="3" />
          </svg>
        </a>
      </div>

      <div class="search-box">
        <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <input
          v-model="registrySearch"
          type="text"
          class="search-input"
          :placeholder="t('mcp_settings.search_registry')"
        />
      </div>

      <div class="settings-card">
        <div v-if="registryLoading" class="loading-state">
          <span>{{ t('common.loading') }}</span>
        </div>

        <div v-else-if="registryServers.length === 0" class="empty-state">
          <span>{{ t('mcp_settings.no_registry_servers') }}</span>
        </div>

        <div v-for="server in registryServers" :key="server.id" class="server-row registry">
          <div class="server-info">
            <div class="server-icon-box">
              <img
                v-if="server.iconUrl"
                :src="server.iconUrl"
                :alt="server.displayName"
                class="server-icon-img"
                @error="(e: Event) => ((e.target as HTMLImageElement).style.display = 'none')"
              />
              <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <rect x="3" y="3" width="18" height="18" rx="2" />
                <path d="M9 9h6M9 12h6M9 15h4" />
              </svg>
            </div>
            <div class="server-details">
              <div class="server-name-row">
                <strong class="server-display-name">{{ server.displayName }}</strong>
                <svg v-if="server.verified" class="verified-icon" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
              </div>
              <span class="server-author">{{ server.namespace }}</span>
              <span class="server-description">{{ server.description }}</span>
            </div>
          </div>
          <div class="server-actions-col">
            <span class="use-count">{{ formatUseCount(server.useCount) }} {{ t('mcp_settings.uses') }}</span>
            <button class="install-btn" @click="installServer(server)">
              {{ t('mcp_settings.install') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Add Server Modal -->
    <XModal v-model:visible="showAddModal" :title="t('mcp_settings.add_server')" size="small" :show-footer="true">
      <div class="modal-form">
        <XFormGroup :label="t('mcp_settings.server_name')" required :error="formErrors.name">
          <XInput v-model="newServerName" :placeholder="t('mcp_settings.server_name_placeholder')" />
        </XFormGroup>

        <XFormGroup :label="t('mcp_settings.server_type')">
          <select v-model="newServerType" class="form-select">
            <option value="stdio">Stdio</option>
            <option value="sse">SSE</option>
            <option value="streamable_http">Streamable HTTP</option>
          </select>
        </XFormGroup>

        <template v-if="newServerType === 'stdio'">
          <XFormGroup :label="t('mcp_settings.command')" required :error="formErrors.command">
            <XInput v-model="newServerCommand" placeholder="npx -y @modelcontextprotocol/server" />
          </XFormGroup>
          <XFormGroup :label="t('mcp_settings.args')" :hint="t('mcp_settings.args_hint')">
            <XTextarea v-model="newServerArgs" :rows="3" :placeholder="t('mcp_settings.args_placeholder')" />
          </XFormGroup>
          <XFormGroup :label="t('mcp_settings.env')" :error="formErrors.env">
            <XTextarea v-model="newServerEnv" :rows="3" :placeholder="t('mcp_settings.env_placeholder')" />
          </XFormGroup>
        </template>

        <template v-else>
          <XFormGroup label="URL" required :error="formErrors.url">
            <XInput v-model="newServerUrl" placeholder="https://example.com/mcp" />
          </XFormGroup>
        </template>
      </div>

      <template #footer>
        <div class="modal-actions">
          <XButton variant="secondary" @click="showAddModal = false">{{ t('common.cancel') }}</XButton>
          <XButton variant="primary" @click="saveNewServer">{{ t('common.save') }}</XButton>
        </div>
      </template>
    </XModal>

    <!-- Edit Server Modal -->
    <XModal v-model:visible="showEditModal" :title="t('mcp_settings.edit_server')" size="small" :show-footer="true">
      <div class="modal-form">
        <XFormGroup :label="t('mcp_settings.server_name')" required :error="formErrors.name">
          <XInput v-model="newServerName" />
        </XFormGroup>

        <XFormGroup :label="t('mcp_settings.server_type')">
          <select v-model="newServerType" class="form-select">
            <option value="stdio">Stdio</option>
            <option value="sse">SSE</option>
            <option value="streamable_http">Streamable HTTP</option>
          </select>
        </XFormGroup>

        <template v-if="newServerType === 'stdio'">
          <XFormGroup :label="t('mcp_settings.command')" required :error="formErrors.command">
            <XInput v-model="newServerCommand" />
          </XFormGroup>
          <XFormGroup :label="t('mcp_settings.args')">
            <XTextarea v-model="newServerArgs" :rows="3" />
          </XFormGroup>
          <XFormGroup :label="t('mcp_settings.env')" :error="formErrors.env">
            <XTextarea v-model="newServerEnv" :rows="3" />
          </XFormGroup>
        </template>

        <template v-else>
          <XFormGroup label="URL" required :error="formErrors.url">
            <XInput v-model="newServerUrl" />
          </XFormGroup>
        </template>
      </div>

      <template #footer>
        <div class="modal-actions">
          <XButton variant="danger" @click="deleteServer(editingServerName!)">{{ t('common.delete') }}</XButton>
          <div class="modal-actions-right">
            <XButton variant="secondary" @click="showEditModal = false">{{ t('common.cancel') }}</XButton>
            <XButton variant="primary" @click="updateServer">{{ t('common.save') }}</XButton>
          </div>
        </div>
      </template>
    </XModal>
  </div>
</template>

<style scoped>
  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }

  .section-title {
    font-size: 24px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 8px 0;
  }

  .section-description {
    font-size: 13px;
    color: var(--text-400);
    margin: -4px 0 0 0;
  }

  .docs-link {
    color: var(--color-primary);
    cursor: pointer;
    text-decoration: none;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    transition: opacity 0.15s ease;
  }

  .docs-link:hover {
    opacity: 0.8;
  }

  .external-link-icon {
    width: 12px;
    height: 12px;
  }

  .settings-group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .group-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-left: 4px;
  }

  .group-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-400);
    margin: 0;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .mode-btn,
  .add-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    background: transparent;
    border: none;
    font-size: 13px;
    font-weight: 400;
    color: var(--text-400);
    cursor: pointer;
    transition: color 0.15s ease;
  }

  .mode-btn:hover,
  .add-btn:hover {
    color: var(--text-200);
  }

  .mode-btn svg,
  .add-btn svg {
    width: 14px;
    height: 14px;
  }

  .settings-card {
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-xl);
    overflow: hidden;
  }

  .json-editor-container {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .json-card {
    padding: 0;
  }

  .json-editor {
    width: 100%;
    min-height: 300px;
    padding: 16px;
    background: var(--bg-200);
    border: none;
    border-radius: var(--border-radius-xl);
    font-size: 13px;
    font-family: var(--font-family-mono);
    color: var(--text-100);
    line-height: 1.6;
    resize: vertical;
    outline: none;
  }

  .json-editor::placeholder {
    color: var(--text-500);
  }

  .json-error {
    font-size: 13px;
    color: var(--color-error);
    padding-left: 4px;
  }

  .save-json-btn {
    align-self: flex-end;
    padding: 8px 20px;
    background: var(--text-100);
    border: none;
    border-radius: var(--border-radius-lg);
    font-size: 13px;
    font-weight: 500;
    color: var(--bg-100);
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .save-json-btn:hover {
    opacity: 0.9;
  }

  .empty-state {
    padding: 24px;
    text-align: center;
    color: var(--text-500);
    font-size: 13px;
  }

  .server-item:not(:last-child) {
    border-bottom: 1px solid var(--border-100);
  }

  .server-row.custom {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    min-height: 60px;
  }

  .server-row.custom .server-info {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
    flex-wrap: wrap;
  }

  .expand-btn {
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-500);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    flex: 0 0 20px;
  }

  .expand-btn:hover {
    color: var(--text-200);
  }

  .expand-btn svg {
    width: 14px;
    height: 14px;
    transition: transform 0.15s ease;
  }

  .expand-btn svg.expanded {
    transform: rotate(90deg);
  }

  .server-row.custom .server-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-100);
  }

  .server-row.custom .server-status {
    font-size: 12px;
  }

  .server-tools-count {
    font-size: 12px;
    color: var(--text-500);
  }

  .server-row.custom .server-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .server-tools {
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 0 20px 12px 52px;
  }

  .tool-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 0;
  }

  .tool-row:not(:last-child) {
    border-bottom: 1px solid var(--border-100);
  }

  .tool-info {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: 1;
    min-width: 0;
  }

  .tool-name {
    font-size: 13px;
    color: var(--text-200);
    word-break: break-word;
  }

  .tool-description {
    font-size: 12px;
    color: var(--text-500);
    line-height: 1.4;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    word-break: break-word;
  }

  .tool-switch {
    flex: 0 0 44px;
    width: 44px;
    display: flex;
    justify-content: flex-end;
    align-items: center;
  }

  .registry-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 13px;
    color: var(--color-primary);
    text-decoration: none;
    transition: opacity 0.15s ease;
  }

  .registry-link:hover {
    opacity: 0.8;
  }

  .registry-link .external-link-icon {
    width: 14px;
    height: 14px;
  }

  .search-box {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 12px;
    width: 16px;
    height: 16px;
    color: var(--text-500);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 10px 12px 10px 38px;
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    font-size: 14px;
    color: var(--text-100);
    outline: none;
    transition: border-color 0.15s ease;
  }

  .search-input:focus {
    border-color: var(--color-primary);
  }

  .search-input::placeholder {
    color: var(--text-500);
  }

  .loading-state {
    padding: 24px;
    text-align: center;
    color: var(--text-500);
    font-size: 13px;
  }

  .server-row.registry {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 16px 20px;
    gap: 16px;
  }

  .server-row.registry:not(:last-child) {
    border-bottom: 1px solid var(--border-100);
  }

  .server-row.registry .server-info {
    display: flex;
    align-items: flex-start;
    gap: 14px;
    flex: 1;
    min-width: 0;
  }

  .server-icon-box {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-300);
    border-radius: var(--border-radius-xl);
    flex-shrink: 0;
    overflow: hidden;
  }

  .server-icon-box svg {
    width: 22px;
    height: 22px;
    color: var(--text-400);
  }

  .server-icon-img {
    width: 24px;
    height: 24px;
    object-fit: contain;
  }

  .verified-icon {
    width: 14px;
    height: 14px;
    color: var(--color-info);
    flex-shrink: 0;
  }

  .server-details {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .server-name-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .server-display-name {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-100);
  }

  .server-author {
    font-size: 12px;
    color: var(--text-400);
  }

  .server-description {
    font-size: 13px;
    color: var(--text-400);
    line-height: 1.4;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .server-actions-col {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
    flex-shrink: 0;
  }

  .use-count {
    font-size: 12px;
    color: var(--text-500);
    white-space: nowrap;
  }

  .icon-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--border-radius-md);
    color: var(--text-400);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    background: var(--bg-300);
    color: var(--text-200);
  }

  .icon-btn svg {
    width: 16px;
    height: 16px;
  }

  .install-btn {
    padding: 8px 16px;
    background: var(--bg-300);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    font-size: 13px;
    font-weight: 500;
    color: var(--text-200);
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .install-btn:hover:not(:disabled) {
    background: var(--bg-400);
    border-color: var(--border-300);
  }

  .install-btn:disabled {
    color: var(--text-400);
    cursor: not-allowed;
  }

  .modal-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 8px 0;
  }

  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-300);
  }

  .form-input,
  .form-textarea {
    padding: 10px 12px;
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    font-size: 14px;
    color: var(--text-100);
    outline: none;
    transition: border-color 0.15s ease;
  }

  .form-select {
    width: 100%;
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
    padding: 10px 36px 10px 12px;
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    font-size: 14px;
    color: var(--text-100);
    outline: none;
    cursor: pointer;
    transition: border-color 0.15s ease;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%239ca3af' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 12px center;
  }

  .form-select:hover {
    border-color: var(--border-300);
  }

  .form-select:focus {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(var(--color-primary-rgb), 0.1);
  }

  .form-select option {
    background: var(--bg-100);
    color: var(--text-100);
    padding: 8px;
  }

  .form-input:focus,
  .form-select:focus,
  .form-textarea:focus {
    border-color: var(--color-primary);
  }

  .form-textarea {
    resize: vertical;
    min-height: 80px;
    font-family: var(--font-family-mono);
  }

  .modal-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .modal-actions-right {
    display: flex;
    gap: 12px;
  }
</style>
