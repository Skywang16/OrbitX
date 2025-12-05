<script setup lang="ts">
  import { useImageLightboxStore, type ImageAttachment } from '@/stores/imageLightbox'

  export type { ImageAttachment }

  interface Props {
    images: ImageAttachment[]
  }

  interface Emits {
    (e: 'remove', id: string): void
  }

  defineProps<Props>()
  const emit = defineEmits<Emits>()

  const lightboxStore = useImageLightboxStore()

  const handleRemove = (event: MouseEvent, id: string) => {
    event.stopPropagation()
    emit('remove', id)
  }

  const handleImageClick = (image: ImageAttachment) => {
    lightboxStore.openImage(image)
  }
</script>

<template>
  <div v-if="images.length > 0" class="image-preview-container">
    <div class="image-list">
      <div v-for="image in images" :key="image.id" class="image-item" @click="handleImageClick(image)">
        <img :src="image.dataUrl" :alt="image.fileName" class="preview-image" />
        <button class="remove-button" @click="handleRemove($event, image.id)">
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
            <path d="M18 6L6 18M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
  .image-preview-container {
    margin-bottom: 8px;
  }

  .image-list {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .image-item {
    position: relative;
    width: 50px;
    height: 50px;
    border-radius: 6px;
    overflow: hidden;
    background: var(--bg-400);
    border: 1px solid var(--border-300);
    cursor: pointer;
    transition: border-color 0.2s ease;
  }

  .image-item:hover {
    border-color: var(--color-primary);
  }

  .preview-image {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .remove-button {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.7);
    border: none;
    border-radius: 50%;
    color: white;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.2s ease;
  }

  .image-item:hover .remove-button {
    opacity: 1;
  }

  .remove-button:hover {
    background: rgba(220, 38, 38, 0.9);
  }
</style>
