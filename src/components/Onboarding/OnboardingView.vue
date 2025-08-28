<template>
  <div class="onboarding-container">
    <!-- 拖动区域 -->
    <div class="drag-area" data-tauri-drag-region @mousedown="startDrag"></div>

    <div class="onboarding-content">
      <!-- 进度指示器 -->
      <div class="progress-indicator">
        <div
          v-for="(step, index) in steps"
          :key="`step-${index}-${step}`"
          class="progress-dot"
          :class="{ active: index === currentStep, completed: index < currentStep }"
          :data-step="step"
          :data-index="index"
        />
      </div>

      <!-- 步骤内容 -->
      <div class="step-content">
        <Transition :name="transitionName" mode="out-in">
          <component :is="currentStepComponent" :key="currentStep" ref="currentStepRef" @next="handleNext" />
        </Transition>
      </div>

      <!-- 导航按钮 -->
      <div class="navigation-buttons">
        <XButton v-if="currentStep > 0" variant="secondary" size="medium" @click="handlePrevious">
          {{ t('onboarding.navigation.previous') }}
        </XButton>

        <div class="button-spacer" />

        <!-- AI步骤时显示跳过按钮 -->
        <XButton v-if="isAIStep" variant="secondary" size="medium" @click="handleSkip">
          {{ t('onboarding.navigation.skip_temporarily') }}
        </XButton>

        <XButton v-if="currentStep < steps.length - 1" variant="primary" size="medium" @click="handleNext">
          {{ getNextButtonText() }}
        </XButton>

        <XButton v-else variant="primary" size="medium" @click="handleNext">
          {{ getFinishButtonText() }}
        </XButton>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { XButton } from '@/ui'
  import LanguageStep from './steps/LanguageStep.vue'
  import ThemeStep from './steps/ThemeStep.vue'
  import AIStep from './steps/AIStep.vue'

  const { t } = useI18n()

  // 定义 AI 步骤组件接口
  interface AIStepComponent {
    handleSaveConfig: () => Promise<boolean>
    handleSkip: () => void
  }

  // 定义步骤
  const steps = ['language', 'ai', 'theme'] as const
  const currentStep = ref(0)
  const transitionName = ref('slide-right')
  const currentStepRef = ref()

  // 步骤组件映射
  const stepComponents = {
    language: LanguageStep,
    ai: AIStep,
    theme: ThemeStep,
  }

  // 当前步骤组件
  const currentStepComponent = computed(() => {
    return stepComponents[steps[currentStep.value]]
  })

  // 是否是AI步骤
  const isAIStep = computed(() => {
    return steps[currentStep.value] === 'ai'
  })

  // 发出事件
  const emit = defineEmits<{
    complete: []
  }>()

  // 开始拖拽窗口
  const startDrag = async () => {
    await getCurrentWindow().startDragging()
  }

  // 处理下一步
  const handleNext = async () => {
    // 如果是AI步骤，需要特殊处理
    if (steps[currentStep.value] === 'ai') {
      const aiStepRef = currentStepRef.value as AIStepComponent

      if (aiStepRef && typeof aiStepRef.handleSaveConfig === 'function') {
        const success = await aiStepRef.handleSaveConfig()
        if (!success) {
          return // 如果保存失败，不继续
        }
      }
    }

    if (currentStep.value < steps.length - 1) {
      transitionName.value = 'slide-right'
      currentStep.value++
    } else {
      // 完成引导
      emit('complete')
    }
  }

  // 处理跳过
  const handleSkip = () => {
    if (steps[currentStep.value] === 'ai') {
      const aiStepRef = currentStepRef.value as AIStepComponent
      if (aiStepRef && typeof aiStepRef.handleSkip === 'function') {
        aiStepRef.handleSkip()
      }

      // 继续到下一步或完成
      if (currentStep.value < steps.length - 1) {
        transitionName.value = 'slide-right'
        currentStep.value++
      } else {
        emit('complete')
      }
    }
  }

  // 处理上一步
  const handlePrevious = () => {
    if (currentStep.value > 0) {
      transitionName.value = 'slide-left'
      currentStep.value--
    }
  }

  // 获取下一步按钮文本
  const getNextButtonText = () => {
    const currentStepName = steps[currentStep.value]
    switch (currentStepName) {
      case 'ai':
        return t('onboarding.navigation.save_config')
      default:
        return t('onboarding.navigation.next')
    }
  }

  // 获取完成按钮文本
  const getFinishButtonText = () => {
    return t('onboarding.navigation.finish')
  }
