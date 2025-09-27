<template>
  <div v-if="hasActiveTasks" class="task-section">
    <div class="task-tree-container">
      <div class="tree-header">
        <div class="header-content" @click="toggleCollapsed">
          <ChevronIcon :expanded="!isCollapsed" />
          <TreeIcon />
          <span class="tree-title">Tasks</span>
          <span class="task-count">{{ activeTasks.length }}</span>
        </div>
      </div>

      <transition name="tree-collapse">
        <div v-show="!isCollapsed" class="tree-content">
          <div v-if="activeTasks.length > 0" class="tree-container">
            <TaskItem
              v-for="task in activeTasks"
              :key="task.task_id"
              :task="task"
              :is-current="task.task_id === currentAgentTaskId"
              @click="handleSwitchTask(task.task_id)"
            />
          </div>
          <EmptyState v-else />
        </div>
      </transition>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { computed, ref, watch, onMounted } from 'vue'
  import { useAgentStateSyncAdapter } from '@/stores/agentStateSyncAdapter'
  import { useAIChatStore } from '@/components/AIChatSidebar/store'

  // Components
  import ChevronIcon from './icons/ChevronIcon.vue'
  import TreeIcon from './icons/TreeIcon.vue'
  import TaskItem from './TaskItem.vue'
  import EmptyState from './EmptyState.vue'

  const { initialize, activeAgentTasks, hasActiveAgentTasks, currentAgentTaskId, switchToAgentTask } =
    useAgentStateSyncAdapter()
  const aiChatStore = useAIChatStore()

  const isCollapsed = ref(false)

  // 计算属性
  const hasActiveTasks = computed(() => hasActiveAgentTasks.value)
  const activeTasks = computed(() => activeAgentTasks.value)

  // 处理任务切换
  const handleSwitchTask = async (taskId: string) => {
    try {
      await switchToAgentTask(taskId)
    } catch (error) {
      console.error('Failed to switch task:', error)
    }
  }

  // 切换折叠状态
  const toggleCollapsed = () => {
    isCollapsed.value = !isCollapsed.value
  }

  // 监听会话变化
  watch(
    () => aiChatStore.currentConversationId,
    async newConversationId => {
      if (newConversationId) {
        // 初始化适配器在 AIChatStore.initialize() 里已经调用过；这里无需重复
      }
    },
    { immediate: true }
  )

  // 生命周期钩子
  onMounted(async () => {
    await initialize()
  })
</script>
<style scoped>
  /* Section */
  .task-section {
    /* stick to bottom above input */
    margin-top: auto;
    margin-bottom: 0;
    padding: 8px 12px 12px 12px;
  }

  /* Container */
  .task-tree-container {
    background: var(--bg-100);
    border: 1px solid var(--border-100);
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  }

  /* Header */
  .tree-header {
    background: var(--bg-200);
    border-bottom: 1px solid var(--border-100);
  }

  .header-content {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    cursor: pointer;
    color: var(--text-200);
    user-select: none;
    transition: background-color 0.15s ease;
  }

  .header-content:hover {
    background: var(--bg-300);
  }

  .tree-title {
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.2px;
    text-transform: uppercase;
  }

  .task-count {
    margin-left: auto;
    font-size: 10px;
    color: var(--text-300);
    background: var(--bg-400);
    border-radius: 10px;
    padding: 1px 6px;
    min-width: 16px;
    text-align: center;
  }

  /* Collapse transition */
  .tree-collapse-enter-active,
  .tree-collapse-leave-active {
    transition: all 0.18s ease;
  }

  .tree-collapse-enter-from,
  .tree-collapse-leave-to {
    opacity: 0;
    transform: translateY(-2px);
  }

  .tree-content {
    /* height limit + scroll */
    max-height: 200px;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .tree-container {
    /* Optimized for performance with large task lists */
    contain: layout style paint;
  }
</style>
