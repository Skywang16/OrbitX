<script setup lang="ts">
  import SettingsCard from '../../../SettingsCard.vue'
  import { settingsApi } from '@/api'
  import { useI18n } from 'vue-i18n'
  import { onMounted, ref } from 'vue'

  const { t } = useI18n()

  const isReloading = ref(false)
  const isSaving = ref(false)
  const error = ref<string | null>(null)
  const mcpJson = ref<string>('{}')

  const load = async () => {
    isReloading.value = true
    error.value = null
    try {
      const settings = await settingsApi.getGlobal()
      // 输出完整格式 { mcpServers: {...} }
      mcpJson.value = JSON.stringify({ mcpServers: settings.mcpServers || {} }, null, 2)
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      isReloading.value = false
    }
  }

  const save = async () => {
    isSaving.value = true
    error.value = null
    try {
      const parsed = JSON.parse(mcpJson.value || '{}')
      // 支持两种格式：{ mcpServers: {...} } 或直接 {...}
      const mcpServers = parsed.mcpServers ?? parsed
      const settings = await settingsApi.getGlobal()
      settings.mcpServers = mcpServers
      await settingsApi.updateGlobal(settings)
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      isSaving.value = false
    }
  }

  onMounted(load)
</script>

<template>
  <div class="settings-group">
    <h3 class="settings-group-title">{{ t('ai_feature.mcp_servers') }}</h3>
    <SettingsCard>
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('ai_feature.mcp_servers') }}</div>
          <div class="settings-description">{{ t('ai_feature.mcp_servers_description') }}</div>
        </div>
      </div>

      <div class="mcp-settings__body">
        <div v-if="error" class="settings-description mcp-settings__error">{{ error }}</div>
        <textarea v-model="mcpJson" class="settings-textarea" rows="12" :disabled="isReloading || isSaving" />

        <div class="mcp-settings__actions">
          <x-button variant="ghost" size="medium" :loading="isReloading" :disabled="isSaving" @click="load">
            {{ t('common.refresh') }}
          </x-button>
          <x-button variant="primary" size="medium" :loading="isSaving" :disabled="isReloading" @click="save">
            {{ t('common.save') }}
          </x-button>
        </div>
      </div>
    </SettingsCard>
  </div>
</template>
<style scoped>
  .mcp-settings__body {
    padding: 20px;
  }

  .mcp-settings__error {
    color: var(--error);
    margin-bottom: 10px;
  }

  .mcp-settings__actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 10px;
  }
</style>
