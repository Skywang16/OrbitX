import { defineStore } from 'pinia'
import { ref } from 'vue'

export interface ImageAttachment {
  id: string
  dataUrl: string
  fileName: string
  fileSize: number
  mimeType: string
}

export const useImageLightboxStore = defineStore('imageLightbox', () => {
  const selectedImage = ref<ImageAttachment | null>(null)

  const openImage = (image: ImageAttachment) => {
    selectedImage.value = image
  }

  const closeImage = () => {
    selectedImage.value = null
  }

  return {
    selectedImage,
    openImage,
    closeImage,
  }
})
