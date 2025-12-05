<script setup lang="ts">
  import { onMounted, onBeforeUnmount } from 'vue'
  import { useImageLightboxStore } from '@/stores/imageLightbox'

  const lightboxStore = useImageLightboxStore()

  const handleClose = () => {
    lightboxStore.closeImage()
  }

  const handleBackdropClick = (event: MouseEvent) => {
    if (event.target === event.currentTarget) {
      handleClose()
    }
  }

  const handleKeydown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      handleClose()
    }
  }

  onMounted(() => {
    document.addEventListener('keydown', handleKeydown)
  })

  onBeforeUnmount(() => {
    document.removeEventListener('keydown', handleKeydown)
  })
</script>

<template>
  <div v-if="lightboxStore.selectedImage" class="image-lightbox" @click="handleBackdropClick">
    <div class="lightbox-content">
      <button class="close-button" @click="handleClose">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 6L6 18M6 6l12 12" />
        </svg>
      </button>
      <img
        :src="lightboxStore.selectedImage.dataUrl"
        :alt="lightboxStore.selectedImage.fileName"
        class="preview-image"
      />
      <div v-if="lightboxStore.selectedImage.fileName" class="image-name">
        {{ lightboxStore.selectedImage.fileName }}
      </div>
    </div>
  </div>
</template>

<style scoped>
  .image-lightbox {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.2s ease;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .lightbox-content {
    position: relative;
    max-width: 90%;
    max-height: 90%;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .close-button {
    position: absolute;
    top: 12px;
    right: 12px;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.6);
    border: none;
    border-radius: 50%;
    color: white;
    cursor: pointer;
    transition: all 0.2s ease;
    z-index: 10;
  }

  .close-button:hover {
    background: rgba(220, 38, 38, 0.9);
  }

  .close-button svg {
    width: 16px;
    height: 16px;
  }

  .preview-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 8px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .image-name {
    color: white;
    font-size: 14px;
    text-align: center;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
