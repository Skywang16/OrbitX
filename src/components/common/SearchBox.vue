<template>
  <div v-if="visible" class="search-box">
    <div class="search-input-wrapper">
      <input
        ref="input"
        v-model="query"
        type="text"
        :placeholder="t('search.placeholder')"
        class="search-input"
        @keydown="handleKeydown"
        @input="search"
      />
      <div class="search-controls">
        <span v-if="totalResults > 0" class="search-count">{{ currentIndex + 1 }}:{{ totalResults }}</span>
        <button
          class="search-btn toggle-btn"
          :class="{ active: caseSensitive }"
          @click="toggleCaseSensitive"
          :title="t('search.case_sensitive')"
        >
          Aa
        </button>
        <button
          class="search-btn toggle-btn"
          :class="{ active: wholeWord }"
          @click="toggleWholeWord"
          :title="t('search.whole_word')"
        >
          ab
        </button>
        <button class="search-btn" :disabled="totalResults === 0" @click="findPrevious" :title="t('search.previous')">
          ↑
        </button>
        <button class="search-btn" :disabled="totalResults === 0" @click="findNext" :title="t('search.next')">↓</button>
        <button class="search-btn close-btn" @click="close" :title="t('search.close')">×</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, nextTick, watch } from 'vue'
  import { useI18n } from 'vue-i18n'

  const { t } = useI18n()

  interface Props {
    visible: boolean
  }

  interface Emits {
    (e: 'close'): void
    (e: 'search', query: string, options: { caseSensitive: boolean; wholeWord: boolean }): void
    (e: 'find-next'): void
    (e: 'find-previous'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  const input = ref<HTMLInputElement>()
  const query = ref('')
  const totalResults = ref(0)
  const currentIndex = ref(-1)
  const caseSensitive = ref(false)
  const wholeWord = ref(false)

  watch(
    () => props.visible,
    async visible => {
      if (visible) {
        query.value = ''
        totalResults.value = 0
        currentIndex.value = -1
        await nextTick()
        input.value?.focus()
      }
    }
  )

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      close()
    } else if (e.key === 'Enter') {
      e.preventDefault()
      if (e.shiftKey) {
        findPrevious()
      } else {
        findNext()
      }
    } else if (e.key === 'F3') {
      e.preventDefault()
      if (e.shiftKey) {
        findPrevious()
      } else {
        findNext()
      }
    } else if (e.key === 'ArrowUp') {
      // 使用上方向键跳转到上一个匹配项（有结果时）
      if (totalResults.value > 0) {
        e.preventDefault()
        findPrevious()
      }
    } else if (e.key === 'ArrowDown') {
      // 使用下方向键跳转到下一个匹配项（有结果时）
      if (totalResults.value > 0) {
        e.preventDefault()
        findNext()
      }
    }
  }

  const search = () => {
    emit('search', query.value, {
      caseSensitive: caseSensitive.value,
      wholeWord: wholeWord.value,
    })
  }

  const findNext = () => {
    emit('find-next')
  }

  const findPrevious = () => {
    emit('find-previous')
  }

  const toggleCaseSensitive = () => {
    caseSensitive.value = !caseSensitive.value
    if (query.value) {
      search() // 重新搜索
    }
  }

  const toggleWholeWord = () => {
    wholeWord.value = !wholeWord.value
    if (query.value) {
      search() // 重新搜索
    }
  }

  const close = () => {
    emit('close')
  }

  defineExpose({
    setSearchState: (total: number, current: number) => {
      totalResults.value = total
      currentIndex.value = current
    },
  })
</script>

<style scoped>
  .search-box {
    position: absolute;
    top: 8px;
    right: 12px;
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-sm);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
    z-index: 1000;
    min-width: 280px;
  }

  .search-input-wrapper {
    display: flex;
    align-items: center;
  }

  .search-input {
    flex: 1;
    padding: 6px 8px;
    border: none;
    background: transparent;
    color: var(--text-100);
    font-size: 13px;
    outline: none;
    font-family: var(--font-mono);
  }

  .search-input::placeholder {
    color: var(--text-400);
  }

  .search-controls {
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 2px;
  }

  .search-count {
    color: var(--text-300);
    font-size: 11px;
    padding: 0 6px;
    font-family: var(--font-mono);
    white-space: nowrap;
    min-width: 30px;
    text-align: center;
  }

  .search-btn {
    background: transparent;
    border: none;
    color: var(--text-200);
    cursor: pointer;
    padding: 4px 6px;
    border-radius: var(--border-radius-xs);
    font-size: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
  }

  .search-btn:hover:not(:disabled) {
    background: var(--bg-300);
  }

  .search-btn:disabled {
    color: var(--text-500);
    cursor: not-allowed;
  }

  .toggle-btn {
    font-size: 10px;
    min-width: 24px;
    border: 1px solid var(--border-300);
    border-radius: var(--border-radius-xs);
    margin: 0 1px;
  }

  .toggle-btn.active {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
  }

  .close-btn {
    font-size: 14px;
    font-weight: bold;
  }
</style>
