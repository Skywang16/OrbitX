<script setup lang="ts">
  import { useWorkspaceStore } from '@/stores/workspace'
  import { computed, onBeforeUnmount, onMounted, ref, watch, nextTick } from 'vue'

  interface Props {
    threadId?: number | null
    scrollContainerId: string
  }

  const props = defineProps<Props>()
  const workspaceStore = useWorkspaceStore()
  const activeMessageId = ref<number | null>(null)
  const hoveredEntryId = ref<string | null>(null)
  const timelineRowsEl = ref<HTMLElement | null>(null)
  const timelineEntries = computed(() => {
    if (!props.threadId || props.threadId <= 0) {
      return []
    }
    return workspaceStore.getThreadView(props.threadId)?.timeline ?? []
  })
  const hovered = ref(false)
  let hideTimer: ReturnType<typeof setTimeout> | null = null

  /**
   * Show/hide logic:
   * - showCard: called when mouse enters dot-area (initial trigger, pointer-events always on)
   *             also called when container re-enters (mouse briefly left during hide delay)
   * - scheduleHide: called ONLY by container @mouseleave — fires once when mouse truly leaves
   *                 the container boundary, NOT when moving between children
   */
  const showCard = () => {
    if (hideTimer) {
      clearTimeout(hideTimer)
      hideTimer = null
    }
    hovered.value = true
  }

  const scheduleHide = () => {
    hideTimer = setTimeout(() => {
      hovered.value = false
      hoveredEntryId.value = null
    }, 150)
  }

  const onDotEnter = (entryId: string) => {
    showCard()
    hoveredEntryId.value = entryId
  }

  const getScrollContainer = () => document.getElementById(props.scrollContainerId)

  const updateActiveMessage = () => {
    const container = getScrollContainer()
    if (timelineEntries.value.length === 0) {
      activeMessageId.value = null
      return
    }
    if (!container) {
      activeMessageId.value = timelineEntries.value[0]?.messageId ?? null
      return
    }
    const threshold = container.scrollTop + container.clientHeight * 0.35
    let currentId = timelineEntries.value[0]?.messageId ?? null
    for (const entry of timelineEntries.value) {
      const el = container.querySelector<HTMLElement>(`[data-user-message-id="${entry.messageId}"]`)
      if (!el) continue
      if (el.offsetTop <= threshold) currentId = entry.messageId
      else break
    }
    activeMessageId.value = currentId
  }

  const handleJump = (messageId: number) => {
    const container = getScrollContainer()
    const target = container?.querySelector<HTMLElement>(`[data-user-message-id="${messageId}"]`)
    if (!container || !target) return
    container.scrollTo({ top: Math.max(0, target.offsetTop - container.clientHeight * 0.28), behavior: 'smooth' })
    activeMessageId.value = messageId
  }

  const onScroll = () => updateActiveMessage()

  const bindScrollListener = () => {
    const c = getScrollContainer()
    if (!c) return
    c.addEventListener('scroll', onScroll, { passive: true })
    updateActiveMessage()
  }

  const unbindScrollListener = () => getScrollContainer()?.removeEventListener('scroll', onScroll)

  watch(
    () => [props.scrollContainerId, props.threadId, timelineEntries.value.length],
    () => {
      unbindScrollListener()
      updateActiveMessage()
      requestAnimationFrame(bindScrollListener)
    },
    { immediate: true }
  )

  // Debounced scroll-into-view: wait until outer scrolling settles before syncing
  let syncTimer: ReturnType<typeof setTimeout> | null = null
  watch(activeMessageId, id => {
    if (syncTimer) {
      clearTimeout(syncTimer)
      syncTimer = null
    }
    if (id == null) return
    syncTimer = setTimeout(async () => {
      if (!timelineRowsEl.value) return
      await nextTick()
      const entry = timelineEntries.value.find(e => e.messageId === id)
      if (!entry) return
      const btn = timelineRowsEl.value.querySelector<HTMLElement>(`[data-entry-id="${entry.id}"]`)
      btn?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
    }, 150)
  })

  onMounted(() => requestAnimationFrame(bindScrollListener))
  onBeforeUnmount(() => unbindScrollListener())
</script>

