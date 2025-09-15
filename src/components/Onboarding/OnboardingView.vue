<template>
  <div class="onboarding-container">
    <div class="drag-area" data-tauri-drag-region @mousedown="startDrag"></div>

    <div class="onboarding-content">
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

      <div class="step-content">
        <Transition :name="transitionName" mode="out-in">
          <component :is="currentStepComponent" :key="currentStep" ref="currentStepRef" @next="handleNext" />
        </Transition>
      </div>

      <div class="navigation-buttons">
        <x-button v-if="currentStep > 0" variant="secondary" size="medium" @click="handlePrevious">
          {{ t('onboarding.navigation.previous') }}
        </x-button>

        <div class="button-spacer" />

        <x-button v-if="isAIStep" variant="secondary" size="medium" @click="handleSkip">
          {{ t('onboarding.navigation.skip_temporarily') }}
        </x-button>

        <x-button v-if="currentStep < steps.length - 1" variant="primary" size="medium" @click="handleNext">
          {{ getNextButtonText() }}
        </x-button>

        <x-button v-else variant="primary" size="medium" @click="handleNext">
          {{ getFinishButtonText() }}
        </x-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import { getCurrentWindow } from '@tauri-apps/api/window'

  import LanguageStep from './steps/LanguageStep.vue'
  import ThemeStep from './steps/ThemeStep.vue'
  import AIStep from './steps/AIStep.vue'

  const { t } = useI18n()

  interface AIStepComponent {
    handleSaveConfig: () => Promise<boolean>
    handleSkip: () => void
  }

  const steps = ['language', 'ai', 'theme'] as const
  const currentStep = ref(0)
  const transitionName = ref('slide-right')
  const currentStepRef = ref()

  const stepComponents = {
    language: LanguageStep,
    ai: AIStep,
    theme: ThemeStep,
  }

  const currentStepComponent = computed(() => {
    return stepComponents[steps[currentStep.value]]
  })

  const isAIStep = computed(() => {
    return steps[currentStep.value] === 'ai'
  })

  const emit = defineEmits<{
    complete: []
  }>()

  const startDrag = async () => {
    await getCurrentWindow().startDragging()
  }

  const handleNext = async () => {
    if (steps[currentStep.value] === 'ai') {
      const aiStepRef = currentStepRef.value as AIStepComponent

      if (aiStepRef && typeof aiStepRef.handleSaveConfig === 'function') {
        const success = await aiStepRef.handleSaveConfig()
        if (!success) {
          return
        }
      }
    }

    if (currentStep.value < steps.length - 1) {
      transitionName.value = 'slide-right'
      currentStep.value++
    } else {
      emit('complete')
    }
  }

  const handleSkip = () => {
    if (steps[currentStep.value] === 'ai') {
      const aiStepRef = currentStepRef.value as AIStepComponent
      if (aiStepRef && typeof aiStepRef.handleSkip === 'function') {
        aiStepRef.handleSkip()
      }

      if (currentStep.value < steps.length - 1) {
        transitionName.value = 'slide-right'
        currentStep.value++
      } else {
        emit('complete')
      }
    }
  }

  const handlePrevious = () => {
    if (currentStep.value > 0) {
      transitionName.value = 'slide-left'
      currentStep.value--
    }
  }

  const getNextButtonText = () => {
    const currentStepName = steps[currentStep.value]
    switch (currentStepName) {
      case 'ai':
        return t('onboarding.navigation.config_save')
      default:
        return t('onboarding.navigation.next')
    }
  }

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
