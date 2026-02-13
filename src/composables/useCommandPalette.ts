import { ref, computed } from 'vue'
import type { CommandItem } from '@/components/CommandPalette'

// Global command registry
const globalCommands = ref<CommandItem[]>([])

export function useCommandPalette() {
  // Register commands
  const registerCommands = (commands: CommandItem[]) => {
    commands.forEach(cmd => {
      const existingIndex = globalCommands.value.findIndex(c => c.id === cmd.id)
      if (existingIndex >= 0) {
        globalCommands.value[existingIndex] = cmd
      } else {
        globalCommands.value.push(cmd)
      }
    })
  }

  // Unregister commands
  const unregisterCommands = (commandIds: string[]) => {
    globalCommands.value = globalCommands.value.filter(cmd => !commandIds.includes(cmd.id))
  }

  // Clear all commands
  const clearCommands = () => {
    globalCommands.value = []
  }

  // Get all commands
  const commands = computed(() => globalCommands.value)

  return {
    commands,
    registerCommands,
    unregisterCommands,
    clearCommands,
  }
}
