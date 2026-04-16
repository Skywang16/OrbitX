<script setup lang="ts">
  import { agentApi } from '@/api/agent'
  import { useToolConfirmationDialogStore } from '@/stores/toolConfirmationDialog'
  import { onBeforeUnmount, onMounted, ref, watch } from 'vue'

  const store = useToolConfirmationDialogStore()
  const submitError = ref<string | null>(null)

  watch(
    () => store.visible,
    visible => {
      if (visible) submitError.value = null
    }
  )

  const submit = async (decision: 'allow_once' | 'allow_always' | 'deny') => {
    if (store.submitting || !store.state) return
    store.submitting = true
    submitError.value = null

    try {
      await agentApi.confirmTool(store.state.requestId, decision)
    } catch (error) {
      submitError.value = error instanceof Error ? error.message : String(error)
      return
    } finally {
      store.submitting = false
    }

    store.close()
  }

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') submit('deny')
  }

  onMounted(() => document.addEventListener('keydown', handleKeydown))
  onBeforeUnmount(() => document.removeEventListener('keydown', handleKeydown))
</script>

<template>
  <transition name="drawer">
    <div v-if="store.visible && store.state" class="tool-confirm-drawer">
      <div class="inner">
        <!-- Left: icon + text -->
        <div class="left">
          <div class="icon" aria-hidden="true">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path
                d="M14.7 6.3a1 1 0 0 0-1.4 0l-7 7a1 1 0 0 0 0 1.4l3 3a1 1 0 0 0 1.4 0l7-7a1 1 0 0 0 0-1.4l-3-3Z"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linejoin="round"
              />
              <path
                d="M7 17l-1 3 3-1"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </div>
          <div class="info">
            <span class="label">{{ store.state.toolName }}</span>
            <span class="summary" :title="store.state.summary">{{ store.state.summary }}</span>
          </div>
        </div>

        <!-- Right: action buttons -->
        <div class="actions">
          <button class="btn btn-deny" @click="submit('deny')" :disabled="store.submitting">Deny</button>
          <button class="btn btn-once" @click="submit('allow_once')" :disabled="store.submitting">Allow Once</button>
          <button
            class="btn btn-always"
            @click="submit('allow_always')"
            :disabled="store.submitting"
            title="Save to .orbitx/settings.local.json"
          >
            Allow in Workspace
          </button>
        </div>
      </div>

      <div v-if="submitError" class="error" :title="submitError">{{ submitError }}</div>
    </div>
  </transition>
</template>

<style scoped>
  /* ── Wrapper: same width & centering as ChatInput ── */
  .tool-confirm-drawer {
    width: 75%;
    margin: 0 auto 8px;
    padding: 8px 12px;
    border-radius: var(--border-radius-xl);
    border: 1px solid var(--border-200);
    background: var(--bg-50);
    box-shadow: var(--shadow-lg);
  }

  .inner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-height: 32px;
  }

  /* ── Left ── */
  .left {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
  }

  .icon {
    flex: 0 0 auto;
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--border-radius-lg);
    border: 1px solid var(--border-200);
    background: var(--bg-100);
    color: var(--text-300);
  }

  .info {
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: 6px;
    overflow: hidden;
  }

  .label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-100);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .summary {
    font-size: 12px;
    color: var(--text-300);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Actions ── */
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 0 0 auto;
  }

  .btn {
    padding: 5px 10px;
    border-radius: var(--border-radius-lg);
    font-size: 12px;
    border: 1px solid var(--border-200);
    cursor: pointer;
    white-space: nowrap;
    transition:
      background 0.12s ease,
      border-color 0.12s ease,
      opacity 0.12s ease;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Deny */
  .btn-deny {
    background: transparent;
    color: var(--text-300);
  }

  .btn-deny:hover:not(:disabled) {
    background: var(--bg-200);
    color: var(--text-100);
  }

  /* Allow Once */
  .btn-once {
    background: var(--bg-100);
    color: var(--text-200);
    border-color: var(--border-300);
  }

  .btn-once:hover:not(:disabled) {
    background: var(--bg-200);
    color: var(--text-100);
  }

  /* Allow in Workspace — primary */
  .btn-always {
    background: var(--text-100);
    border-color: var(--text-100);
    color: var(--bg-100);
  }

  .btn-always:hover:not(:disabled) {
    opacity: 0.85;
  }

  /* ── Error ── */
  .error {
    margin-top: 6px;
    font-size: 12px;
    color: var(--color-error);
    background: color-mix(in srgb, var(--color-error) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-error) 30%, transparent);
    border-radius: var(--border-radius-lg);
    padding: 5px 10px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ── Slide-up animation ── */
  .drawer-enter-active,
  .drawer-leave-active {
    transition:
      transform 140ms ease,
      opacity 140ms ease;
  }

  .drawer-enter-from,
  .drawer-leave-to {
    transform: translateY(8px);
    opacity: 0;
  }
</style>
