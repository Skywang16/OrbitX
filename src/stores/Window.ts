import { defineStore } from 'pinia'
import { windowApi } from '@/api/window'

export const useWindowStore = defineStore('window', {
  state: () => ({
    alwaysOnTop: false as boolean,
  }),
  actions: {
    setAlwaysOnTop(value: boolean) {
      this.alwaysOnTop = value
    },
    async initFromSystem() {
      try {
        const state = await windowApi.getWindowState()
        this.alwaysOnTop = !!state.alwaysOnTop
      } catch (e) {
        // ignore init failure, keep default false
      }
    },
  },
})
