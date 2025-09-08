<template>
  <div class="settings-group">
    <!-- 功能开关 -->
    <div class="settings-group">
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ $t('settings.vectorIndex.feature_enabled') }}</div>
          <div class="settings-description">{{ $t('settings.vectorIndex.feature_enabled_description') }}</div>
        </div>
        <div class="settings-item-control">
          <!-- 连接状态指示器 -->
          <span v-if="settings.enabled" style="margin-right: 8px; color: var(--text-400); font-size: 12px">
            <template v-if="isConnecting">{{ $t('common.loading') }}</template>
            <template v-else>
              <span :style="{ color: isConnected ? 'var(--color-success)' : 'var(--color-error)' }">
                {{ connectionStatusText }}
              </span>
            </template>
          </span>
          <x-switch v-model="settings.enabled" @update:modelValue="onEnabledChange" />
        </div>
      </div>
    </div>

    <!-- 仅在启用时显示：已索引工作区列表（可删除） -->
    <template v-if="settings.enabled">
      <div class="settings-group">
        <h3 class="settings-group-title">{{ $t('settings.vectorIndex.workspace_management') }}</h3>
        <div v-if="settings.workspaces.length === 0" class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ $t('settings.vectorIndex.no_workspaces') }}</div>
            <div class="settings-description">{{ $t('settings.vectorIndex.no_workspaces_description') }}</div>
          </div>
        </div>

        <div v-for="(workspace, index) in settings.workspaces" :key="index" class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ workspace }}</div>
            <div class="settings-description">{{ $t('settings.vectorIndex.workspace_path_description') }}</div>
          </div>
          <div class="settings-item-control">
            <x-button variant="secondary" @click="removeWorkspace(index)">
              {{ $t('settings.vectorIndex.remove_workspace') }}
            </x-button>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
  import { ref, reactive, computed, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { vectorIndexAppSettingsApi } from '@/api/vector-index/app-settings'
  import { vectorIndexApi } from '@/api/vector-index'
  import XSwitch from '@/ui/components/Switch.vue'
  import XButton from '@/ui/components/Button.vue'

  // 响应式数据
  const settings = reactive({
    enabled: false,
    workspaces: [] as string[],
  })

  const needsRestart = ref(false)
  const originalEnabled = ref(false)
  const isConnecting = ref(false)
  const connectionMessage = ref('')
  const isConnected = ref<boolean | null>(null)
  const { t } = useI18n()

  const connectionStatusText = computed(() => {
    if (isConnecting.value) return '...'
    if (isConnected.value === true) return t('settings.vectorIndex.connection_success')
    if (isConnected.value === false) return t('settings.vectorIndex.connection_failed')
    return t('settings.vectorIndex.not_initialized')
  })

  // 加载设置
  const loadSettings = async () => {
    try {
      const appSettings = await vectorIndexAppSettingsApi.getSettings()
      settings.enabled = appSettings.enabled
      settings.workspaces = [...appSettings.workspaces]
      // 加载后尝试获取当前连接状态
      try {
        const status = await vectorIndexApi.getStatus()
        isConnected.value = !!status?.isInitialized
      } catch {
        isConnected.value = false
      }
    } catch (error) {
      console.error('加载向量索引全局设置失败:', error)
    }
  }

  // 保存设置（开关变化时保存）
  const saveSettings = async () => {
    try {
      await vectorIndexAppSettingsApi.saveSettings({
        enabled: settings.enabled,
        workspaces: settings.workspaces,
      })
    } catch (error) {
      console.error('保存向量索引全局设置失败:', error)
    }
  }

  // 移除工作目录
  const removeWorkspace = async (index: number) => {
    const workspace = settings.workspaces[index]

    try {
      await vectorIndexAppSettingsApi.removeWorkspace(workspace)
      settings.workspaces.splice(index, 1)
      // 工作目录移除成功
    } catch (error) {
      console.error('移除工作目录失败:', error)
      alert(`移除工作目录失败: ${error}`)
    }
  }

  // 开关变更处理
  const onEnabledChange = (enabled: boolean) => {
    if (enabled !== originalEnabled.value) {
      needsRestart.value = true
    }
    // 当开关切换时立即保存
    saveSettings()
    // 若开启则尝试连接
    if (enabled) {
      void tryConnect()
    }
  }

  // 手动尝试连接数据库
  const tryConnect = async () => {
    isConnecting.value = true
    connectionMessage.value = ''
    try {
      // 获取现有配置并尝试测试连接
      const cfg = await vectorIndexApi.getConfig()
      const result = await vectorIndexApi.testConnection(cfg)
      connectionMessage.value = result
      // 初始化服务
      await vectorIndexApi.init(cfg)
      const status = await vectorIndexApi.getStatus()
      isConnected.value = !!status?.isInitialized
    } catch (e) {
      isConnected.value = false
      connectionMessage.value = e instanceof Error ? e.message : String(e)
    } finally {
      isConnecting.value = false
    }
  }

  // 重启应用
  // const restartApp = () => {
  //   // TODO: 实现重启逻辑
  // }

  // 忽略重启提示
  // const dismissRestartNotice = () => {
  //   needsRestart.value = false
  // }

  // 组件挂载时加载设置
  onMounted(() => {
    loadSettings()
  })
</script>

<style scoped>
  /* 使用全局设置样式，无需自定义样式 */
</style>
