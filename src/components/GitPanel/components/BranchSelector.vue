<script setup lang="ts">
  import { computed, onMounted } from 'vue'
  import { useI18n } from 'vue-i18n'
  import type { BranchInfo } from '@/api/git/types'

  interface Props {
    branches: BranchInfo[]
    currentBranch: string | null
  }

  interface Emits {
    (e: 'load'): void
  }

  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()
  const { t } = useI18n()

  const localBranches = computed(() => props.branches.filter(b => !b.isRemote))
  const remoteBranches = computed(() => props.branches.filter(b => b.isRemote))

  const load = () => {
    emit('load')
  }

  onMounted(() => {
    load()
  })
</script>

<template>
  <div class="branches">
    <div class="section-header">
      <span class="section-header__title">{{ t('git.branches') }}</span>
      <button class="section-header__action" @click="load" :title="t('git.refresh')">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 12a9 9 0 1 1-3-6.7" />
          <path d="M21 3v6h-6" />
        </svg>
      </button>
    </div>

    <div class="branch-list">
      <details v-if="localBranches.length > 0" class="branch-group" open>
        <summary class="branch-group__summary">
          <span class="branch-group__caret">▸</span>
          <span class="branch-group__title">{{ t('git.local') }}</span>
          <span class="branch-group__count">{{ localBranches.length }}</span>
        </summary>
        <div class="branch-group__items">
          <div
            v-for="b in localBranches"
            :key="`local:${b.name}`"
            class="branch-item"
            :class="{ 'branch-item--current': b.isCurrent || b.name === currentBranch }"
          >
            <span class="branch-item__name">{{ b.name }}</span>
            <span v-if="b.ahead || b.behind" class="branch-item__sync">↑{{ b.ahead ?? 0 }} ↓{{ b.behind ?? 0 }}</span>
          </div>
        </div>
      </details>

      <details v-if="remoteBranches.length > 0" class="branch-group">
        <summary class="branch-group__summary">
          <span class="branch-group__caret">▸</span>
          <span class="branch-group__title">{{ t('git.remote') }}</span>
          <span class="branch-group__count">{{ remoteBranches.length }}</span>
        </summary>
        <div class="branch-group__items">
          <div v-for="b in remoteBranches" :key="`remote:${b.name}`" class="branch-item branch-item--remote">
            <span class="branch-item__name">{{ b.name }}</span>
          </div>
        </div>
      </details>
    </div>
  </div>
</template>

<style scoped>
  .branches {
    border-bottom: 1px solid var(--border-200);
    padding: 8px 10px;
    overflow: hidden;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 6px;
  }

  .section-header__title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-200);
  }

  .section-header__action {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    border: 1px solid var(--border-200);
    background: var(--bg-50);
    color: var(--text-200);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .section-header__action svg {
    width: 14px;
    height: 14px;
  }

  .branch-list {
    max-height: 140px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .branch-group {
    border: 1px solid var(--border-200);
    border-radius: 8px;
    background: var(--bg-50);
    overflow: hidden;
  }

  .branch-group__summary {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    cursor: pointer;
    list-style: none;
  }

  .branch-group__summary::-webkit-details-marker {
    display: none;
  }

  details[open] > .branch-group__summary .branch-group__caret {
    transform: rotate(90deg);
  }

  .branch-group__caret {
    width: 12px;
    color: var(--text-300);
    font-size: 12px;
    transition: transform 0.12s ease;
  }

  .branch-group__title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-200);
    flex: 1;
  }

  .branch-group__count {
    font-size: 11px;
    color: var(--text-300);
    padding: 0 6px;
    border: 1px solid var(--border-200);
    border-radius: 999px;
    background: var(--bg-100);
  }

  .branch-group__items {
    border-top: 1px solid var(--border-200);
    display: flex;
    flex-direction: column;
  }

  .branch-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px;
    color: var(--text-100);
    font-size: 12px;
  }

  .branch-item:hover {
    background: var(--bg-200);
  }

  .branch-item--current {
    background: var(--bg-200);
  }

  .branch-item--remote {
    color: var(--text-200);
  }

  .branch-item__name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .branch-item__sync {
    color: var(--text-300);
    font-size: 11px;
    flex: 0 0 auto;
  }
</style>
