<script setup lang="ts">
  import { computed, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { aiApi, workspaceApi } from '@/api'
  import { debounce } from 'lodash-es'
  import { XSelect } from '@/ui'
  import SettingsCard from '../../../SettingsCard.vue'

  const { t } = useI18n()

  const userRules = ref<string>('')
  const isLoadingRules = ref(false)

  const projectRules = ref<string>('')
  const isLoadingProjectRules = ref(false)

  const rulesOptions = computed(() => [
    { value: '', label: t('ai_feature.rules_auto') },
    { value: 'CLAUDE.md', label: t('ai_feature.rules_claude') },
    { value: 'AGENTS.md', label: t('ai_feature.rules_agents') },
    { value: 'WARP.md', label: t('ai_feature.rules_warp') },
    { value: '.cursorrules', label: t('ai_feature.rules_cursorrules') },
    { value: 'README.md', label: t('ai_feature.rules_readme') },
  ])

  const loadUserRules = async () => {
    isLoadingRules.value = true
    try {
      const rules = await aiApi.getUserRules()
      userRules.value = rules || ''
    } catch (error) {
      // Error handled silently
    } finally {
      isLoadingRules.value = false
    }
  }

  const saveUserRules = async (value: string) => {
    try {
      const rulesToSave = value.trim() || null
      await aiApi.setUserRules(rulesToSave)
    } catch (error) {
      // Error handled silently
    }
  }

  const loadProjectRules = async () => {
    isLoadingProjectRules.value = true
    const rules = await workspaceApi.getProjectRules()
    projectRules.value = rules || ''
    isLoadingProjectRules.value = false
  }

  const saveProjectRules = async (value: string) => {
    const rulesToSave = value.trim() || null
    await workspaceApi.setProjectRules(rulesToSave)
  }

  const handleProjectRulesChange = async (value: string | number | (string | number)[] | null) => {
    if (typeof value === 'string') {
      projectRules.value = value
      await saveProjectRules(value)
    }
  }

  const debouncedSaveUserRules = debounce((newValue: string) => {
    saveUserRules(newValue)
  }, 500)

  watch(userRules, debouncedSaveUserRules)

  onMounted(() => {
    loadUserRules()
    loadProjectRules()
  })
</script>

<template>
  <div class="settings-group">
    <h3 class="settings-group-title">{{ t('ai_feature.user_rules') }}</h3>

    <!-- Project Rules -->
    <SettingsCard>
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('ai_feature.project_rules') }}</div>
          <div class="settings-description">{{ t('ai_feature.project_rules_description') }}</div>
        </div>
        <div class="settings-item-control">
          <x-select
            :model-value="projectRules"
            :options="rulesOptions"
            @update:model-value="handleProjectRulesChange"
            :placeholder="t('ai_feature.rules_auto')"
            :disabled="isLoadingProjectRules"
          />
        </div>
      </div>
    </SettingsCard>

    <!-- User Rules -->
    <SettingsCard>
      <div class="settings-item">
        <div class="settings-item-header">
          <div class="settings-label">{{ t('ai_feature.user_rules') }}</div>
          <div class="settings-description">{{ t('ai_feature.rules_placeholder') }}</div>
        </div>
      </div>

      <div style="padding: 20px">
        <textarea
          v-model="userRules"
          class="settings-textarea"
          :placeholder="t('ai_feature.rules_placeholder')"
          :aria-label="t('ai_feature.user_rules')"
          rows="4"
          :disabled="isLoadingRules"
        ></textarea>
      </div>
    </SettingsCard>
  </div>
</template>

<style scoped></style>
