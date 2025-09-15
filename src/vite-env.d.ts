/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>;
  export default component;
}

// 扩展 Window 接口用于开发环境的全局函数
declare global {
  interface Window {
    showOnboarding?: () => void;
    reloadShortcuts?: () => void;
  }
}
