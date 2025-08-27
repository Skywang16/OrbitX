// XUI 组件库类型定义

// 基础类型
export type Size = 'small' | 'medium' | 'large'
export type Theme = 'light' | 'dark'
export type Placement = 'top' | 'top-start' | 'top-end' | 'bottom' | 'bottom-start' | 'bottom-end' | 'left' | 'right'

// 按钮组件属性类型
export interface ButtonProps {
  variant?: 'primary' | 'secondary' | 'danger' | 'ghost' | 'link'
  size?: Size
  disabled?: boolean
  loading?: boolean
  type?: 'button' | 'submit' | 'reset'
  icon?: string
  iconPosition?: 'left' | 'right'
  block?: boolean
  round?: boolean
  circle?: boolean
}

// 按钮组件事件类型
export interface ButtonEmits {
  click: (event: MouseEvent) => void
}

// 开关组件属性类型
export interface SwitchProps {
  modelValue: boolean
  disabled?: boolean
  loading?: boolean
  size?: Size
  checkedText?: string
  uncheckedText?: string
  checkedColor?: string
  uncheckedColor?: string
}

// 开关组件事件类型
export interface SwitchEmits {
  'update:modelValue': (value: boolean) => void
  change: (value: boolean) => void
}

// 模态框组件属性类型
export interface ModalProps {
  visible?: boolean
  title?: string
  size?: 'small' | 'medium' | 'large' | 'full'
  closable?: boolean
  maskClosable?: boolean
  showHeader?: boolean
  showFooter?: boolean
  showCancelButton?: boolean
  showConfirmButton?: boolean
  cancelText?: string
  confirmText?: string
  loadingText?: string
  closeButtonTitle?: string
  loading?: boolean
  noPadding?: boolean
  zIndex?: number
}

// 模态框组件事件类型
export interface ModalEmits {
  'update:visible': (visible: boolean) => void
  close: () => void
  cancel: () => void
  confirm: () => void
  opened: () => void
  closed: () => void
}

// 弹出框组件属性类型
export interface PopoverProps {
  modelValue?: boolean
  visible?: boolean
  trigger?: 'click' | 'hover' | 'manual'
  triggerText?: string
  placement?: Placement
  offset?: number
  x?: number
  y?: number
  content?: string
  menuItems?: Array<{
    label: string
    value?: unknown
    icon?: string | object
    disabled?: boolean
    onClick?: () => void
  }>
  width?: string | number
  maxWidth?: string | number
  disabled?: boolean
  mask?: boolean
  closeOnClickOutside?: boolean
  closeOnClickInside?: boolean
}

// 弹出框组件事件类型
export interface PopoverEmits {
  'update:modelValue': (value: boolean) => void
  'update:visible': (value: boolean) => void
  action: () => void
  close: () => void
  show: () => void
  hide: () => void
  'menu-item-click': (item: any) => void
}

// 搜索输入框组件属性类型
export interface SearchInputProps {
  modelValue?: string
  placeholder?: string
  disabled?: boolean
  clearable?: boolean
  autofocus?: boolean
  debounce?: number
  size?: Size
  maxLength?: number
  showWordLimit?: boolean
}

// 搜索输入框组件事件类型
export interface SearchInputEmits {
  'update:modelValue': (value: string) => void
  search: (value: string) => void
  focus: (event: FocusEvent) => void
  blur: (event: FocusEvent) => void
  clear: () => void
  input: (value: string) => void
}

// 消息组件属性类型
export interface MessageProps {
  visible: boolean
  message: string
  type?: 'success' | 'warning' | 'error' | 'info'
  duration?: number
  closable?: boolean
  showIcon?: boolean
  dangerouslyUseHTMLString?: boolean
}

// 消息组件事件类型
export interface MessageEmits {
  close: () => void
}

// 选择器选项类型
export interface SelectOption {
  label: string
  value: string | number
  disabled?: boolean
  icon?: string
  description?: string
}

// 选择器组件属性类型
export interface SelectProps {
  modelValue?: string | number | null
  options: SelectOption[]
  placeholder?: string
  disabled?: boolean
  clearable?: boolean
  filterable?: boolean
  size?: Size
  borderless?: boolean
  placement?: 'top' | 'bottom' | 'auto'
  maxHeight?: string | number
  noDataText?: string
  filterPlaceholder?: string
  loading?: boolean
  multiple?: boolean
  multipleLimit?: number
  collapseTags?: boolean
  allowCreate?: boolean
  remote?: boolean
  remoteMethod?: (query: string) => void
}

// 选择器组件事件类型
export interface SelectEmits {
  'update:modelValue': (value: string | number | null | Array<string | number>) => void
  change: (value: string | number | null | Array<string | number>) => void
  focus: (event: FocusEvent) => void
  blur: (event: FocusEvent) => void
  clear: () => void
  'visible-change': (visible: boolean) => void
  'remove-tag': (value: string | number) => void
}

// 气泡确认框组件属性类型
export interface PopconfirmProps {
  title?: string
  description?: string
  confirmText?: string
  cancelText?: string
  type?: 'warning' | 'danger' | 'info'
  placement?: Placement
  trigger?: 'click' | 'hover' | 'manual'
  disabled?: boolean
  loading?: boolean
  closeOnClickOutside?: boolean
  offset?: number
  icon?: string | object
  triggerText?: string
  triggerButtonVariant?: 'primary' | 'secondary' | 'danger' | 'ghost' | 'link'
  triggerButtonSize?: Size
  triggerButtonProps?: Record<string, any>
}

// 气泡确认框组件事件类型
export interface PopconfirmEmits {
  confirm: () => void
  cancel: () => void
  'update:visible': (value: boolean) => void
}

// 组件实例类型
export type ButtonInstance = InstanceType<typeof import('./Button.vue').default>
export type SwitchInstance = InstanceType<typeof import('./Switch.vue').default>
export type ModalInstance = InstanceType<typeof import('./Modal.vue').default>
export type PopoverInstance = InstanceType<typeof import('./Popover.vue').default>
export type SearchInputInstance = InstanceType<typeof import('./SearchInput.vue').default>
export type MessageInstance = InstanceType<typeof import('./Message.vue').default>
export type SelectInstance = InstanceType<typeof import('./Select.vue').default>
export type PopconfirmInstance = InstanceType<typeof import('./Popconfirm.vue').default>