<template>
  <!--
    Container owns all show/hide logic:
    - @mouseenter: re-shows if mouse comes back during hide delay
    - @mouseleave: the ONE place scheduleHide is called — fires only when mouse truly
                   leaves the container boundary, never when moving between children
    When hovered=false, container pointer-events:none so these don't fire.
    Initial trigger is via .row-dot-area (always pointer-events:auto).
  -->
  <div
    v-if="timelineEntries.length > 0"
    class="thread-execution-timeline"
    :class="{ hovered }"
    @mouseenter="showCard"
    @mouseleave="scheduleHide"
  >
    <div ref="timelineRowsEl" class="timeline-rows">
      <button
        v-for="entry in timelineEntries"
        :key="entry.id"
        type="button"
        class="timeline-row"
        :data-entry-id="entry.id"
        :class="{
          active: entry.messageId === activeMessageId,
          'row-hover': entry.id === hoveredEntryId,
        }"
        @mouseenter="hoveredEntryId = entry.id"
        @mouseleave="hoveredEntryId = null"
        @click="handleJump(entry.messageId)"
      >
        <span class="row-title">{{ entry.title }}</span>
        <!--
          dot-area is always pointer-events:auto so it triggers the card
          even before hovered=true (when container is still pointer-events:none).
        -->
        <span class="row-dot-area" @mouseenter="onDotEnter(entry.id)">
          <span class="row-dot" />
        </span>
      </button>
    </div>
  </div>
</template>

<style scoped>
  .thread-execution-timeline {
    position: absolute;
    top: 50%;
    right: 20px;
    transform: translateY(-50%);
    z-index: 20;
    border-radius: 14px;
    background: transparent;
    box-shadow: none;
    overflow: hidden;
    max-width: 260px;
    /*
     * pointer-events:none by default so the invisible title area never blocks
     * clicks on the message list. Becomes auto when card is open so the
     * container's @mouseleave can fire.
     */
    pointer-events: none;
    transition:
      background 0.15s ease,
      box-shadow 0.15s ease;
  }

  .thread-execution-timeline.hovered {
    background: color-mix(in srgb, var(--bg-100) 85%, transparent);
    backdrop-filter: blur(16px) saturate(160%);
    -webkit-backdrop-filter: blur(16px) saturate(160%);
    box-shadow:
      0 0 0 1px var(--border-200),
      0 2px 12px var(--border-200);
    /* Enable pointer events on container so @mouseleave fires correctly */
    pointer-events: auto;
  }

  /* gradient fade masks */
  .thread-execution-timeline::before,
  .thread-execution-timeline::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    height: 32px;
    pointer-events: none;
    z-index: 3;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  .thread-execution-timeline::before {
    top: 0;
    background: linear-gradient(to bottom, color-mix(in srgb, var(--bg-100) 90%, transparent), transparent);
  }

  .thread-execution-timeline::after {
    bottom: 0;
    background: linear-gradient(to top, color-mix(in srgb, var(--bg-100) 90%, transparent), transparent);
  }

  .thread-execution-timeline.hovered::before,
  .thread-execution-timeline.hovered::after {
    opacity: 1;
  }

  .timeline-rows {
    display: flex;
    flex-direction: column;
    /* gap:0 + row padding so dot-areas are vertically contiguous, no dead zones */
    gap: 0;
    padding: 6px 12px;
    max-height: min(400px, 60vh);
    overflow-y: auto;
    scrollbar-width: none;
  }

  .timeline-rows::-webkit-scrollbar {
    display: none;
  }

  .timeline-row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    border: none;
    background: transparent;
    /* padding fills the visual gap between rows so row hover has no dead zones */
    padding: 5px 0;
    pointer-events: none;
    cursor: default;
  }

  /* when card is open, rows become interactive for hover tracking and clicks */
  .thread-execution-timeline.hovered .timeline-row {
    pointer-events: auto;
    cursor: pointer;
  }

  .row-title {
    flex: 1;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-500);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-align: right;
    opacity: 0;
    pointer-events: none;
    transition:
      opacity 0.15s ease,
      color 0.12s ease;
  }

  .thread-execution-timeline.hovered .row-title {
    opacity: 1;
  }

  .timeline-row.row-hover .row-title {
    color: var(--text-300);
  }

  .timeline-row.active .row-title {
    color: var(--color-primary);
    font-weight: 500;
  }

  .timeline-row.active.row-hover .row-title {
    color: var(--color-primary);
  }

  /* always-interactive area covering the dot column for each row */
  .row-dot-area {
    flex-shrink: 0;
    /* stretch fills the full row height including its padding — no vertical gaps */
    align-self: stretch;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    width: 26px;
    pointer-events: auto;
    cursor: pointer;
  }

  .row-dot {
    flex-shrink: 0;
    width: 14px;
    height: 3px;
    border-radius: 999px;
    background: var(--border-300);
    transform: scaleX(0.5);
    transform-origin: right center;
    pointer-events: none;
    transition:
      background 0.12s ease,
      transform 0.15s ease;
  }

  .timeline-row.row-hover .row-dot {
    background: var(--text-300);
    transform: scaleX(0.7);
  }

  .timeline-row.active .row-dot {
    background: var(--color-primary);
    transform: scaleX(1);
  }

  .timeline-row.active.row-hover .row-dot {
    background: var(--color-primary);
    transform: scaleX(1);
  }
</style>
