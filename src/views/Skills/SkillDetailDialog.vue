<script setup lang="ts">
  import type { SkillSummary } from '@/api/agent'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { XButton, XModal } from '@/ui'
  import { renderMarkdown } from '@/utils/markdown'
  import { invoke } from '@tauri-apps/api/core'
  import { appDataDir } from '@tauri-apps/api/path'
  import { mkdir, readTextFile, remove, writeTextFile } from '@tauri-apps/plugin-fs'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import { computed, nextTick, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'

  interface DiscoverSkill {
    name: string
    description: string
    content: string
    homepage: string
    authorName: string
    downloadCount: number
    tags: string[]
  }

  const props = defineProps<{
    discoverSkill: DiscoverSkill | null
    installedSkill: SkillSummary | null
  }>()

  const emit = defineEmits<{
    (e: 'close'): void
    (e: 'installed'): void
    (e: 'uninstalled'): void
  }>()

  const { t } = useI18n()
  const workspaceStore = useWorkspaceStore()

  const dialogContent = ref('')
  const dialogContentLoading = ref(false)
  const installing = ref(false)
  const uninstalling = ref(false)
  const scrollRef = ref<HTMLElement | null>(null)

  const isOpen = computed(() => !!props.discoverSkill || !!props.installedSkill)

  const currentName = computed(() => props.discoverSkill?.name || props.installedSkill?.name || '')
  const currentDesc = computed(() => props.discoverSkill?.description || props.installedSkill?.description || '')

  // ---- Fallback icon ----
  const COLORS = [
    '#6366f1',
    '#8b5cf6',
    '#ec4899',
    '#f43f5e',
    '#f97316',
    '#eab308',
    '#22c55e',
    '#14b8a6',
    '#06b6d4',
    '#3b82f6',
  ]
  const hash = (s: string) => {
    let h = 0
    for (let i = 0; i < s.length; i++) h = ((h << 5) - h + s.charCodeAt(i)) | 0
    return Math.abs(h)
  }
  const iconColor = computed(() => COLORS[hash(currentName.value) % COLORS.length])
  const iconInitial = computed(() => currentName.value.charAt(0).toUpperCase())

  // ---- Content loading ----
  const stripFrontmatter = (md: string): string => {
    const lines = md.split('\n')
    let inside = false
    let end = 0
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].trim() === '---') {
        if (!inside) {
          inside = true
          continue
        }
        end = i + 1
        break
      }
    }
    return end > 0 ? lines.slice(end).join('\n').trim() : md
  }

  watch(
    () => props.discoverSkill,
    skill => {
      if (skill) {
        dialogContent.value = skill.content || skill.description
        dialogContentLoading.value = false
      }
    }
  )

  watch(
    () => props.installedSkill,
    async skill => {
      if (skill) {
        dialogContent.value = ''
        dialogContentLoading.value = true
        try {
          dialogContent.value = stripFrontmatter(await readTextFile(`${skill.skillDir}/SKILL.md`))
        } catch (error) {
          console.warn(`Failed to load skill content from '${skill.skillDir}':`, error)
          dialogContent.value = '*Failed to load skill content.*'
        } finally {
          dialogContentLoading.value = false
        }
      }
    }
  )

  watch(dialogContent, () => {
    nextTick(() => {
      if (scrollRef.value) scrollRef.value.scrollTop = 0
    })
  })

  // ---- Actions ----
  const openExternal = (url: string) => {
    openUrl(url).catch(err => console.warn('Failed to open:', err))
  }

  const openSkillDir = () => {
    const dir = props.installedSkill?.skillDir
    if (dir) invoke('open_in_editor', { path: dir }).catch(err => console.warn('Failed to open in editor:', err))
  }

  const installSkill = async (target: 'global' | 'workspace') => {
    const skill = props.discoverSkill
    if (!skill) return
    installing.value = true
    try {
      const baseDir =
        target === 'global' ? `${await appDataDir()}skills` : `${workspaceStore.currentWorkspacePath}/.orbitx/skills`
      const dir = `${baseDir}/${skill.name}`
      await mkdir(dir, { recursive: true })
      const fm = `---\nname: ${skill.name}\ndescription: ${skill.description}\n---`
      const body = skill.content || `# ${skill.name}\n\n${skill.description}`
      await writeTextFile(`${dir}/SKILL.md`, `${fm}\n\n${body}`)
      emit('installed')
    } catch (error) {
      console.warn('Failed to install skill:', error)
    } finally {
      installing.value = false
    }
  }

  const uninstallSkill = async () => {
    const skill = props.installedSkill
    if (!skill) return
    uninstalling.value = true
    try {
      await remove(skill.skillDir, { recursive: true })
      emit('uninstalled')
      emit('close')
    } catch (error) {
      console.warn('Failed to uninstall skill:', error)
    } finally {
      uninstalling.value = false
    }
  }

  const handleClose = () => {
    dialogContent.value = ''
    emit('close')
  }
</script>

