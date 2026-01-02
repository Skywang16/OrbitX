<script setup lang="ts">
  import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
  import { agentApi } from '@/api/agent'
  import { useToolConfirmationDialogStore } from '@/stores/toolConfirmationDialog'

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
      await agentApi.confirmTool(store.state.taskId, store.state.requestId, decision)
    } catch (error) {
      console.error('[ToolConfirmationDialog] confirm failed:', error)
      submitError.value = String(error)
      return
    } finally {
      store.submitting = false
    }

    store.close()
  }

  const handleAllow = async () => {
    await submit(store.remember ? 'allow_always' : 'allow_once')
  }

  const handleDeny = async () => {
    await submit('deny')
  }

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      handleDeny()
    }
  }

  onMounted(() => {
    document.addEventListener('keydown', handleKeydown)
  })

  onBeforeUnmount(() => {
    document.removeEventListener('keydown', handleKeydown)
  })
</script>

<template>
  <transition name="drawer">
    <div v-if="store.visible && store.state" class="tool-confirm-drawer">
      <div class="left">
        <div class="icon" :title="store.state.toolName" aria-hidden="true">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none">
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
          <div class="title">需要确认执行操作</div>
          <div class="summary" :title="store.state.summary">{{ store.state.summary }}</div>
        </div>
      </div>

      <div class="right">
        <label class="remember">
          <input v-model="store.remember" type="checkbox" :disabled="store.submitting" />
          <span>记住</span>
        </label>
        <button class="btn btn-ghost" @click="handleDeny" :disabled="store.submitting">拒绝</button>
        <button class="btn btn-primary" @click="handleAllow" :disabled="store.submitting">允许</button>
      </div>

      <div v-if="submitError" class="error" :title="submitError">{{ submitError }}</div>
    </div>
  </transition>
</template>

<style scoped>
  .tool-confirm-drawer {
    margin: 0 12px 8px;
    padding: 8px 10px;
    border-radius: 10px;
    border: 1px solid var(--border-200);
    background: var(--bg-50);
    box-shadow: 0 10px 26px rgba(0, 0, 0, 0.12);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .left {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .icon {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    border: 1px solid var(--border-200);
    background: var(--bg-100);
    color: var(--text-200);
    flex: 0 0 auto;
  }

  .info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    font-size: 12px;
    color: var(--text-100);
    line-height: 1.2;
  }

  .summary {
    font-size: 12px;
    color: var(--text-200);
    max-width: 720px;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    line-height: 1.3;
  }

  .remember {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-300);
    font-size: 12px;
    user-select: none;
    padding: 0 4px;
  }

  .right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 0 0 auto;
  }

  .error {
    font-size: 12px;
    color: var(--color-error);
    background: color-mix(in srgb, var(--color-error) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-error) 30%, transparent);
    border-radius: 10px;
    padding: 6px 10px;
    max-width: 520px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .btn {
    padding: 7px 10px;
    border-radius: 10px;
    font-size: 12px;
    border: 1px solid var(--border-200);
    cursor: pointer;
  }

  .btn:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .btn-ghost {
    background: transparent;
    color: var(--text-200);
  }

  .btn-primary {
    background: var(--color-primary-alpha);
    border-color: var(--color-primary);
    color: var(--text-100);
  }

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
