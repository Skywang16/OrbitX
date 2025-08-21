<template>
  <div v-if="visible" class="search-overlay" @click="close">
    <div class="search-box" @click.stop>
      <input
        ref="input"
        v-model="query"
        type="text"
        placeholder="搜索终端内容..."
        class="search-input"
        @keydown="handleKeydown"
        @input="search"
      />
      <div v-if="results.length" class="search-results">
        <div
          v-for="(result, index) in results"
          :key="index"
          class="search-result"
          :class="{ active: index === activeIndex }"
          @click="select(result)"
        >
          {{ result }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { ref, nextTick, watch } from 'vue'

  interface Props {
    visible: boolean
  }

  interface Emits {
    (e: 'close'): void
    (e: 'search', query: string): void
    (e: 'select', result: string): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  const input = ref<HTMLInputElement>()
  const query = ref('')
  const results = ref<string[]>([])
  const activeIndex = ref(-1)

  watch(
    () => props.visible,
    async visible => {
      if (visible) {
        query.value = ''
        results.value = []
        activeIndex.value = -1
        await nextTick()
        input.value?.focus()
      }
    }
  )

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      close()
    } else if (e.key === 'ArrowDown') {
      e.preventDefault()
      activeIndex.value = Math.min(activeIndex.value + 1, results.value.length - 1)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      activeIndex.value = Math.max(activeIndex.value - 1, 0)
    } else if (e.key === 'Enter' && activeIndex.value >= 0) {
      select(results.value[activeIndex.value])
    }
  }

  const search = () => {
    emit('search', query.value)
  }

  const select = (result: string) => {
    emit('select', result)
    close()
  }

  const close = () => {
    emit('close')
  }

  defineExpose({
    setResults: (newResults: string[]) => {
      results.value = newResults
      activeIndex.value = newResults.length > 0 ? 0 : -1
    },
  })
</script>

<style scoped>
  .search-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    z-index: 9999;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 20vh;
  }

  .search-box {
    background: var(--bg-200);
    border: 1px solid var(--border-200);
    border-radius: 8px;
    width: 500px;
    max-width: 90vw;
    overflow: hidden;
  }

  .search-input {
    width: 100%;
    padding: 12px 16px;
    border: none;
    background: transparent;
    color: var(--text-100);
    font-size: 16px;
    outline: none;
  }

  .search-input::placeholder {
    color: var(--text-400);
  }

  .search-results {
    border-top: 1px solid var(--border-300);
    max-height: 300px;
    overflow-y: auto;
  }

  .search-result {
    padding: 8px 16px;
    cursor: pointer;
    color: var(--text-200);
    font-family: var(--font-mono);
    font-size: 14px;
  }

  .search-result:hover,
  .search-result.active {
    background: var(--bg-300);
  }
</style>
