<script setup lang="ts">
  import AISettings from '@/components/settings/components/AI/AISettings.vue'
  import ThemeSettings from '@/components/settings/components/Theme/ThemeSettings.vue'
  import ShortcutSettings from '@/components/settings/components/Shortcuts/ShortcutSettings.vue'
  import { LanguageSettings } from '@/components/settings/components/Language'
  import SettingsNav from '@/components/settings/SettingsNav.vue'
  import { useSettingsStore } from '@/components/settings/store'
  import { onMounted, onUnmounted, watch, toRef, ref } from 'vue'

  interface Props {
    section?: string
  }

  const props = defineProps<Props>()
  const settingsStore = useSettingsStore()

  // 滚动容器引用
  const settingsMainRef = ref<HTMLElement>()
  const isScrolling = ref(false)
  let scrollTimeout: NodeJS.Timeout | null = null

  // 设置模块列表
  const settingSections = ['theme', 'ai', 'shortcuts', 'language']

  // 组件挂载时设置设置页面为打开状态并初始化设置
  onMounted(async () => {
    settingsStore.openSettings()
    if (props.section) {
      settingsStore.setActiveSection(props.section)
    }
    // 初始化所有设置
    await settingsStore.initializeSettings()

    // 添加滚动监听
    setupScrollListener()

    // 如果有指定的section，滚动到对应位置
    if (props.section) {
      setTimeout(() => {
        handleNavigationChange(props.section!)
      }, 100) // 等待DOM渲染完成
    }
  })

  // 组件卸载时移除滚动监听和清理定时器
  onUnmounted(() => {
    if (settingsMainRef.value) {
      settingsMainRef.value.removeEventListener('scroll', handleScroll)
    }
    if (scrollTimeout) {
      clearTimeout(scrollTimeout)
    }
  })

  // 外部传入的 section 变化时同步到 store
  const sectionRef = toRef(props, 'section')
  watch(sectionRef, newVal => {
    if (newVal) settingsStore.setActiveSection(newVal)
  })

  // 设置滚动监听
  const setupScrollListener = () => {
    if (settingsMainRef.value) {
      settingsMainRef.value.addEventListener('scroll', handleScroll, { passive: true })
    }
  }

  // 处理滚动事件，自动更新活动导航项（使用防抖优化性能）
  const handleScroll = () => {
    if (isScrolling.value || !settingsMainRef.value) return

    // 清除之前的定时器
    if (scrollTimeout) {
      clearTimeout(scrollTimeout)
    }

    // 使用防抖，避免频繁更新
    scrollTimeout = setTimeout(() => {
      if (!settingsMainRef.value) return

      const containerHeight = settingsMainRef.value.clientHeight

      // 找到当前可见的设置模块
      let activeSection = 'theme' // 默认值

      for (const section of settingSections) {
        const element = document.getElementById(`settings-${section}`)
        if (element) {
          const rect = element.getBoundingClientRect()
          const containerRect = settingsMainRef.value.getBoundingClientRect()

          // 如果模块的顶部在视口的上半部分，则认为它是活动的
          if (rect.top - containerRect.top <= containerHeight / 3) {
            activeSection = section
          }
        }
      }

      // 只有当活动模块真正改变时才更新
      if (activeSection !== settingsStore.activeSection) {
        settingsStore.setActiveSection(activeSection)
      }
    }, 100) // 100ms 防抖
  }

  // 处理导航项切换 - 滚动到对应模块
  const handleNavigationChange = (section: string) => {
    isScrolling.value = true
    settingsStore.setActiveSection(section)

    // 滚动到对应的设置模块
    const targetElement = document.getElementById(`settings-${section}`)
    if (targetElement && settingsMainRef.value) {
      const containerRect = settingsMainRef.value.getBoundingClientRect()
      const targetRect = targetElement.getBoundingClientRect()
      const scrollTop = settingsMainRef.value.scrollTop + targetRect.top - containerRect.top - 20

      settingsMainRef.value.scrollTo({
        top: scrollTop,
        behavior: 'smooth',
      })

      // 滚动完成后重新启用滚动监听
      setTimeout(() => {
        isScrolling.value = false
      }, 500)
    } else {
      isScrolling.value = false
    }
  }
</script>

<template>
  <div class="settings-view">
    <!-- 设置页面主体 -->
    <div class="settings-content">
      <!-- 左侧导航 -->
      <div class="settings-sidebar">
        <SettingsNav :activeSection="settingsStore.activeSection" @change="handleNavigationChange" />
      </div>

      <!-- 右侧内容区域 -->
      <div ref="settingsMainRef" class="settings-main">
        <div class="settings-panel">
          <!-- 所有设置模块整合到一个连续的滚动页面 -->
          <div id="settings-theme" class="settings-section">
            <ThemeSettings />
          </div>

          <div id="settings-ai" class="settings-section">
            <AISettings />
          </div>

          <div id="settings-shortcuts" class="settings-section">
            <ShortcutSettings />
          </div>

          <div id="settings-language" class="settings-section">
            <LanguageSettings />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .settings-view {
    height: 100vh;
    background-color: var(--bg-200);
    overflow: hidden;
  }

  .settings-content {
    display: flex;
    height: 100%;
  }

  .settings-sidebar {
    width: 220px;
    background-color: var(--bg-300);
    border-right: 1px solid var(--border-300);
    overflow-y: auto;
    flex-shrink: 0;
  }

  .settings-main {
    flex: 1;
    overflow-y: auto;
    background-color: var(--bg-200);
    scroll-behavior: smooth;
  }

  .settings-panel {
    min-height: 100%;
    padding: 0;
  }

  .settings-section {
    scroll-margin-top: 20px;
    position: relative;
    background: var(--bg-200);
  }

  /* 模块间的简洁分割 */
  .settings-section:not(:last-child) {
    border-bottom: 1px solid var(--border-200);
  }
</style>