</script>

<style scoped>
  .onboarding-container {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: var(--bg-100);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  /* 拖动区域 */
  .drag-area {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 40px;
    z-index: 1001;
    cursor: default;
    -webkit-app-region: drag;
  }

  .onboarding-content {
    width: 90%;
    max-width: 600px;
    height: 100vh;
    display: flex;
    flex-direction: column;
    padding: 5vh 5vw;
    margin: 0 auto;
  }

  /* 进度指示器 */
  .progress-indicator {
    display: flex;
    justify-content: center;
    gap: 16px;
    margin-bottom: 8vh;
    flex-shrink: 0;
    align-items: center;
  }

  .progress-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--border-200, #666);
    transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
    position: relative;
    cursor: default;
  }

  .progress-dot.active {
    background: var(--color-primary, #007acc);
    transform: scale(1.3);
  }

  .progress-dot.completed {
    background: var(--color-primary, #007acc);
    transform: scale(1.1);
  }

  /* 移除过于激进的脉冲动画，使用更微妙的效果 */
  .progress-dot.active::after {
    content: '';
    position: absolute;
    top: -2px;
    left: -2px;
    right: -2px;
    bottom: -2px;
    border: 1px solid var(--color-primary);
    border-radius: 50%;
    opacity: 0.4;
    animation: subtle-pulse 3s ease-in-out infinite;
  }

  @keyframes subtle-pulse {
    0%,
    100% {
      transform: scale(1);
      opacity: 0.4;
    }
    50% {
      transform: scale(1.1);
      opacity: 0.2;
    }
  }

  /* 步骤内容 */
  .step-content {
    flex: 1;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    position: relative;
    overflow: hidden;
    min-height: 0;
    padding: 8vh 0 2vh 0;
  }

  /* 导航按钮 */
  .navigation-buttons {
    display: flex;
    align-items: center;
    gap: 16px;
    padding-top: 4vh;
    margin-top: auto;
    flex-shrink: 0;
  }

  .button-spacer {
    flex: 1;
  }

  /* 过渡动画 */
  .slide-right-enter-active,
  .slide-right-leave-active,
  .slide-left-enter-active,
  .slide-left-leave-active {
    transition: all 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94);
  }

  .slide-right-enter-from {
    transform: translateX(100%);
    opacity: 0;
  }

  .slide-right-leave-to {
    transform: translateX(-100%);
    opacity: 0;
  }

  .slide-left-enter-from {
    transform: translateX(-100%);
    opacity: 0;
  }

  .slide-left-leave-to {
    transform: translateX(100%);
    opacity: 0;
  }

  /* 响应式设计 */
  @media (max-width: 768px) {
    .onboarding-content {
      width: 95%;
      padding: 3vh 3vw;
    }

    .progress-indicator {
      margin-bottom: 6vh;
    }

    .navigation-buttons {
      padding-top: 3vh;
    }
  }

  @media (max-width: 480px) {
    .onboarding-content {
      width: 100%;
      padding: 2vh 4vw;
    }

    .progress-indicator {
      margin-bottom: 4vh;
    }

    .navigation-buttons {
      flex-wrap: wrap;
      justify-content: center;
      gap: 8px;
      padding-top: 2vh;
    }

    .button-spacer {
      display: none;
    }
  }

  @media (max-height: 600px) {
    .onboarding-content {
      padding: 2vh 5vw;
    }

    .progress-indicator {
      margin-bottom: 4vh;
    }

    .step-content {
      padding: 1vh 0;
    }

    .navigation-buttons {
      padding-top: 2vh;
    }
  }
</style>
