/**
 * 色阶生成工具
 * 根据基础颜色生成完整的1-9级色阶系统
 */

/**
 * 将十六进制颜色转换为HSL
 */
function hexToHsl(hex) {
  const r = parseInt(hex.slice(1, 3), 16) / 255
  const g = parseInt(hex.slice(3, 5), 16) / 255
  const b = parseInt(hex.slice(5, 7), 16) / 255

  const max = Math.max(r, g, b)
  const min = Math.min(r, g, b)
  let h,
    s,
    l = (max + min) / 2

  if (max === min) {
    h = s = 0 // achromatic
  } else {
    const d = max - min
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
    switch (max) {
      case r:
        h = (g - b) / d + (g < b ? 6 : 0)
        break
      case g:
        h = (b - r) / d + 2
        break
      case b:
        h = (r - g) / d + 4
        break
    }
    h /= 6
  }

  return [h * 360, s * 100, l * 100]
}

/**
 * 将HSL转换为十六进制颜色
 */
function hslToHex(h, s, l) {
  h /= 360
  s /= 100
  l /= 100

  const hue2rgb = (p, q, t) => {
    if (t < 0) t += 1
    if (t > 1) t -= 1
    if (t < 1 / 6) return p + (q - p) * 6 * t
    if (t < 1 / 2) return q
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6
    return p
  }

  let r, g, b

  if (s === 0) {
    r = g = b = l // achromatic
  } else {
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s
    const p = 2 * l - q
    r = hue2rgb(p, q, h + 1 / 3)
    g = hue2rgb(p, q, h)
    b = hue2rgb(p, q, h - 1 / 3)
  }

  const toHex = c => {
    const hex = Math.round(c * 255).toString(16)
    return hex.length === 1 ? '0' + hex : hex
  }

  return `#${toHex(r)}${toHex(g)}${toHex(b)}`
}

/**
 * 生成颜色色阶
 * @param {string} baseColor - 基础颜色（十六进制）
 * @param {string} colorName - 颜色名称
 * @param {boolean} isDark - 是否为深色主题
 */
function generateColorScale(baseColor, colorName, isDark = false) {
  const [h, s, l] = hexToHsl(baseColor)
  const scale = {}

  // 色阶配置：[亮度调整, 饱和度调整]
  const lightScaleConfig = [
    [95, -20], // 1: 最浅
    [85, -15], // 2: 很浅
    [75, -10], // 3: 浅
    [65, -5], // 4: 较浅
    [l, 0], // 5: 基础色
    [l - 10, 5], // 6: 较深
    [l - 20, 10], // 7: 深
    [l - 30, 15], // 8: 很深
    [l - 40, 20], // 9: 最深
  ]

  const darkScaleConfig = [
    [l + 40, 20], // 1: 最浅（深色主题下实际是较亮）
    [l + 30, 15], // 2: 很浅
    [l + 20, 10], // 3: 浅
    [l + 10, 5], // 4: 较浅
    [l, 0], // 5: 基础色
    [65, -5], // 6: 较深
    [75, -10], // 7: 深
    [85, -15], // 8: 很深
    [95, -20], // 9: 最深
  ]

  const config = isDark ? darkScaleConfig : lightScaleConfig

  for (let i = 1; i <= 9; i++) {
    const [lightness, satAdjust] = config[i - 1]
    const adjustedSat = Math.max(0, Math.min(100, s + satAdjust))
    const adjustedLight = Math.max(0, Math.min(100, lightness))

    scale[`--color-${colorName}-${i}`] = hslToHex(h, adjustedSat, adjustedLight)
  }

  return scale
}

/**
 * 生成完整的颜色系统
 */
function generateColorSystem(theme = 'light') {
  const isDark = theme === 'dark'

  // 基础颜色定义
  const baseColors = {
    primary: '#007acc',
    success: '#52c41a',
    warning: '#faad14',
    danger: '#ff4d4f',
    info: '#1890ff',
  }

  // 中性色定义
  const grayColors = isDark
    ? {
        1: '#262626',
        2: '#434343',
        3: '#595959',
        4: '#8c8c8c',
        5: '#bfbfbf',
        6: '#d9d9d9',
        7: '#f0f0f0',
        8: '#f5f5f5',
        9: '#fafafa',
      }
    : {
        1: '#fafafa',
        2: '#f5f5f5',
        3: '#f0f0f0',
        4: '#d9d9d9',
        5: '#bfbfbf',
        6: '#8c8c8c',
        7: '#595959',
        8: '#434343',
        9: '#262626',
      }

  let colorSystem = {}

  // 生成功能色色阶
  Object.entries(baseColors).forEach(([name, color]) => {
    const scale = generateColorScale(color, name, isDark)
    colorSystem = { ...colorSystem, ...scale }
  })

  // 添加中性色
  Object.entries(grayColors).forEach(([level, color]) => {
    colorSystem[`--color-gray-${level}`] = color
  })

  return colorSystem
}

/**
 * 生成CSS变量字符串
 */
function generateCSSVariables(colorSystem, themeName = '') {
  const selector = themeName ? `[data-theme='${themeName}']` : ':root'

  let css = `/* ${themeName || '默认'} 主题色阶系统 */\n${selector} {\n`

  Object.entries(colorSystem).forEach(([variable, value]) => {
    css += `  ${variable}: ${value};\n`
  })

  css += '}\n'

  return css
}

// 导出函数供其他模块使用
if (typeof module !== 'undefined' && module.exports) {
  module.exports = {
    generateColorScale,
    generateColorSystem,
    generateCSSVariables,
    hexToHsl,
    hslToHex,
  }
}

// 如果直接运行此脚本，生成示例
if (typeof require !== 'undefined' && require.main === module) {
  const fs = require('fs')
  const path = require('path')

  // 生成浅色主题
  const lightColors = generateColorSystem('light')
  const lightCSS = generateCSSVariables(lightColors, 'light')

  // 生成深色主题
  const darkColors = generateColorSystem('dark')
  const darkCSS = generateCSSVariables(darkColors, 'dark')

  // 输出到控制台
  console.log('=== 浅色主题色阶 ===')
  console.log(lightCSS)
  console.log('\n=== 深色主题色阶 ===')
  console.log(darkCSS)

  // 保存到文件
  const outputDir = path.join(__dirname, '../src/styles/generated')
  if (!fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true })
  }

  fs.writeFileSync(path.join(outputDir, 'color-scale-light.css'), lightCSS)
  fs.writeFileSync(path.join(outputDir, 'color-scale-dark.css'), darkCSS)

  console.log('\n色阶文件已生成到 src/styles/generated/ 目录')
}
