import { onMounted, onUnmounted } from 'vue'
import { useTerminalStore } from '@/stores/Terminal'

interface TerminalEventHandlers {
  onOutput: (data: string) => void
  onExit: (exitCode: number | null) => void
}

/**
 * A Vue composable for subscribing to terminal events in a lifecycle-aware manner.
 * It automatically registers the event listeners when the component is mounted
 * and unregisters them when the component is unmounted.
 *
 * @param terminalId The ID of the terminal to subscribe to.
 * @param handlers An object containing the callback functions for terminal events.
 */
export function useTerminalEvents(terminalId: string, handlers: TerminalEventHandlers) {
  const terminalStore = useTerminalStore()

  onMounted(() => {
    // Ensure terminalId is valid before registering
    if (terminalId) {
      terminalStore.registerTerminalCallbacks(terminalId, handlers)
    }
  })

  onUnmounted(() => {
    // Ensure terminalId is valid before unregistering
    if (terminalId) {
      terminalStore.unregisterTerminalCallbacks(terminalId, handlers)
    }
  })
}
