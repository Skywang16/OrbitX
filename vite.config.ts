import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'
import { defineConfig } from 'vite'

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST

// https://vitejs.dev/config/
export default defineConfig(() => ({
  plugins: [vue()],

  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },

  // 防止 Vite 清除 Rust 显示的错误
  clearScreen: false,
  server: {
    port: 1420,
    // Tauri 工作于固定端口，如果端口不可用则报错
    strictPort: true,
    // 如果设置了 host，Tauri 则会使用
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 告诉 Vite 忽略监听 `src-tauri` 目录
      ignored: ['**/src-tauri/**'],
    },
  },
  // 添加有关当前构建目标的额外前缀，使这些 CLI 设置的 Tauri 环境变量可以在客户端代码中访问
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    // Tauri 在 Windows 上使用 Chromium，在 macOS 和 Linux 上使用 WebKit
    // @ts-expect-error process is a nodejs global
    target: process.env.TAURI_ENV_PLATFORM == 'windows' ? 'chrome105' : 'safari13',
    // 在 debug 构建中不使用 minify
    // @ts-expect-error process is a nodejs global
    minify: !process.env.TAURI_ENV_DEBUG ? ('esbuild' as const) : false,
    // 在 debug 构建中生成 sourcemap
    // @ts-expect-error process is a nodejs global
    sourcemap: !!process.env.TAURI_ENV_DEBUG,

    // 代码分割和chunk优化
    rollupOptions: {
      output: {
        // 手动分割chunks
        manualChunks: {
          // Vue核心
          'vue-core': ['vue', '@vueuse/core'],
          // 状态管理
          pinia: ['pinia'],
          // 终端相关
          xterm: ['@xterm/xterm', '@xterm/addon-fit', '@xterm/addon-search', '@xterm/addon-web-links'],
          // Tauri相关
          tauri: [
            '@tauri-apps/api',
            '@tauri-apps/plugin-fs',
            '@tauri-apps/plugin-http',
            '@tauri-apps/plugin-opener',
            '@tauri-apps/plugin-process',
            '@tauri-apps/plugin-window-state',
          ],
          // AI相关
          ai: ['@eko-ai/eko'],
          // 工具库
          utils: ['lodash-es', 'dayjs', 'uuid', 'marked', 'strip-ansi'],
          // UI动画
          lottie: ['lottie-web'],
          // 数据验证
          validation: ['ajv'],
        },
        // 优化chunk文件名
        chunkFileNames: () => {
          return `js/[name]-[hash].js`
        },
        // 优化资源文件名
        assetFileNames: assetInfo => {
          const info = (assetInfo.names?.[0] || assetInfo.name || '').split('.')
          let extType = info[info.length - 1]
          if (/png|jpe?g|svg|gif|tiff|bmp|ico/i.test(extType)) {
            extType = 'img'
          } else if (/woff2?|eot|ttf|otf/i.test(extType)) {
            extType = 'fonts'
          }
          return `${extType}/[name]-[hash].[ext]`
        },
      },
    },

    // 提高chunk大小警告阈值（针对Tauri桌面应用）
    chunkSizeWarningLimit: 800,
  },
}))
