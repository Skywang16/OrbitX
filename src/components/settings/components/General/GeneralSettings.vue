<script setup lang="ts">
  import { ref } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { XSwitch, createMessage } from '@/ui'
  import { enable as enableAutostart, disable as disableAutostart, isEnabled } from '@tauri-apps/plugin-autostart'
  import { debounce } from 'lodash-es'
  import SettingsCard from '../../SettingsCard.vue'

  const { t } = useI18n()

  // 开机自启动状态
  const autoStartEnabled = ref(false)

  // 加载当前自启动状态
  const loadAutoStartStatus = async () => {
    try {
      autoStartEnabled.value = await isEnabled()
    } catch (error) {
      console.error('Failed to get autostart status:', error)
    }
  }

  // 切换开机自启动（带防抖）
  const handleAutoStartToggle = debounce(async (enabled: boolean) => {
    try {
      if (enabled) {
        await enableAutostart()
        createMessage.success(t('settings.general.autostart_enabled'))
      } else {
        await disableAutostart()
        createMessage.success(t('settings.general.autostart_disabled'))
      }
      autoStartEnabled.value = enabled
    } catch (error) {
      console.error('Failed to toggle autostart:', error)
      // 恢复原状态
      autoStartEnabled.value = !enabled
    }
  }, 300)

  // 初始化方法，供外部调用
  const init = async () => {
    await loadAutoStartStatus()
  }

  // 暴露初始化方法给父组件
  defineExpose({
    init,
  })
</script>

<template>
  <div class="settings-group">
    <h2 class="settings-section-title">{{ t('settings.general.title') }}</h2>

    <div class="settings-group">
      <h3 class="settings-group-title">{{ t('settings.general.startup_title') }}</h3>
      <SettingsCard>
        <div class="settings-item">
          <div class="settings-item-header">
            <div class="settings-label">{{ t('settings.general.autostart_title') }}</div>
            <div class="settings-description">{{ t('settings.general.description') }}</div>
          </div>
          <div class="settings-item-control">
            <XSwitch :model-value="autoStartEnabled" @update:model-value="handleAutoStartToggle" />
          </div>
        </div>
      </SettingsCard>
    </div>
  </div>
</template>

<style scoped></style>
