<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ $t('settings.vectorIndex.title') }}</h2>
    <!-- 功能开关 -->
    <div class="settings-group">
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ $t('settings.vectorIndex.feature_enabled') }}</div>
          <div class="settings-description">{{ $t('settings.vectorIndex.feature_enabled_description') }}</div>
        </div>
        <div class="settings-item-control">
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
  import { ref, reactive, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { vectorIndexAppSettingsApi } from '@/api/vector-index/app-settings'
  import XSwitch from '@/ui/components/Switch.vue'
  import XButton from '@/ui/components/Button.vue'

  const { t } = useI18n()

  // 响应式数据
  const settings = reactive({
    enabled: false,
    workspaces: [] as string[],
  })

  const needsRestart = ref(false)
  const originalEnabled = ref(false)

  // 加载设置
  const loadSettings = async () => {
    try {
      const appSettings = await vectorIndexAppSettingsApi.getSettings()
      settings.enabled = appSettings.enabled
      settings.workspaces = [...appSettings.workspaces]
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
  }

  // 重启应用
  const restartApp = () => {
    // TODO: 实现重启逻辑
  }

  // 忽略重启提示
  const dismissRestartNotice = () => {
    needsRestart.value = false
  }

  // 组件挂载时加载设置
  onMounted(() => {
    loadSettings()
  })
</script>

<style scoped>
  /* 使用全局设置样式，无需自定义样式 */
</style>
