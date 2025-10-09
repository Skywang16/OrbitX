export function hexToRgba(hex: string, opacity: number): string {
  const cleanHex = hex.replace(/^#/, '')
  let r: number, g: number, b: number
  if (cleanHex.length === 3) {
    r = parseInt(cleanHex[0] + cleanHex[0], 16)
    g = parseInt(cleanHex[1] + cleanHex[1], 16)
    b = parseInt(cleanHex[2] + cleanHex[2], 16)
  } else if (cleanHex.length === 6) {
    r = parseInt(cleanHex.substring(0, 2), 16)
    g = parseInt(cleanHex.substring(2, 4), 16)
    b = parseInt(cleanHex.substring(4, 6), 16)
  } else {
    console.warn(`Invalid hex color: ${hex}`)
    return `rgba(0, 0, 0, ${opacity})`
  }

  if (isNaN(r) || isNaN(g) || isNaN(b)) {
    console.warn(`Failed to parse hex color: ${hex}`)
    return `rgba(0, 0, 0, ${opacity})`
  }

  return `rgba(${r}, ${g}, ${b}, ${opacity})`
}

export function rgbToRgba(rgb: string, opacity: number): string {
  const rgbMatch = rgb.match(/rgba?\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/)

  if (!rgbMatch) {
    console.warn(`Invalid rgb color: ${rgb}`)
    return `rgba(0, 0, 0, ${opacity})`
  }

  const r = parseInt(rgbMatch[1], 10)
  const g = parseInt(rgbMatch[2], 10)
  const b = parseInt(rgbMatch[3], 10)

  if (isNaN(r) || isNaN(g) || isNaN(b)) {
    console.warn(`Failed to parse rgb color: ${rgb}`)
    return `rgba(0, 0, 0, ${opacity})`
  }

  return `rgba(${r}, ${g}, ${b}, ${opacity})`
}

export function applyOpacityToColor(color: string, opacity: number): string {
  if (!color) {
    return `rgba(0, 0, 0, ${opacity})`
  }

  const clampedOpacity = Math.max(0, Math.min(1, opacity))
  const trimmedColor = color.trim()

  if (trimmedColor.startsWith('#')) {
    return hexToRgba(trimmedColor, clampedOpacity)
  }

  if (trimmedColor.startsWith('rgb')) {
    return rgbToRgba(trimmedColor, clampedOpacity)
  }

  if (/^[0-9A-Fa-f]{3}$|^[0-9A-Fa-f]{6}$/.test(trimmedColor)) {
    return hexToRgba(`#${trimmedColor}`, clampedOpacity)
  }

  console.warn(`Unsupported color format: ${color}`)
  return `rgba(0, 0, 0, ${clampedOpacity})`
}

export function getCurrentOpacity(): number {
  if (typeof window === 'undefined' || typeof document === 'undefined') {
    return 1.0
  }

  try {
    const opacityStr = getComputedStyle(document.documentElement).getPropertyValue('--bg-opacity').trim()

    if (!opacityStr) {
      return 1.0
    }

    const opacity = parseFloat(opacityStr)
    return isNaN(opacity) ? 1.0 : Math.max(0, Math.min(1, opacity))
  } catch (error) {
    console.warn('Failed to get current opacity:', error)
    return 1.0
  }
}