<template>
  <XModal
    :visible="isOpen"
    size="medium"
    :show-header="false"
    :show-footer="false"
    no-padding
    modal-class="skill-dialog"
    @close="handleClose"
  >
    <!-- Header -->
    <div class="dialog-header">
      <div class="dialog-icon" :style="{ background: iconColor + '18', color: iconColor }">
        <span class="dialog-icon-initial">{{ iconInitial }}</span>
      </div>
      <div class="dialog-header-text">
        <h2 class="dialog-title">{{ currentName }}</h2>
        <p class="dialog-subtitle">
          {{ currentDesc }}
          <span v-if="discoverSkill?.authorName" class="dialog-author">
            {{ t('skills_page.by_author', { author: discoverSkill.authorName }) }}
          </span>
        </p>
      </div>
    </div>

    <!-- Body -->
    <div ref="scrollRef" class="dialog-body">
      <div v-if="dialogContentLoading" class="dialog-loading">{{ t('common.loading') }}</div>
      <!-- eslint-disable-next-line vue/no-v-html -->
      <div v-else class="dialog-markdown" v-html="renderMarkdown(dialogContent)" />
    </div>

    <!-- Footer -->
    <div class="dialog-footer">
      <template v-if="discoverSkill">
        <div class="dialog-footer-left">
          <XButton variant="link" size="medium" @click="openExternal(discoverSkill.homepage)">
            {{ t('skills_page.homepage') }}
          </XButton>
        </div>
        <div class="dialog-footer-right">
          <XButton
            v-if="workspaceStore.currentWorkspacePath"
            variant="secondary"
            size="medium"
            :loading="installing"
            @click="installSkill('workspace')"
          >
            {{ t('skills_page.install_workspace') }}
          </XButton>
          <XButton variant="primary" size="medium" :loading="installing" @click="installSkill('global')">
            {{ t('skills_page.install_global') }}
          </XButton>
        </div>
      </template>
      <template v-if="installedSkill">
        <div class="dialog-footer-left">
          <XButton variant="danger" size="medium" :loading="uninstalling" @click="uninstallSkill">
            {{ t('skills_page.uninstall') }}
          </XButton>
        </div>
        <div class="dialog-footer-right">
          <XButton variant="secondary" size="medium" @click="openSkillDir">
            <template #icon>
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
              </svg>
            </template>
            {{ t('skills_page.open_folder') }}
          </XButton>
        </div>
      </template>
    </div>
  </XModal>
</template>

<style scoped>
  .dialog-header {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 24px 24px 16px;
    flex-shrink: 0;
  }

  .dialog-header-text {
    flex: 1;
    min-width: 0;
  }

  .dialog-icon {
    width: 48px;
    height: 48px;
    border-radius: var(--border-radius-xl);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .dialog-icon-initial {
    font-size: 22px;
    font-weight: 700;
    line-height: 1;
  }

  .dialog-title {
    font-size: 20px;
    font-weight: 600;
    color: var(--text-100);
    margin-bottom: 2px;
  }

  .dialog-subtitle {
    font-size: 13px;
    color: var(--text-300);
    line-height: 1.4;
  }

  .dialog-author {
    font-size: 12px;
    color: var(--text-400);
    margin-left: 4px;
  }

  /* Body — only this area scrolls */
  .dialog-body {
    flex: 1;
    overflow-y: auto;
    padding: 0 24px 16px;
    min-height: 0;
  }

  .dialog-body::-webkit-scrollbar {
    width: 6px;
  }
  .dialog-body::-webkit-scrollbar-track {
    background: transparent;
  }
  .dialog-body::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: var(--border-radius-xs);
  }

  .dialog-loading {
    padding: 24px;
    text-align: center;
    color: var(--text-500);
    font-size: 13px;
  }

  .dialog-markdown {
    font-size: 14px;
    line-height: 1.7;
    color: var(--text-200);
    background: var(--bg-200);
    border-radius: var(--border-radius-xl);
    padding: 20px;
  }

  .dialog-markdown :deep(h1),
  .dialog-markdown :deep(h2),
  .dialog-markdown :deep(h3) {
    color: var(--text-100);
    margin-top: 1.2em;
    margin-bottom: 0.5em;
  }

  .dialog-markdown :deep(h1) {
    font-size: 18px;
  }
  .dialog-markdown :deep(h2) {
    font-size: 16px;
  }
  .dialog-markdown :deep(h3) {
    font-size: 14px;
  }
  .dialog-markdown :deep(p) {
    margin-bottom: 0.8em;
  }

  .dialog-markdown :deep(code) {
    font-family: var(--font-family-mono);
    font-size: 12px;
    background: var(--bg-300);
    padding: 2px 5px;
    border-radius: var(--border-radius-sm);
  }

  .dialog-markdown :deep(pre) {
    background: var(--bg-300);
    border-radius: var(--border-radius-lg);
    padding: 12px;
    overflow-x: auto;
    margin-bottom: 1em;
  }

  .dialog-markdown :deep(pre code) {
    background: none;
    padding: 0;
  }

  .dialog-markdown :deep(ul),
  .dialog-markdown :deep(ol) {
    padding-left: 1.5em;
    margin-bottom: 0.8em;
  }

  .dialog-markdown :deep(a) {
    color: var(--color-primary);
    text-decoration: none;
  }
  .dialog-markdown :deep(a:hover) {
    text-decoration: underline;
  }

  .dialog-markdown :deep(table) {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 1em;
    font-size: 13px;
  }

  .dialog-markdown :deep(th),
  .dialog-markdown :deep(td) {
    border: 1px solid var(--border-200);
    padding: 6px 10px;
    text-align: left;
  }

  .dialog-markdown :deep(th) {
    background: var(--bg-300);
    font-weight: 600;
  }

  /* Footer */
  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 24px;
    flex-shrink: 0;
  }

  .dialog-footer-left,
  .dialog-footer-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>

<!-- XModal Teleports to body — scoped cannot reach it -->
<style>
  .skill-dialog.modal-container {
    max-width: 640px;
  }

  .skill-dialog > .modal-body {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
</style>
