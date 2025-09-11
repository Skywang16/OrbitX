<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import AISettings from '@/components/settings/components/AI/AISettings.vue'
  import ThemeSettings from '@/components/settings/components/Theme/ThemeSettings.vue'
  import ShortcutSettings from '@/components/settings/components/Shortcuts/ShortcutSettings.vue'
  import { LanguageSettings } from '@/components/settings/components/Language'
  import { GeneralSettings } from '@/components/settings/components/General'
  import { VectorIndexSettings } from '@/components/settings/components/VectorIndex'
  import SettingsNav from '@/components/settings/SettingsNav.vue'
  import { XButton } from '@/ui'
  import { configApi } from '@/api/config'
  import { onMounted, ref, nextTick } from 'vue'
  import { debounce } from 'lodash-es'

  const { t } = useI18n()
  const activeSection = ref<string>('general')

  // 组件引用，用于调用各页面的初始化方法
  const aiSettingsRef = ref()
  const themeSettingsRef = ref()
  const shortcutSettingsRef = ref()
  const generalSettingsRef = ref()
  const vectorIndexSettingsRef = ref()

  // 移除这个缓存机制，每次切换都重新初始化

  onMounted(async () => {
    // 初始化当前显示的页面（默认是general）
    await initializeCurrentSection()
  })

  const initializeCurrentSection = async () => {
    const section = activeSection.value

    try {
      // 等待组件渲染完成
      await nextTick()

      switch (section) {
        case 'general':
          if (generalSettingsRef.value?.init) {
            await generalSettingsRef.value.init()
          }
          break
        case 'ai':
          if (aiSettingsRef.value?.init) {
            await aiSettingsRef.value.init()
          }
          break
        case 'theme':
          if (themeSettingsRef.value?.init) {
            await themeSettingsRef.value.init()
          }
          break
        case 'shortcuts':
          if (shortcutSettingsRef.value?.init) {
            await shortcutSettingsRef.value.init()
          }
          break
        case 'vectorIndex':
          if (vectorIndexSettingsRef.value?.init) {
            await vectorIndexSettingsRef.value.init()
          }
          break
        case 'language':
          // 语言设置没有需要初始化的接口
          break
        default:
          break
      }
    } catch (error) {
      console.error(`Failed to initialize ${section} settings:`, error)
    }
  }

  const handleNavigationChange = async (section: string) => {
    activeSection.value = section
    // 当切换页面时，初始化新页面
    await initializeCurrentSection()
  }

  const openConfigFolder = async () => {
    await configApi.openConfigFolder()
  }

  // 创建防抖版本的函数，防止用户快速点击导致重复调用
  const handleOpenConfigFolder = debounce(openConfigFolder, 500)
</script>

<template>
  <div class="settings-container">
    <div class="settings-content">
      <!-- 左侧导航 -->
      <div class="settings-sidebar">
        <SettingsNav :activeSection="activeSection" @change="handleNavigationChange" />

        <!-- 底部按钮区域 -->
        <div class="settings-sidebar-footer">
          <XButton variant="primary" size="medium" @click="handleOpenConfigFolder">
            {{ t('settings.general.open_config_folder') }}
          </XButton>
        </div>
      </div>

      <!-- 右侧内容区域 -->
      <div class="settings-main">
        <div class="settings-panel">
          <GeneralSettings v-if="activeSection === 'general'" ref="generalSettingsRef" />
          <AISettings v-else-if="activeSection === 'ai'" ref="aiSettingsRef" />
          <VectorIndexSettings v-else-if="activeSection === 'vectorIndex'" ref="vectorIndexSettingsRef" />
          <ThemeSettings v-else-if="activeSection === 'theme'" ref="themeSettingsRef" />
          <ShortcutSettings v-else-if="activeSection === 'shortcuts'" ref="shortcutSettingsRef" />
          <LanguageSettings v-else-if="activeSection === 'language'" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .settings-sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .settings-sidebar-footer {
    display: flex;
    justify-content: center;
    padding: var(--spacing-md);
    background: var(--bg-200);
    border-top: 1px solid var(--border-200);
  }

  /* 响应式设计 */
  @media (max-width: 480px) {
    .settings-sidebar-footer {
      padding: var(--spacing-sm);
    }
  }
</style>
