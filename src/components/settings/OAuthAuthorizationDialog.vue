<script setup lang="ts">
  import { ref, watch } from 'vue'
  import { useOAuth } from '@/composables/useOAuth'
  import OAuthProviderSelector from './OAuthProviderSelector.vue'
  import Button from '@/ui/components/Button.vue'
  import { OAuthProvider, type OAuthConfig } from '@/types/oauth'

  interface Props {
    visible: boolean
    initialProvider?: OAuthProvider | null
  }

  interface Emits {
    (e: 'update:visible', value: boolean): void
    (e: 'success', config: OAuthConfig): void
    (e: 'cancel'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  const selectedProvider = ref<OAuthProvider | null>(props.initialProvider || null)
  const { isAuthenticating, error, startAuthorization, cancelAuthorization, reset } = useOAuth()
  const isMouseDownOnOverlay = ref(false)

  watch(
    () => props.visible,
    visible => {
      if (!visible) {
        reset()
        selectedProvider.value = props.initialProvider || null
      }
    }
  )

  async function handleAuthorize() {
    if (!selectedProvider.value) return

    const config = await startAuthorization(selectedProvider.value)
    if (config) {
      emit('success', config)
      emit('update:visible', false)
    }
  }

  async function handleCancel() {
    if (isAuthenticating.value) {
      await cancelAuthorization()
    }
    emit('cancel')
    emit('update:visible', false)
  }

  function handleOverlayMouseDown(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      isMouseDownOnOverlay.value = true
    }
  }

  function handleOverlayMouseUp(event: MouseEvent) {
    if (isMouseDownOnOverlay.value && event.target === event.currentTarget) {
      handleCancel()
    }
    isMouseDownOnOverlay.value = false
  }
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="oauth-dialog-overlay"
      @mousedown="handleOverlayMouseDown"
      @mouseup="handleOverlayMouseUp"
    >
      <div class="oauth-dialog">
        <div class="oauth-dialog-header">
          <h2 class="oauth-dialog-title">OAuth 授权</h2>
          <button class="oauth-dialog-close" @click="handleCancel" :disabled="isAuthenticating">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div class="oauth-dialog-body">
          <div v-if="!isAuthenticating" class="auth-step-select">
            <p class="step-description">选择你的订阅服务提供商</p>
            <OAuthProviderSelector v-model="selectedProvider" />
          </div>

          <div v-else class="auth-step-loading">
            <div class="settings-loading-spinner"></div>
            <p class="loading-text">正在授权...</p>
            <p class="loading-hint">请在浏览器中完成授权</p>
          </div>

          <div v-if="error" class="auth-error">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
            <span>{{ error }}</span>
          </div>
        </div>

        <div class="oauth-dialog-footer">
          <Button variant="secondary" size="small" @click="handleCancel" :disabled="isAuthenticating">取消</Button>
          <Button
            variant="primary"
            size="small"
            @click="handleAuthorize"
            :disabled="!selectedProvider || isAuthenticating"
            :loading="isAuthenticating"
          >
            开始授权
          </Button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
  .oauth-dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    padding: 16px;
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .oauth-dialog {
    background: var(--bg-200);
    border-radius: var(--border-radius);
    width: 90%;
    max-width: 500px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
    animation: slideUp 0.2s ease-out;
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .oauth-dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 20px 24px;
    border-bottom: 1px solid var(--border-200);
  }

  .oauth-dialog-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-100);
    margin: 0;
  }

  .oauth-dialog-close {
    background: none;
    border: none;
    padding: 4px;
    cursor: pointer;
    color: var(--text-300);
    transition: color 0.15s ease;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--border-radius-xs);
  }

  .oauth-dialog-close:hover:not(:disabled) {
    color: var(--text-100);
    background: var(--bg-400);
  }

  .oauth-dialog-close:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .oauth-dialog-body {
    flex: 1;
    overflow-y: auto;
    padding: 24px;
  }

  .auth-step-select {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .step-description {
    font-size: 13px;
    color: var(--text-300);
    margin: 0;
  }

  .auth-step-loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 20px;
    gap: 12px;
  }

  .loading-text {
    font-size: 15px;
    font-weight: 500;
    color: var(--text-200);
    margin: 0;
  }

  .loading-hint {
    font-size: 13px;
    color: var(--text-400);
    margin: 0;
  }

  .auth-error {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px;
    background: color-mix(in srgb, #ef4444 10%, transparent);
    color: #ef4444;
    border: 1px solid color-mix(in srgb, #ef4444 20%, transparent);
    border-radius: var(--border-radius-sm);
    font-size: 13px;
    margin-top: 16px;
  }

  .oauth-dialog-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 24px;
    border-top: 1px solid var(--border-200);
  }
</style>
