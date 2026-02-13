/**
 * 图片处理工具函数
 */

export interface ProcessedImage {
  dataUrl: string
  mimeType: string
  fileName: string
  fileSize: number
}

/**
 * 将文件转换为 base64 data URL
 */
export const fileToDataUrl = (file: File): Promise<string> => {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(reader.result as string)
    reader.onerror = reject
    reader.readAsDataURL(file)
  })
}

/**
 * 压缩图片（如果超过最大尺寸）
 */
export const compressImage = async (
  dataUrl: string,
  maxWidth: number = 2048,
  maxHeight: number = 2048,
  quality: number = 0.9
): Promise<string> => {
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.onload = () => {
      let { width, height } = img

      // 计算缩放比例
      if (width > maxWidth || height > maxHeight) {
        const ratio = Math.min(maxWidth / width, maxHeight / height)
        width = Math.floor(width * ratio)
        height = Math.floor(height * ratio)
      }

      // 创建 canvas 进行压缩
      const canvas = document.createElement('canvas')
      canvas.width = width
      canvas.height = height

      const ctx = canvas.getContext('2d')
      if (!ctx) {
        reject(new Error('Failed to get canvas context'))
        return
      }

      ctx.drawImage(img, 0, 0, width, height)

      // 转换为 base64
      const compressedDataUrl = canvas.toDataURL('image/jpeg', quality)
      resolve(compressedDataUrl)
    }
    img.onerror = reject
    img.src = dataUrl
  })
}

/**
 * 处理图片文件：读取、压缩、返回处理后的数据
 */
export const processImageFile = async (file: File): Promise<ProcessedImage> => {
  // 检查文件类型
  if (!file.type.startsWith('image/')) {
    throw new Error('File is not an image')
  }

  // 读取文件
  const dataUrl = await fileToDataUrl(file)

  // 压缩图片（如果需要）
  const compressedDataUrl = await compressImage(dataUrl)

  return {
    dataUrl: compressedDataUrl,
    mimeType: 'image/jpeg', // 压缩后统一为 JPEG
    fileName: file.name,
    fileSize: file.size,
  }
}

/**
 * 从剪贴板获取图片
 */
export const getImageFromClipboard = async (event: ClipboardEvent): Promise<File | null> => {
  const items = event.clipboardData?.items
  if (!items) return null

  for (const item of Array.from(items)) {
    if (item.type.startsWith('image/')) {
      const file = item.getAsFile()
      return file
    }
  }

  return null
}

/**
 * 验证图片文件
 */
export const validateImageFile = (file: File): { valid: boolean; error?: string } => {
  // 检查文件类型
  const validTypes = ['image/jpeg', 'image/jpg', 'image/png', 'image/gif', 'image/webp']
  if (!validTypes.includes(file.type)) {
    return {
      valid: false,
      error: 'Unsupported image format. Please use JPEG, PNG, GIF, or WebP.',
    }
  }

  // 检查文件大小（最大 20MB）
  const maxSize = 20 * 1024 * 1024
  if (file.size > maxSize) {
    return {
      valid: false,
      error: 'Image file is too large. Maximum size is 20MB.',
    }
  }

  return { valid: true }
}
