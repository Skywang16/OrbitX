<script setup lang="ts">
  import { ref, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { XSwitch } from '@/ui'
  import { createMessage } from '@/ui'
  import { enable as enableAutostart, disable as disableAutostart, isEnabled } from '@tauri-apps/plugin-autostart'
  import { debounce } from 'lodash-es'

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

  onMounted(() => {
    loadAutoStartStatus()
  })
</script>

<template>
  <div class="general-settings">
    <div class="settings-header">
      <h2 class="settings-title">{{ t('settings.general.title') }}</h2>
      <p class="settings-description">{{ t('settings.general.description') }}</p>
    </div>

    <div class="settings-content">
      <!-- 开机自启动 -->
      <div class="setting-group">
        <div class="setting-item">
          <div class="setting-main">
            <div class="setting-info">
              <h3 class="setting-label">{{ t('settings.general.autostart_title') }}</h3>
            </div>
            <div class="setting-control">
              <XSwitch :model-value="autoStartEnabled" @update:model-value="handleAutoStartToggle" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .general-settings {
    padding: var(--spacing-lg);
    max-width: 800px;
  }

  .settings-header {
    margin-bottom: var(--spacing-xl);
  }

  .settings-title {
    font-size: 24px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0 0 var(--spacing-sm) 0;
  }

  .settings-description {
    color: var(--text-300);
    margin: 0;
    font-size: 14px;
    line-height: 1.5;
  }

  .settings-content {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-lg);
  }

  .setting-group {
    background: var(--bg-300);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-lg);
    overflow: hidden;
  }

  .setting-item {
    padding: var(--spacing-md);
    border-bottom: 1px solid var(--border-300);
  }

  .setting-item:last-child {
    border-bottom: none;
  }

  .setting-main {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);
  }

  .setting-info {
    flex: 1;
    min-width: 0;
  }

  .setting-label {
    font-size: 16px;
    font-weight: 500;
    color: var(--text-100);
    margin: 0 0 var(--spacing-xs) 0;
  }

  .setting-description {
    font-size: 13px;
    color: var(--text-300);
    margin: 0;
    line-height: 1.4;
  }

  .setting-control {
    flex-shrink: 0;
  }

  /* 响应式设计 */
  @media (max-width: 600px) {
    .general-settings {
      padding: var(--spacing-md);
    }

    .setting-main {
      flex-direction: column;
      align-items: flex-start;
      gap: var(--spacing-sm);
    }

    .setting-control {
      width: 100%;
      display: flex;
      justify-content: flex-end;
    }
  }
</style>
