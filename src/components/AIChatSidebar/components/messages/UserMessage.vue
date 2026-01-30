<script setup lang="ts">
  import { computed } from 'vue'
  import type { Message, CheckpointSummary } from '@/types'
  import { useImageLightboxStore } from '@/stores/imageLightbox'
  import CheckpointIndicator from './CheckpointIndicator.vue'

  interface Props {
    message: Message
    checkpoint?: CheckpointSummary | null
    workspacePath?: string
  }

  const props = defineProps<Props>()

  const lightboxStore = useImageLightboxStore()

  const userText = computed(() => {
    const block = props.message.blocks.find(b => b.type === 'user_text')
    return block?.content || ''
  })

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
  <div class="user-message">
    <div class="user-message-content">
      <div class="user-message-bubble">
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
    margin-bottom: var(--spacing-sm);
  }

  .user-message-content {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    max-width: 80%;
    position: relative;
  }

  .rollback-action {
    opacity: 0;
    transition: opacity 0.15s ease;
    margin-top: 4px;
    margin-right: 4px;
  }

  .user-message:hover .rollback-action {
    opacity: 1;
  }

  .user-message-bubble {
    background: var(--color-primary-alpha);
    color: var(--text-100);
    padding: var(--spacing-sm) var(--spacing-md);
    border-radius: var(--border-radius-lg);
    word-wrap: break-word;
    word-break: break-word;
    white-space: pre-wrap;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  .user-message-text {
    font-size: var(--font-size-md);
    line-height: 1.6;
    margin: 0;
    letter-spacing: 0.01em;
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
    border-radius: 6px;
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
</style>
