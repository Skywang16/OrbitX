<script setup lang="ts">
  import { ref, computed, onMounted, onUnmounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { windowApi } from '@/api'
  import { handleErrorWithMessage } from '@/utils/errorHandler'
  import { useAIChatStore } from '@/components/AIChatSidebar'
  import { useTabManagerStore } from '@/stores/TabManager'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { createMessage } from '@/ui/composables/message-api'

  const { t } = useI18n()
  type ButtonGroup = { alwaysOnTop?: boolean }

  interface Props {
    controls?: ButtonGroup
  }

  const props = withDefaults(defineProps<Props>(), {
    controls: () => ({ alwaysOnTop: true }),
  })

  const aiChatStore = useAIChatStore()
  const tabManagerStore = useTabManagerStore()
  const isAlwaysOnTop = ref(false)
  const showSettingsPopover = ref(false)

  // 设置菜单项 - 使用计算属性确保响应式更新
  const settingsMenuItems = computed(() => [
    { label: t('ui.open_settings'), value: 'settings' },
    { label: t('ui.feedback'), value: 'feedback' },
  ])

  // 切换窗口置顶状态
  // 使用新的批量操作接口，提供更好的性能
  const toggleAlwaysOnTop = async () => {
    if (!props.controls.alwaysOnTop) return

    try {
      const newState = await windowApi.toggleAlwaysOnTop()
      isAlwaysOnTop.value = newState
    } catch (error) {
      handleErrorWithMessage(error, '切换窗口置顶失败')
      isAlwaysOnTop.value = !isAlwaysOnTop.value
    }
  }

  // 处理设置操作
  const handleSettingsAction = async (item: { label: string; value: string }) => {
    if (item.value === 'settings') {
      tabManagerStore.createSettingsTab()
    } else if (item.value === 'shortcuts') {
      tabManagerStore.createSettingsTab()
    } else if (item.value === 'feedback') {
      try {
        // 使用Tauri的opener插件在外部浏览器中打开GitHub Issues页面
        await openUrl('https://github.com/Skywang16/OrbitX/issues')
        // 显示成功提示
        // createMessage.success('已在浏览器中打开问题反馈页面')
      } catch (error) {
        handleErrorWithMessage(error, '无法打开问题反馈页面')
        createMessage.error('请手动访问：https://github.com/Skywang16/OrbitX/issues')
      }
    }
    showSettingsPopover.value = false
  }

  // 切换AI聊天侧边栏
  const toggleAIChat = () => {
    aiChatStore.toggleSidebar()
  }

  // 监听快捷键触发的窗口状态变化
  const handleShortcutToggle = (event: CustomEvent) => {
    if (event.detail?.action === 'toggle_window_pin') {
      // 快捷键触发时，直接切换状态
      isAlwaysOnTop.value = !isAlwaysOnTop.value
    }
  }

  onMounted(() => {
    // 监听快捷键事件
    window.addEventListener('shortcut-action', handleShortcutToggle as EventListener)
  })

  onUnmounted(() => {
    window.removeEventListener('shortcut-action', handleShortcutToggle as EventListener)
  })
</script>

<template>
  <div class="window-controls" data-tauri-drag-region="false">
    <!-- 按钮组 -->
    <div class="button-group">
      <!-- AI聊天按钮 -->
      <button
        class="control-btn ai-chat-btn"
        :class="{ active: aiChatStore.isVisible }"
        @click="toggleAIChat"
        title="AI助手"
      >
        <svg class="ai-chat-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
        </svg>
      </button>
      <!-- 设置按钮 -->
      <x-popover
        v-model="showSettingsPopover"
        placement="bottom-end"
        :menu-items="settingsMenuItems"
        @menu-item-click="handleSettingsAction"
      >
        <template #trigger>
          <button class="control-btn settings-btn" :title="t('ui.settings')">
            <svg class="settings-icon" viewBox="0 0 1024 1024" fill="currentColor">
              <path
                d="M449.194667 82.346667a128 128 0 0 1 125.610666 0l284.16 160a128 128 0 0 1 65.194667 111.530666v316.245334a128 128 0 0 1-65.194667 111.530666l-284.16 160a128 128 0 0 1-125.610666 0l-284.16-160a128 128 0 0 1-65.194667-111.530666V353.877333A128 128 0 0 1 165.034667 242.346667z m83.754666 74.410666a42.666667 42.666667 0 0 0-41.898666 0L206.933333 316.714667a42.666667 42.666667 0 0 0-21.76 37.162666v316.245334a42.666667 42.666667 0 0 0 21.76 37.162666l284.16 160a42.666667 42.666667 0 0 0 41.898667 0l284.16-160a42.666667 42.666667 0 0 0 21.76-37.162666V353.877333a42.666667 42.666667 0 0 0-21.76-37.162666zM512 341.333333a170.666667 170.666667 0 1 1 0 341.333334 170.666667 170.666667 0 0 1 0-341.333334z m0 85.333334a85.333333 85.333333 0 1 0 0 170.666666 85.333333 85.333333 0 0 0 0-170.666666z"
              />
            </svg>
          </button>
        </template>
      </x-popover>
      <!-- 置顶按钮 -->
      <button
        v-if="controls.alwaysOnTop"
        class="control-btn pin-btn"
        :class="{ active: isAlwaysOnTop }"
        @click="toggleAlwaysOnTop"
        :title="t('ui.pin')"
      >
        <svg class="pin-icon" viewBox="0 0 1024 1024" fill="currentColor">
          <path
            d="M706.081047 78.192364c2.699736 0 4.899522 0 9.399082 4.39957 3.999609 4.0996 4.39957 6.399375 4.39957 9.399083 0 3.099697-0.299971 5.299482-4.49956 9.599062-0.999902 1.099893-1.999805 1.899814-2.999707 2.499756-43.995704 30.197051-70.193145 79.592227-70.193146 132.187091v2.099795c0 123.787911 42.795821 244.576116 120.488234 340.166781 0.399961 0.499951 0.699932 0.899912 1.099893 1.399863 17.09833 21.497901 27.597305 44.395664 31.696904 69.393223h-566.944634c4.0996-24.997559 14.598574-47.895323 31.596914-69.293233 0.19998-0.299971 0.399961-0.599941 0.699932-0.799922 77.992384-96.290597 120.988185-217.278781 120.988185-340.766722v-2.299775c0-52.594864-26.297432-101.99004-70.293136-132.187091-0.999902-0.699932-2.099795-1.599844-3.199687-2.699737-3.999609-3.999609-4.29958-6.299385-4.29958-9.299091 0-3.099697 0.299971-5.299482 4.39957-9.499073 3.999609-3.999609 6.299385-4.29958 9.299092-4.29958h388.362074m0-63.993751H317.718973c-21.09794 0-39.396153 7.799238-54.694659 23.097745-15.298506 15.498486-22.997754 33.596719-22.997754 54.694659s7.799238 39.396153 23.097744 54.694658c3.899619 3.799629 7.899229 7.199297 12.098819 10.099014 26.297432 18.098233 42.49585 47.495362 42.49585 79.492237v2.299775C317.718973 347.966019 279.922664 453.955668 211.029392 538.947368c-0.299971 0.399961-0.599941 0.799922-0.999903 1.199883-31.796895 39.896104-47.595352 84.691729-47.595352 134.386877 0 10.498975 3.799629 19.598086 11.498877 27.297334 7.599258 7.699248 16.79836 11.498877 27.297335 11.498877h245.076066l46.095499 294.371253c1.999805 10.898936 8.49917 16.298408 19.398106 16.298408h0.599941c4.799531 0 8.999121-1.599844 12.398789-4.999512 3.499658-3.299678 5.699443-7.599258 6.399375-12.398789l31.096963-293.17137h260.374573c10.498975 0 19.598086-3.799629 27.297334-11.498877 7.699248-7.599258 11.498877-16.79836 11.498877-27.297334 0-49.895127-15.898447-94.690753-47.695342-134.586857-0.499951-0.599941-0.999902-1.299873-1.499853-1.899814-68.893272-84.691729-106.18963-190.681379-106.18963-299.770726v-2.099795c0-31.896885 16.098428-61.394004 42.39586-79.392247 4.29958-2.899717 8.29919-6.299385 12.198808-10.298994 15.398496-15.498486 23.097744-33.596719 23.097745-54.694659s-7.799238-39.296162-23.097745-54.694658c-15.398496-15.298506-33.496729-22.997754-54.594668-22.997755z"
          />
        </svg>
      </button>
    </div>
  </div>
</template>

<style scoped>
  .window-controls {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .button-group {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    height: var(--titlebar-element-height);
    background: var(--bg-400);
    border-radius: var(--border-radius-md);
    padding: 2px;
    border: 1px solid var(--border-300);
  }

  .control-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
    border-radius: var(--border-radius-sm);
    -webkit-app-region: no-drag;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background-color 0.2s ease;
  }

  .control-btn:hover {
    background: var(--color-hover);
  }

  .pin-btn.active {
    background: var(--color-primary);
  }

  .pin-btn.active .pin-icon {
    color: var(--text-100);
  }

  .pin-icon {
    height: 14px;
    width: 14px;
    color: var(--text-200);
    transition: color 0.2s ease;
  }

  .settings-icon {
    height: 14px;
    width: 14px;
    color: var(--text-200);
  }

  .ai-chat-btn.active {
    background: var(--color-primary);
  }

  .ai-chat-btn.active .ai-chat-icon {
    color: var(--text-100);
  }

  .ai-chat-icon {
    height: 14px;
    width: 14px;
    color: var(--text-200);
    transition: color 0.2s ease;
  }
</style>
