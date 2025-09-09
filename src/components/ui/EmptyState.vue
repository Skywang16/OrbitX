<template>
  <div class="empty-state">
    <div class="orbit-logo">
      <svg width="160" height="160" viewBox="0 0 160 160" xmlns="http://www.w3.org/2000/svg">
        <!-- 背景光晕 -->
        <defs>
          <radialGradient id="centerGlow" cx="50%" cy="50%" r="50%">
            <stop offset="0%" style="stop-color: var(--text-200); stop-opacity: 0.15" />
            <stop offset="40%" style="stop-color: var(--text-300); stop-opacity: 0.08" />
            <stop offset="80%" style="stop-color: var(--text-400); stop-opacity: 0.03" />
            <stop offset="100%" style="stop-color: transparent; stop-opacity: 0" />
          </radialGradient>
          <filter id="glow">
            <feGaussianBlur stdDeviation="1.5" result="coloredBlur" />
            <feMerge>
              <feMergeNode in="coloredBlur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>

        <!-- 中心光晕 -->
        <circle cx="80" cy="80" r="35" fill="url(#centerGlow)" opacity="0.8" />

        <!-- 中心星球 -->
        <circle cx="80" cy="80" r="6" fill="var(--text-200)" filter="url(#glow)" />

        <!-- 第一个轨道环 - 45度 -->
        <ellipse
          cx="80"
          cy="80"
          rx="35"
          ry="18"
          fill="none"
          stroke="var(--text-400)"
          stroke-width="0.8"
          opacity="0.4"
          transform="rotate(45 80 80)"
        />

        <!-- 第二个轨道环 - 水平 -->
        <ellipse
          cx="80"
          cy="80"
          rx="45"
          ry="22"
          fill="none"
          stroke="var(--text-400)"
          stroke-width="0.6"
          opacity="0.3"
          transform="rotate(0 80 80)"
        />

        <!-- 第三个轨道环 - -45度 -->
        <ellipse
          cx="80"
          cy="80"
          rx="55"
          ry="25"
          fill="none"
          stroke="var(--text-400)"
          stroke-width="0.4"
          opacity="0.2"
          transform="rotate(-45 80 80)"
        />

        <!-- 行星 -->
        <circle class="planet planet-1" cx="80" cy="80" r="2.5" fill="var(--text-200)" opacity="0.9" />
        <circle class="planet planet-2" cx="80" cy="80" r="2" fill="var(--text-300)" opacity="0.8" />
        <circle class="planet planet-3" cx="80" cy="80" r="1.5" fill="var(--text-400)" opacity="0.7" />
      </svg>
    </div>
    <h1 class="title">OrbitX</h1>
    <div class="shortcuts">
      <div class="shortcut" @click="handleNewTabClick">
        <span class="shortcut-desc">{{ t('shortcuts.actions.new_tab') }}</span>
        <div class="keys">
          <kbd>⌘</kbd>
          <kbd>T</kbd>
        </div>
      </div>
      <div class="shortcut" @click="handleToggleAISidebarClick">
        <span class="shortcut-desc">{{ t('shortcuts.actions.toggle_ai_sidebar') }}</span>
        <div class="keys">
          <kbd>⌘</kbd>
          <kbd>I</kbd>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
  import { useI18n } from 'vue-i18n'
  import { shortcutActionsService } from '@/shortcuts/actions'

  const { t } = useI18n()

  // 处理新建标签页点击
  const handleNewTabClick = async () => {
    await shortcutActionsService.newTab()
  }

  // 处理切换AI侧边栏点击
  const handleToggleAISidebarClick = () => {
    shortcutActionsService.toggleAISidebar()
  }
</script>

<style scoped>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    background: var(--bg-200);
  }

  .orbit-logo {
    display: flex;
    justify-content: center;
    align-items: center;
  }

  .orbit-logo svg {
    filter: drop-shadow(0 0 12px rgba(255, 255, 255, 0.15));
    animation: float 6s ease-in-out infinite;
  }

  @keyframes float {
    0%,
    100% {
      transform: translateY(0px);
    }
    50% {
      transform: translateY(-8px);
    }
  }

  @property --angle {
    syntax: '<angle>';
    inherits: true;
    initial-value: 0deg;
  }

  .planet {
    transform-origin: 80px 80px;
  }

  /* 主行星轨道 */
  .planet-1 {
    --x-amplitude: 35px;
    --y-amplitude: 18px;
    --rotation: 45deg;
    --x: calc(cos(var(--angle)) * var(--x-amplitude));
    --y: calc(sin(var(--angle)) * var(--y-amplitude));
    transform: rotate(var(--rotation)) translate(var(--x), var(--y)) rotate(calc(var(--rotation) * -1));
    animation: revolve-1 4s linear infinite;
  }

  .planet-2 {
    --x-amplitude: 45px;
    --y-amplitude: 22px;
    --rotation: 0deg;
    --x: calc(cos(var(--angle)) * var(--x-amplitude));
    --y: calc(sin(var(--angle)) * var(--y-amplitude));
    transform: rotate(var(--rotation)) translate(var(--x), var(--y)) rotate(calc(var(--rotation) * -1));
    animation: revolve-2 6s linear infinite;
  }

  .planet-3 {
    --x-amplitude: 55px;
    --y-amplitude: 25px;
    --rotation: -45deg;
    --x: calc(cos(var(--angle)) * var(--x-amplitude));
    --y: calc(sin(var(--angle)) * var(--y-amplitude));
    transform: rotate(var(--rotation)) translate(var(--x), var(--y)) rotate(calc(var(--rotation) * -1));
    animation: revolve-3 8s linear infinite;
  }

  @keyframes revolve-1 {
    from {
      --angle: 0deg;
    }
    to {
      --angle: 360deg;
    }
  }

  @keyframes revolve-2 {
    from {
      --angle: 0deg;
    }
    to {
      --angle: 360deg;
    }
  }

  @keyframes revolve-3 {
    from {
      --angle: 0deg;
    }
    to {
      --angle: 360deg;
    }
  }

  .title {
    font-size: 48px;
    font-weight: 300;
    color: var(--text-200);
    margin: 0 0 48px 0;
    letter-spacing: -0.02em;
  }

  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: 16px;
    align-items: center;
  }

  .shortcut {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    padding: 12px 20px;
    background: var(--bg-300);
    border: 1px solid var(--border-200);
    border-radius: var(--border-radius-lg);
    transition: all 0.2s ease;
    min-width: 200px;
    cursor: pointer;
    user-select: none;
  }

  .shortcut:hover {
    background: var(--bg-400);
    border-color: var(--border-300);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .shortcut:active {
    transform: translateY(0);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .keys {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    padding: 0 8px;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-200);
    background: var(--bg-500);
    border: 1px solid var(--border-400);
    border-radius: var(--border-radius);
    box-shadow:
      0 2px 4px rgba(0, 0, 0, 0.1),
      0 0 0 1px rgba(255, 255, 255, 0.05) inset;
  }

  .shortcut-desc {
    font-size: 13px;
    color: var(--text-300);
    font-weight: 500;
    text-align: center;
  }
</style>
