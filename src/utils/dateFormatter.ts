/**
 * 统一的时间格式化工具
 * 使用dayjs替代原生Date API
 */

import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import 'dayjs/locale/en'
import { getCurrentLocale } from '@/i18n'

// 配置dayjs
dayjs.extend(relativeTime)

// 动态设置locale
const updateDayjsLocale = () => {
  const currentLocale = getCurrentLocale()
  dayjs.locale(currentLocale === 'zh-CN' ? 'zh-cn' : 'en')
}
updateDayjsLocale()

/**
 * 格式化时间为 HH:mm 格式
 * @param date 日期对象、字符串或时间戳
 * @returns 格式化后的时间字符串，如 "14:30"
 */
export const formatTime = (date: Date | string | number): string => {
  return dayjs(date).format('HH:mm')
}

/**
 * 格式化日期为相对时间
 * @param date 日期对象、字符串或时间戳
 * @returns 相对时间字符串，如 "2小时前"、"昨天"、"3天前"
 */
export const formatRelativeTime = (date: Date | string | number): string => {
  // 确保使用当前语言设置
  updateDayjsLocale()
  
  const target = dayjs(date)
  const now = dayjs()
  const diffDays = now.diff(target, 'day')
  const currentLocale = getCurrentLocale()

  if (diffDays === 0) {
    // 今天，显示具体时间
    return target.format('HH:mm')
  } else if (diffDays === 1) {
    // 昨天
    return currentLocale === 'zh-CN' ? '昨天' : 'Yesterday'
  } else if (diffDays < 7) {
    // 一周内，显示天数
    if (currentLocale === 'zh-CN') {
      return `${diffDays}天前`
    } else {
      return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`
    }
  } else if (diffDays < 30) {
    // 一个月内，显示周数
    const weeks = Math.floor(diffDays / 7)
    if (currentLocale === 'zh-CN') {
      return `${weeks}周前`
    } else {
      return `${weeks} week${weeks > 1 ? 's' : ''} ago`
    }
  } else {
    // 超过一个月，显示具体日期
    return target.format('MM-DD')
  }
}

/**
 * 格式化完整的日期时间
 * @param date 日期对象、字符串或时间戳
 * @returns 完整的日期时间字符串，如 "2024-01-15 14:30:25"
 */
export const formatDateTime = (date: Date | string | number): string => {
  return dayjs(date).format('YYYY-MM-DD HH:mm:ss')
}

/**
 * 格式化日期
 * @param date 日期对象、字符串或时间戳
 * @returns 日期字符串，如 "2024-01-15"
 */
export const formatDate = (date: Date | string | number): string => {
  return dayjs(date).format('YYYY-MM-DD')
}

/**
 * 格式化为本地化的日期时间
 * @param date 日期对象、字符串或时间戳
 * @returns 本地化的日期时间字符串
 */
export const formatLocaleDateTime = (date: Date | string | number): string => {
  return dayjs(date).format('YYYY年MM月DD日 HH:mm')
}

/**
 * 格式化为简短的日期
 * @param date 日期对象、字符串或时间戳
 * @returns 简短的日期字符串，如 "1月15日"
 */
export const formatShortDate = (date: Date | string | number): string => {
  return dayjs(date).format('M月D日')
}

/**
 * 检查日期是否有效
 * @param date 日期对象、字符串或时间戳
 * @returns 是否为有效日期
 */
export const isValidDate = (date: Date | string | number): boolean => {
  return dayjs(date).isValid()
}

/**
 * 获取相对时间（使用dayjs的相对时间插件）
 * @param date 日期对象、字符串或时间戳
 * @returns 相对时间字符串，如 "2小时前"
 */
export const getRelativeTime = (date: Date | string | number): string => {
  return dayjs(date).fromNow()
}

/**
 * 格式化文件修改时间（用于工具文件）
 * @param timestamp 时间戳（秒）
 * @returns 格式化后的时间字符串
 */
export const formatFileTime = (timestamp: number): string => {
  return dayjs(timestamp * 1000).format('YYYY-MM-DD HH:mm:ss')
}

/**
 * 格式化会话时间（专门用于会话列表）
 * @param date 日期对象、字符串或时间戳
 * @returns 适合会话列表显示的时间格式
 */
export const formatSessionTime = (date: Date | string | number): string => {
  const target = dayjs(date)
  const now = dayjs()
  const diffDays = now.diff(target, 'day')
  const diffHours = now.diff(target, 'hour')
  const diffMinutes = now.diff(target, 'minute')

  if (diffMinutes < 1) {
    return '刚刚'
  } else if (diffMinutes < 60) {
    return `${diffMinutes}分钟前`
  } else if (diffHours < 24) {
    return `${diffHours}小时前`
  } else if (diffDays === 1) {
    return '昨天'
  } else if (diffDays < 7) {
    return `${diffDays}天前`
  } else {
    return target.format('MM-DD')
  }
}
