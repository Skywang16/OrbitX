<script setup lang="ts">
  import { onMounted, ref, watch } from 'vue'
  import { ai } from '@/api/ai'
  import { debounce } from 'lodash-es'

  // 用户前置提示词
  const userPrefixPrompt = ref<string>('')
  const isLoadingPrefix = ref(false)

  // 加载用户前置提示词
  const loadUserPrefixPrompt = async () => {
    isLoadingPrefix.value = true
    try {
      const prompt = await ai.getUserPrefixPrompt()
      userPrefixPrompt.value = prompt || ''
    } catch (error) {
      // 加载用户前置提示词失败
    } finally {
      isLoadingPrefix.value = false
    }
  }

  // 自动保存用户前置提示词
  const saveUserPrefixPrompt = async (value: string) => {
    try {
      const promptToSave = value.trim() || null
      await ai.setUserPrefixPrompt(promptToSave)
    } catch (error) {
      // 保存用户前置提示词失败
    }
  }

  // 使用lodash防抖监听输入变化，自动保存
  const debouncedSave = debounce((newValue: string) => {
    saveUserPrefixPrompt(newValue)
  }, 500)

  watch(userPrefixPrompt, debouncedSave)

  // 组件挂载时加载前置提示词
  onMounted(() => {
    loadUserPrefixPrompt()
  })
</script>

<template>
  <div class="ai-feature-settings">
    <!-- 用户前置提示词设置 -->
    <div class="prefix-prompt-section">
      <h3 class="section-title">用户前置提示词</h3>
      <p class="section-description">设置一个通用的前置提示词，它会自动添加到所有AI请求的前面</p>

      <textarea
        v-model="userPrefixPrompt"
        class="prompt-textarea"
        placeholder="在这里输入你的前置提示词，例如：请用中文回答所有问题..."
        rows="4"
        :disabled="isLoadingPrefix"
      ></textarea>
    </div>
  </div>
</template>

<style scoped>
  .ai-feature-settings {
    width: 100%;
  }

  .prefix-prompt-section {
    margin-bottom: var(--spacing-lg);
  }

  .section-title {
    font-size: var(--font-size-md);
    font-weight: 600;
    color: var(--text-200);
    margin: 0 0 var(--spacing-xs) 0;
  }

  .section-description {
    font-size: var(--font-size-sm);
    color: var(--text-400);
    margin: 0 0 var(--spacing-md) 0;
    line-height: 1.5;
  }

  .prompt-textarea {
    width: 100%;
    min-height: 100px;
    padding: var(--spacing-sm);
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius);
    background-color: var(--bg-400);
    color: var(--text-200);
    font-size: var(--font-size-sm);
    font-family: inherit;
    line-height: 1.5;
    resize: vertical;
    transition: border-color 0.2s ease;
  }

  .prompt-textarea:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .prompt-textarea:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .prompt-textarea::placeholder {
    color: var(--text-400);
  }

  .prompt-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--spacing-sm);
  }
</style>
