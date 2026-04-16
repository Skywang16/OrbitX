<script setup lang="ts">
  import { useImageLightboxStore } from '@/stores/imageLightbox'
  import { createSlashCommands, SLASH_COMMAND_ICONS } from '@/types/slashCommand'
  import type { CheckpointSummary, Message } from '@/types'
  import { computed } from 'vue'
  import { useI18n } from 'vue-i18n'
  import CheckpointIndicator from './CheckpointIndicator.vue'

  interface Props {
    message: Message
    checkpoint?: CheckpointSummary | null
    workspacePath?: string
  }

  const props = defineProps<Props>()
  const { t } = useI18n()

  const lightboxStore = useImageLightboxStore()
  const slashCommands = computed(() => createSlashCommands(t))

  const COMMAND_MARKER_RE = /^<!--\s*command:(\S+)\s*-->\n?/
  const LEGACY_PREFIX_RE = /^\/(code-review|skill-creator|skill-installer|plan-mode|orchestrate-mode)\n/
  const LEADING_IMAGE_PLACEHOLDER_RE = /^(?:\s*\[Image #\d+\]\s*)+/
  const XML_MODE_RE = /^<([a-z][\w-]*-mode)(?:\s+[^>]*)?>\s*/i
  const XML_MODE_CLOSE_RE = /\s*<\/([a-z][\w-]*-mode)>$/i
  const XML_MODE_SELF_CLOSING_RE = /^<([a-z][\w-]*-mode)(?:\s+[^>]*)?\s*\/>\s*/i

  const formatCommandLabel = (id: string) => id.replace(/[-_]+/g, ' ').replace(/\b\w/g, char => char.toUpperCase())

  const parsedContent = computed(() => {
    const block = props.message.blocks.find(b => b.type === 'user_text')
    const raw = block?.content || ''

    let commandId: string | null = null
    let text = raw

    text = text.replace(LEADING_IMAGE_PLACEHOLDER_RE, '').trimStart()

    const markerMatch = text.match(COMMAND_MARKER_RE)
    if (markerMatch) {
      commandId = markerMatch[1]
      text = text.slice(markerMatch[0].length)
    } else {
      const legacyMatch = text.match(LEGACY_PREFIX_RE)
      if (legacyMatch) {
        commandId = legacyMatch[1]
        text = text.slice(legacyMatch[0].length)
      } else {
        const selfClosingXmlMatch = text.match(XML_MODE_SELF_CLOSING_RE)
        if (selfClosingXmlMatch) {
          commandId = selfClosingXmlMatch[1]
          text = text.slice(selfClosingXmlMatch[0].length)
        } else {
          const xmlMatch = text.match(XML_MODE_RE)
          if (xmlMatch) {
            commandId = xmlMatch[1]
            text = text.slice(xmlMatch[0].length)
            const closeMatch = text.match(XML_MODE_CLOSE_RE)
            if (closeMatch && closeMatch[1].toLowerCase() === commandId.toLowerCase()) {
              text = text.slice(0, closeMatch.index)
            }
          }
        }
      }
    }

    return { commandId, text: text.trim() }
  })

  const userText = computed(() => parsedContent.value.text)

  const commandInfo = computed(() => {
    const id = parsedContent.value.commandId
    if (!id) return null
    const cmd = slashCommands.value.find(c => c.id === id)
    if (!cmd) {
      return {
        id,
        label: formatCommandLabel(id),
        icon: SLASH_COMMAND_ICONS.sparkles || '',
      }
    }
    return {
      id,
      label: cmd.label,
      icon: SLASH_COMMAND_ICONS[cmd.icon] || '',
    }
  })

  const isModeTag = computed(() => (commandInfo.value?.id || '').endsWith('-mode'))

  const userImages = computed(() => {
    return props.message.blocks.filter(b => b.type === 'user_image')
  })

  const handleImageClick = (image: { id: string; dataUrl: string; fileName: string }) => {
    lightboxStore.openImage({
      id: image.id,
      dataUrl: image.dataUrl,
      fileName: image.fileName,
      fileSize: 0,
      mimeType: 'image/jpeg',
    })
  }
</script>

<template>
  <div class="user-message" :data-user-message-id="message.id">
    <div class="user-message-content">
      <div class="user-message-bubble">
        <div v-if="commandInfo" class="command-indicator" :class="{ 'command-indicator-mode': isModeTag }">
          <span class="command-indicator-icon" v-html="commandInfo.icon" />
          <span class="command-indicator-label">{{ commandInfo.label }}</span>
        </div>
        <div v-if="userImages.length > 0" class="user-message-images">
          <div
            v-for="(image, index) in userImages"
            :key="`${message.id}-img-${index}`"
            class="message-image-item"
            @click="
              handleImageClick({
                id: `${message.id}-img-${index}`,
                dataUrl: image.dataUrl,
                fileName: image.fileName || `image_${index}`,
              })
            "
          >
            <img :src="image.dataUrl" :alt="image.fileName || `image_${index}`" class="message-image" />
          </div>
        </div>
        <div v-if="userText" class="user-message-text">{{ userText }}</div>
      </div>
      <CheckpointIndicator
        class="rollback-action"
        :checkpoint="checkpoint"
        :workspace-path="workspacePath || ''"
        :message-content="userText"
      />
    </div>
  </div>
</template>

<style scoped>
  .user-message {
    display: flex;
    justify-content: flex-end;
    padding: var(--spacing-md) 0;
  }

  .user-message-content {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    max-width: 85%;
  }

  .user-message-bubble {
    background: var(--color-primary-alpha);
    color: var(--text-100);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-lg);
    word-wrap: break-word;
    word-break: break-word;
    white-space: pre-wrap;
    animation: bubble-enter 0.3s cubic-bezier(0.4, 0, 0.2, 1) both;
  }

  @keyframes bubble-enter {
    from {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .user-message-text {
    font-size: var(--font-size-md);
    line-height: 1.6;
  }

  .command-indicator {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px 2px 5px;
    margin-bottom: 6px;
    background: color-mix(in srgb, var(--text-100) 8%, transparent);
    border: 1px solid transparent;
    border-radius: var(--border-radius-md);
    font-size: 11px;
    font-weight: 500;
    color: var(--text-200);
    line-height: 1;
  }

  .command-indicator-mode {
    background: color-mix(in srgb, var(--color-primary) 18%, transparent);
    border-color: color-mix(in srgb, var(--color-primary) 36%, transparent);
    color: var(--color-primary);
  }

  .command-indicator-icon {
    width: 13px;
    height: 13px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .command-indicator-icon :deep(svg) {
    width: 13px;
    height: 13px;
    stroke: currentColor;
    stroke-width: 2;
  }

  .command-indicator-label {
    line-height: 1;
  }

  .user-message-images {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: var(--spacing-sm);
  }

  .message-image-item {
    width: 80px;
    height: 80px;
    border-radius: var(--border-radius-md);
    overflow: hidden;
    cursor: pointer;
    transition: transform 0.2s ease;
  }

  .message-image-item:hover {
    transform: scale(1.05);
  }

  .message-image {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .rollback-action {
    opacity: 0;
    transition: opacity 0.15s ease;
    margin-top: var(--spacing-xs);
  }

  .user-message:hover .rollback-action {
    opacity: 1;
  }
</style>
