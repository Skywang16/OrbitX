import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'
import 'dayjs/locale/zh-cn'
import 'dayjs/locale/en'
import { getCurrentLocale } from '@/i18n'

dayjs.extend(relativeTime)

const updateDayjsLocale = () => {
  const currentLocale = getCurrentLocale()
  dayjs.locale(currentLocale === 'zh-CN' ? 'zh-cn' : 'en')
}
updateDayjsLocale()

export const formatTime = (date: Date | string | number): string => {
  return dayjs(date).format('HH:mm')
}

export const formatRelativeTime = (date: Date | string | number): string => {
  updateDayjsLocale()

  const target = dayjs(date)
  const now = dayjs()
  const diffDays = now.diff(target, 'day')
  const currentLocale = getCurrentLocale()

  if (diffDays === 0) {
    return target.format('HH:mm')
  } else if (diffDays === 1) {
    return currentLocale === 'zh-CN' ? '昨天' : 'Yesterday'
  } else if (diffDays < 7) {
    if (currentLocale === 'zh-CN') {
      return `${diffDays}天前`
    } else {
      return `${diffDays} day${diffDays > 1 ? 's' : ''} ago`
    }
  } else if (diffDays < 30) {
    const weeks = Math.floor(diffDays / 7)
    if (currentLocale === 'zh-CN') {
      return `${weeks}周前`
    } else {
      return `${weeks} week${weeks > 1 ? 's' : ''} ago`
    }
  } else {
    return target.format('MM-DD')
  }
}

export const formatDateTime = (date: Date | string | number): string => {
  return dayjs(date).format('YYYY-MM-DD HH:mm:ss')
}

export const formatDate = (date: Date | string | number): string => {
  return dayjs(date).format('YYYY-MM-DD')
}

export const formatLocaleDateTime = (date: Date | string | number): string => {
  return dayjs(date).format('YYYY年MM月DD日 HH:mm')
}

export const formatShortDate = (date: Date | string | number): string => {
  return dayjs(date).format('M月D日')
}

export const isValidDate = (date: Date | string | number): boolean => {
  return dayjs(date).isValid()
}

export const getRelativeTime = (date: Date | string | number): string => {
  return dayjs(date).fromNow()
}

export const formatFileTime = (timestamp: number): string => {
  return dayjs(timestamp * 1000).format('YYYY-MM-DD HH:mm:ss')
}

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
