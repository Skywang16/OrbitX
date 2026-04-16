<script setup lang="ts">
  import { agentApi, type SkillSummary } from '@/api/agent'
  import { useWorkspaceStore } from '@/stores/workspace'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { invoke } from '@tauri-apps/api/core'
  import { computed, onMounted, ref, watch } from 'vue'
  import { useI18n } from 'vue-i18n'
  import SkillCard from './SkillCard.vue'
  import SkillDetailDialog from './SkillDetailDialog.vue'

  const { t } = useI18n()
  const workspaceStore = useWorkspaceStore()

  interface DiscoverSkill {
    name: string
    description: string
    content: string
    homepage: string
    authorName: string
    downloadCount: number
    tags: string[]
  }

  // ============ Types ============
  interface InstalledSkill extends SkillSummary {
    badge: string
  }

  // ============ State ============
  const installedSkills = ref<InstalledSkill[]>([])
  const installedLoading = ref(false)
  const discoverSkills = ref<DiscoverSkill[]>([])
  const discoverLoading = ref(false)
  const searchQuery = ref('')

  // Dialog
  const selectedDiscover = ref<DiscoverSkill | null>(null)
  const selectedInstalled = ref<InstalledSkill | null>(null)

  // ============ Computed ============
  const filteredDiscoverSkills = computed(() => {
    if (!searchQuery.value) return discoverSkills.value
    const q = searchQuery.value.toLowerCase()
    return discoverSkills.value.filter(
      s =>
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.tags.some(tag => tag.toLowerCase().includes(q))
    )
  })

  const wsDirName = (path: string) => path.split('/').filter(Boolean).pop() || path

  // ============ Data Loading ============
  const loadInstalledSkills = async () => {
    installedLoading.value = true
    try {
      const seen = new Map<string, InstalledSkill>()
      const globalLabel = t('skills_page.global')

      const addSkills = (skills: SkillSummary[], wsPath: string) => {
        const wsLabel = wsDirName(wsPath)
        for (const s of skills) {
          const key = `${s.source}:${s.skillDir}`
          if (!seen.has(key)) {
            seen.set(key, { ...s, badge: s.source === 'global' ? globalLabel : wsLabel })
          }
        }
      }

      // Load global + current workspace skills
      const currentPath = workspaceStore.currentWorkspacePath || ''
      addSkills(await agentApi.listSkills(currentPath), currentPath)

      // Load skills from all other workspaces
      for (const ws of workspaceStore.workspaces) {
        if (ws.path === currentPath) continue
        try {
          addSkills(await agentApi.listSkills(ws.path), ws.path)
        } catch (error) {
          console.warn(`Failed to load skills for workspace '${ws.path}', skipping:`, error)
        }
      }

      installedSkills.value = Array.from(seen.values())
    } catch (error) {
      console.warn('Failed to load installed skills:', error)
      installedSkills.value = []
    } finally {
      installedLoading.value = false
    }
  }

  const loadDiscoverSkills = async () => {
    discoverLoading.value = true
    try {
      const resp: any = await invoke('fetch_registry_skills')
      if (!resp || resp.code !== 200 || !resp.data) throw new Error(resp?.message || 'Fetch failed')
      const data: DiscoverSkill[] = JSON.parse(resp.data)
      discoverSkills.value = data.map(s => ({
        name: s.name,
        description: s.description || '',
        content: s.content || '',
        homepage: s.homepage || `https://skillregistry.io/skills/${s.name}`,
        authorName: s.authorName || '',
        downloadCount: s.downloadCount || 0,
        tags: s.tags || [],
      }))
    } catch (error) {
      console.warn('Failed to load discover skills:', error)
      discoverSkills.value = []
    } finally {
      discoverLoading.value = false
    }
  }

  // ============ Fallback Icon ============
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
  const getIcon = (name: string) => ({
    initial: name.charAt(0).toUpperCase(),
    color: COLORS[hash(name) % COLORS.length],
  })

  // ============ Dialog ============
  const closeDialog = () => {
    selectedDiscover.value = null
    selectedInstalled.value = null
  }

  // ============ Window Drag ============
  const startDrag = async () => {
    await getCurrentWindow().startDragging()
  }
  const handleDoubleClick = async () => {
    const win = getCurrentWindow()
    ;(await win.isMaximized()) ? await win.unmaximize() : await win.maximize()
  }

  // ============ Lifecycle ============
  onMounted(async () => {
    await loadInstalledSkills()
    await loadDiscoverSkills()
  })

  watch(
    () => workspaceStore.currentWorkspacePath,
    () => loadInstalledSkills()
  )
</script>

