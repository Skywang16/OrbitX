<template>
  <div class="shortcut-search-bar">
    <div class="search-input-group">
      <i class="icon-search"></i>
      <input v-model="filter.query" class="search-input" placeholder="搜索快捷键..." @input="handleSearch" />
      <button v-if="filter.query" class="btn-clear" @click="handleClear">
        <i class="icon-close"></i>
      </button>
    </div>

    <div class="search-filters">
      <div class="filter-group">
        <label class="filter-label">类别:</label>
        <div class="category-filters">
          <label class="checkbox-label">
            <input type="checkbox" value="Global" v-model="filter.categories" @change="handleSearch" />
            全局
          </label>
          <label class="checkbox-label">
            <input type="checkbox" value="Terminal" v-model="filter.categories" @change="handleSearch" />
            终端
          </label>
          <label class="checkbox-label">
            <input type="checkbox" value="Custom" v-model="filter.categories" @change="handleSearch" />
            自定义
          </label>
        </div>
      </div>

      <div class="filter-group">
        <label class="checkbox-label">
          <input type="checkbox" v-model="filter.conflictsOnly" @change="handleSearch" />
          只显示冲突
        </label>
      </div>

      <div class="filter-group">
        <label class="checkbox-label">
          <input type="checkbox" v-model="filter.errorsOnly" @change="handleSearch" />
          只显示错误
        </label>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { watch } from 'vue'
  import type { ShortcutSearchFilter } from './types'

  interface Props {
    filter: ShortcutSearchFilter
  }

  interface Emits {
    (e: 'update:filter', filter: ShortcutSearchFilter): void
    (e: 'search'): void
    (e: 'clear'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  // 方法
  const handleSearch = () => {
    emit('update:filter', props.filter)
    emit('search')
  }

  const handleClear = () => {
    const clearedFilter: ShortcutSearchFilter = {
      query: '',
      categories: [],
      conflictsOnly: false,
      errorsOnly: false,
    }
    emit('update:filter', clearedFilter)
    emit('clear')
  }

  // 监听过滤器变化
  watch(
    () => props.filter,
    () => {
      handleSearch()
    },
    { deep: true }
  )
</script>

<style scoped>
  .shortcut-search-bar {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 16px;
    background: var(--bg-secondary);
    border-radius: 6px;
    border: 1px solid var(--border);
  }

  .search-input-group {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-input-group .icon-search {
    position: absolute;
    left: 12px;
    color: var(--text-secondary);
    z-index: 1;
  }

  .search-input {
    width: 100%;
    padding: 8px 12px 8px 36px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 14px;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 2px var(--primary-bg);
  }

  .btn-clear {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
    border-radius: 3px;
  }

  .btn-clear:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .search-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 20px;
    align-items: center;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .filter-label {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .category-filters {
    display: flex;
    gap: 12px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 14px;
    color: var(--text-primary);
    cursor: pointer;
    user-select: none;
  }

  .checkbox-label input[type='checkbox'] {
    margin: 0;
    cursor: pointer;
  }

  @media (max-width: 768px) {
    .shortcut-search-bar {
      gap: 12px;
    }

    .search-filters {
      flex-direction: column;
      align-items: flex-start;
      gap: 12px;
    }

    .category-filters {
      flex-direction: column;
      gap: 8px;
    }
  }
</style>
