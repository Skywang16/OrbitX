<template>
  <div v-if="visible" class="confirm-overlay" @click="cancel">
    <div class="confirm-dialog" @click.stop>
      <div class="confirm-content">
        <h3>{{ title }}</h3>
        <p v-if="message">{{ message }}</p>
        <input
          v-if="needInput"
          ref="inputRef"
          v-model="inputValue"
          type="text"
          :placeholder="inputPlaceholder"
          class="confirm-input"
          @keydown="handleKeydown"
        />
      </div>
      <div class="confirm-actions">
        <button @click="cancel" class="btn btn-cancel">{{ $t('dialog.cancel') }}</button>
        <button @click="confirm" class="btn btn-confirm" :class="type">{{ $t('dialog.confirm') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, nextTick, watch } from 'vue'

  interface Props {
    visible: boolean
    title: string
    message?: string
    type?: 'primary' | 'danger'
    needInput?: boolean
    inputPlaceholder?: string
  }

  interface Emits {
    (e: 'confirm', value?: string): void
    (e: 'cancel'): void
  }

  const props = withDefaults(defineProps<Props>(), {
    type: 'primary',
    needInput: false,
    inputPlaceholder: '',
  })

  const emit = defineEmits<Emits>()

  const inputRef = ref<HTMLInputElement>()
  const inputValue = ref('')

  watch(
    () => props.visible,
    async visible => {
      if (visible) {
        inputValue.value = ''
        if (props.needInput) {
          await nextTick()
          inputRef.value?.focus()
        }
      }
    }
  )

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      confirm()
    } else if (e.key === 'Escape') {
      cancel()
    }
  }

  const confirm = () => {
    emit('confirm', props.needInput ? inputValue.value : undefined)
  }

  const cancel = () => {
    emit('cancel')
  }
</script>

<style scoped>
  .confirm-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 10000;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .confirm-dialog {
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: 8px;
    width: 400px;
    max-width: 90vw;
    overflow: hidden;
  }

  .confirm-content {
    padding: 20px;
  }

  .confirm-content h3 {
    margin: 0 0 12px 0;
    color: var(--text-100);
    font-size: 18px;
  }

  .confirm-content p {
    margin: 0 0 16px 0;
    color: var(--text-300);
    line-height: 1.4;
  }

  .confirm-input {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--border-300);
    border-radius: 4px;
    background: var(--bg-300);
    color: var(--text-100);
    outline: none;
  }

  .confirm-input:focus {
    border-color: var(--primary);
  }

  .confirm-actions {
    padding: 16px 20px;
    border-top: 1px solid var(--border-300);
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .btn {
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .btn-cancel {
    background: var(--bg-400);
    color: var(--text-300);
  }

  .btn-cancel:hover {
    background: var(--bg-500);
  }

  .btn-confirm {
    color: white;
  }

  .btn-confirm.primary {
    background: var(--primary);
  }

  .btn-confirm.danger {
    background: var(--error);
  }

  .btn-confirm:hover {
    opacity: 0.9;
  }
</style>