<template>
  <div class="skills-view">
    <!-- Header Tools -->
    <div class="header-tools" @mousedown="startDrag" @dblclick="handleDoubleClick">
      <div class="search-box" @mousedown.stop @dblclick.stop>
        <svg
          class="search-icon"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.3-4.3" />
        </svg>
        <input
          v-model="searchQuery"
          type="text"
          :placeholder="t('skills_page.search_placeholder')"
          class="search-input"
        />
      </div>
      <button class="new-skill-btn" @mousedown.stop @dblclick.stop>
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M12 5v14m7-7H5" />
        </svg>
        <span>{{ t('skills_page.new_skill') }}</span>
      </button>
    </div>

    <div class="skills-scroll-content">
      <div class="skills-container">
        <!-- Title -->
        <div class="title-section">
          <h1 class="page-title">{{ t('skills_page.title') }}</h1>
          <p class="page-description">{{ t('skills_page.description') }}</p>
        </div>

        <!-- Installed -->
        <section class="skills-section">
          <h2 class="section-title">{{ t('skills_page.installed') }}</h2>
          <div v-if="installedLoading" class="empty-state">{{ t('common.loading') }}</div>
          <div v-else-if="installedSkills.length === 0" class="empty-state">{{ t('skills_page.no_skills') }}</div>
          <div v-else class="skills-grid">
            <SkillCard
              v-for="skill in installedSkills"
              :key="skill.name"
              variant="installed"
              :name="skill.name"
              :description="skill.description"
              :color="getIcon(skill.name).color"
              :initial="getIcon(skill.name).initial"
              :source="skill.badge"
              @click="selectedInstalled = skill"
            />
          </div>
        </section>

        <!-- Discover -->
        <section class="skills-section">
          <h2 class="section-title">{{ t('skills_page.discover') }}</h2>
          <div v-if="discoverLoading" class="empty-state">{{ t('common.loading') }}</div>
          <div v-else-if="filteredDiscoverSkills.length === 0" class="empty-state">
            {{ t('skills_page.no_results') }}
          </div>
          <div v-else class="skills-grid">
            <SkillCard
              v-for="skill in filteredDiscoverSkills"
              :key="skill.name"
              variant="discover"
              :name="skill.name"
              :description="skill.description"
              :color="getIcon(skill.name).color"
              :initial="getIcon(skill.name).initial"
              @click="selectedDiscover = skill"
              @quick-install="selectedDiscover = skill"
            />
          </div>
        </section>
      </div>
    </div>

    <!-- Detail Dialog -->
    <SkillDetailDialog
      :discover-skill="selectedDiscover"
      :installed-skill="selectedInstalled"
      @close="closeDialog"
      @installed="loadInstalledSkills()"
      @uninstalled="loadInstalledSkills()"
    />
  </div>
</template>

<style scoped>
  .skills-view {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-100);
  }

  /* Header Tools */
  .header-tools {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    padding: 16px 32px 0;
  }

  .search-box {
    position: relative;
    width: 200px;
  }

  .search-icon {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    width: 14px;
    height: 14px;
    color: var(--text-400);
  }

  .search-input {
    width: 100%;
    padding: 6px 12px 6px 32px;
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    background: var(--bg-100);
    color: var(--text-100);
    font-size: 13px;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-alpha);
  }

  .new-skill-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border: 1px solid var(--border-200);
    background: var(--bg-100);
    color: var(--text-200);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    border-radius: var(--border-radius-lg);
  }

  .new-skill-btn:hover {
    background: var(--color-hover);
    border-color: var(--border-300);
  }

  .new-skill-btn svg {
    width: 14px;
    height: 14px;
  }

  /* Content */
  .skills-scroll-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px 48px 48px;
  }

  .skills-scroll-content::-webkit-scrollbar {
    width: 8px;
  }
  .skills-scroll-content::-webkit-scrollbar-track {
    background: transparent;
  }
  .skills-scroll-content::-webkit-scrollbar-thumb {
    background: var(--border-300);
    border-radius: var(--border-radius-sm);
  }

  .skills-container {
    max-width: 1200px;
    margin: 0 auto;
  }

  .title-section {
    margin-bottom: 40px;
  }

  .page-title {
    font-size: 32px;
    font-weight: 600;
    color: var(--text-100);
    margin-bottom: 8px;
    letter-spacing: -0.5px;
  }

  .page-description {
    font-size: 15px;
    color: var(--text-300);
  }

  .skills-section {
    margin-bottom: 40px;
  }

  .section-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-300);
    margin-bottom: 16px;
  }

  .skills-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
    gap: 16px;
  }

  .empty-state {
    padding: 24px;
    text-align: center;
    color: var(--text-500);
    font-size: 13px;
  }
</style>
