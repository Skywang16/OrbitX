<template>
  <div ref="selectRef" :class="selectClasses" @click="handleClick" @keydown="handleKeydown" tabindex="0">
    <!-- 选择器输入框 -->
    <div class="x-select__input-wrapper">
      <div class="x-select__input">
        <!-- 显示选中的值 -->
        <span v-if="displayValue" class="x-select__value">{{ displayValue }}</span>
        <span v-else class="x-select__placeholder">{{ placeholder }}</span>

        <!-- 清除按钮 -->
        <button v-if="clearable && modelValue && !disabled" class="x-select__clear" @click.stop="handleClear">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>

        <!-- 下拉箭头 -->
        <div class="x-select__arrow" :class="{ 'x-select__arrow--open': visible }">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="6,9 12,15 18,9"></polyline>
          </svg>
        </div>
      </div>
    </div>

    <!-- 下拉选项面板 -->
    <Teleport to="body">
      <div v-if="visible" ref="dropdownRef" :class="dropdownClasses" :style="dropdownStyle">
        <!-- 搜索框 -->
        <div v-if="filterable" class="x-select__filter">
          <input
            ref="filterInputRef"
            v-model="filterQuery"
            type="text"
            class="x-select__filter-input"
            :placeholder="filterPlaceholder"
            @keydown.stop="handleFilterKeydown"
          />
        </div>

        <!-- 选项列表 -->
        <div class="x-select__options" :style="{ maxHeight: maxHeight }">
          <div
            v-for="(option, index) in filteredOptions"
            :key="option.value"
            :class="getOptionClasses(option, index)"
            @click="handleOptionClick(option)"
            @mouseenter="highlightedIndex = index"
          >
            <!-- 选项图标 -->
            <span v-if="option.icon" class="x-select__option-icon">
              <svg v-if="typeof option.icon === 'string'" viewBox="0 0 24 24">
                <use :href="`#${option.icon}`"></use>
              </svg>
              <component v-else :is="option.icon" />
            </span>

            <!-- 选项内容 -->
            <div class="x-select__option-content">
              <span class="x-select__option-label">{{ option.label }}</span>
              <span v-if="option.description" class="x-select__option-description">
                {{ option.description }}
              </span>
            </div>

            <!-- 选中标记 -->
            <span v-if="isSelected(option)" class="x-select__option-check">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="20,6 9,17 4,12"></polyline>
              </svg>
            </span>
          </div>

          <!-- 无数据提示 -->
          <div v-if="filteredOptions.length === 0" class="x-select__no-data">
            {{ noDataText }}
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
  import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
  import type { SelectProps, SelectOption } from '../types/index'

  const props = withDefaults(defineProps<SelectProps>(), {
    placeholder: '请选择',
    disabled: false,
    clearable: false,
    filterable: false,
    size: 'medium',
    borderless: false,
    placement: 'auto',
    maxHeight: '200px',
    noDataText: '无数据',
    filterPlaceholder: '搜索选项',
    loading: false,
    multiple: false,
    multipleLimit: 0,
    collapseTags: false,
    allowCreate: false,
    remote: false,
  })

  const emit = defineEmits<{
    'update:modelValue': [value: string | number | null | Array<string | number>]
    change: [value: string | number | null | Array<string | number>]
    focus: [event: FocusEvent]
    blur: [event: FocusEvent]
    clear: []
    'visible-change': [visible: boolean]
    'remove-tag': [value: string | number]
  }>()

  // 响应式引用
  const selectRef = ref<HTMLElement>()
  const dropdownRef = ref<HTMLElement>()
  const filterInputRef = ref<HTMLInputElement>()
  const visible = ref(false)
  const filterQuery = ref('')
  const highlightedIndex = ref(-1)

  // 计算属性
  const selectClasses = computed(() => [
    'x-select',
    `x-select--${props.size}`,
    {
      'x-select--disabled': props.disabled,
      'x-select--open': visible.value,
      'x-select--clearable': props.clearable,
      'x-select--borderless': props.borderless,
    },
  ])

  const dropdownClasses = computed(() => ['x-select__dropdown', `x-select__dropdown--${actualPlacement.value}`])

  const displayValue = computed(() => {
    if (!props.modelValue) return ''
    const option = props.options.find(opt => opt.value === props.modelValue)
    return option?.label || ''
  })

  const filteredOptions = computed(() => {
    if (!props.filterable || !filterQuery.value) {
      return props.options
    }

    const query = filterQuery.value.toLowerCase()
    return props.options.filter(
      option =>
        option.label.toLowerCase().includes(query) ||
        (option.description && option.description.toLowerCase().includes(query))
    )
  })

  const actualPlacement = ref<'top' | 'bottom'>('bottom')
  const dropdownStyle = ref<Record<string, string>>({})

  // 方法
  const isSelected = (option: SelectOption) => {
    return option.value === props.modelValue
  }

  const getOptionClasses = (option: SelectOption, index: number) => [
    'x-select__option',
    {
      'x-select__option--disabled': option.disabled,
      'x-select__option--selected': isSelected(option),
      'x-select__option--highlighted': index === highlightedIndex.value,
    },
  ]

  const handleClick = () => {
    if (props.disabled) return

    if (visible.value) {
      hideDropdown()
    } else {
      showDropdown()
    }
  }

  const handleClear = () => {
    emit('update:modelValue', null)
    emit('change', null)
    emit('clear')
  }

  const handleOptionClick = (option: SelectOption) => {
    if (option.disabled) return

    emit('update:modelValue', option.value)
    emit('change', option.value)
    hideDropdown()
  }

  const showDropdown = async () => {
    visible.value = true
    emit('visible-change', true)

    await nextTick()
    updateDropdownPosition()

    if (props.filterable) {
      filterInputRef.value?.focus()
    }
  }

  const hideDropdown = () => {
    visible.value = false
    emit('visible-change', false)
    filterQuery.value = ''
    highlightedIndex.value = -1
  }

  const updateDropdownPosition = () => {
    if (!selectRef.value || !dropdownRef.value) return

    const selectRect = selectRef.value.getBoundingClientRect()
    const dropdownRect = dropdownRef.value.getBoundingClientRect()
    const viewportHeight = window.innerHeight

    // 计算放置位置
    let placement = props.placement
    if (placement === 'auto') {
      const spaceBelow = viewportHeight - selectRect.bottom
      const spaceAbove = selectRect.top
      placement = spaceBelow >= dropdownRect.height || spaceBelow >= spaceAbove ? 'bottom' : 'top'
    }

    actualPlacement.value = placement

    // 设置位置样式
    const style: Record<string, string> = {
      position: 'fixed',
      left: `${selectRect.left}px`,
      width: `${selectRect.width}px`,
      zIndex: '1000',
    }

    if (placement === 'bottom') {
      style.top = `${selectRect.bottom + 4}px`
    } else {
      style.bottom = `${viewportHeight - selectRect.top + 4}px`
    }

    dropdownStyle.value = style
  }

  const handleKeydown = (event: KeyboardEvent) => {
    if (props.disabled) return

    switch (event.key) {
      case 'Enter':
      case ' ':
        event.preventDefault()
        if (!visible.value) {
          showDropdown()
        } else if (highlightedIndex.value >= 0) {
          const option = filteredOptions.value[highlightedIndex.value]
          if (option && !option.disabled) {
            handleOptionClick(option)
          }
        }
        break
      case 'Escape':
        event.preventDefault()
        hideDropdown()
        break
      case 'ArrowDown':
        event.preventDefault()
        if (!visible.value) {
          showDropdown()
        } else {
          highlightedIndex.value = Math.min(highlightedIndex.value + 1, filteredOptions.value.length - 1)
        }
        break
      case 'ArrowUp':
        event.preventDefault()
        if (visible.value) {
          highlightedIndex.value = Math.max(highlightedIndex.value - 1, 0)
        }
        break
    }
  }

  const handleFilterKeydown = (event: KeyboardEvent) => {
    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        highlightedIndex.value = Math.min(highlightedIndex.value + 1, filteredOptions.value.length - 1)
        break
      case 'ArrowUp':
        event.preventDefault()
        highlightedIndex.value = Math.max(highlightedIndex.value - 1, 0)
        break
      case 'Enter':
        event.preventDefault()
        if (highlightedIndex.value >= 0) {
          const option = filteredOptions.value[highlightedIndex.value]
          if (option && !option.disabled) {
            handleOptionClick(option)
          }
        }
        break
      case 'Escape':
        event.preventDefault()
        hideDropdown()
        break
    }
  }

  const handleClickOutside = (event: Event) => {
    if (!selectRef.value || !dropdownRef.value) return

    const target = event.target as Node
    if (!selectRef.value.contains(target) && !dropdownRef.value.contains(target)) {
      hideDropdown()
    }
  }

  // 生命周期
  onMounted(() => {
    document.addEventListener('click', handleClickOutside)
    window.addEventListener('resize', updateDropdownPosition)
    window.addEventListener('scroll', updateDropdownPosition)
  })

  onUnmounted(() => {
    document.removeEventListener('click', handleClickOutside)
    window.removeEventListener('resize', updateDropdownPosition)
    window.removeEventListener('scroll', updateDropdownPosition)
  })

  // 监听器
  watch(visible, newVisible => {
    if (newVisible) {
      nextTick(updateDropdownPosition)
    }
  })

  watch(filteredOptions, () => {
    highlightedIndex.value = -1
  })

  // 暴露方法
  defineExpose({
    focus: () => selectRef.value?.focus(),
    blur: () => selectRef.value?.blur(),
    showDropdown,
    hideDropdown,
  })
</script>

<style scoped>
  /* 选择器主容器 */
  .x-select {
    position: relative;
    display: inline-block;
    width: 100%;
    outline: none;
  }

  .x-select--disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }

  /* 输入框包装器 */
  .x-select__input-wrapper {
    position: relative;
    width: 100%;
  }

  .x-select__input {
    display: flex;
    align-items: center;
    width: 100%;
    min-height: 32px;
    padding: 6px 32px 6px 12px;
    border: 1px solid var(--color-border);
    border-radius: 3px;
    background-color: var(--color-background);
    color: var(--color-text);
    font-size: 14px;
    line-height: 1.5;
    cursor: pointer;
    transition: all 0.2s ease;
    box-sizing: border-box;
  }

  .x-select__input:hover {
    border-color: var(--color-primary);
  }

  .x-select--open .x-select__input {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 2px var(--color-primary-background);
  }

  .x-select--disabled .x-select__input {
    cursor: not-allowed;
    background-color: var(--color-background-disabled);
    color: var(--color-text-disabled);
  }

  .x-select--disabled .x-select__input:hover {
    border-color: var(--color-border);
  }

  /* 尺寸变体 */
  .x-select--small .x-select__input {
    min-height: 20px;
    padding: 2px 24px 2px 6px;
    font-size: 11px;
  }

  .x-select--large .x-select__input {
    min-height: 40px;
    padding: 10px 36px 10px 16px;
    font-size: 16px;
  }

  /* 无边框变体 */
  .x-select--borderless .x-select__input {
    border: none;
    background-color: transparent;
  }

  .x-select--borderless .x-select__input:hover {
    background-color: var(--color-background-hover, rgba(0, 0, 0, 0.05));
    border: none;
  }

  .x-select--borderless.x-select--open .x-select__input {
    background-color: var(--color-background-hover, rgba(0, 0, 0, 0.05));
    box-shadow: none;
    border: none;
  }

  /* 显示值和占位符 */
  .x-select__value {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .x-select__placeholder {
    flex: 1;
    color: var(--color-text-placeholder);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* 清除按钮 */
  .x-select__clear {
    position: absolute;
    right: 24px;
    top: 50%;
    transform: translateY(-50%);
    width: 14px;
    height: 14px;
    border: none;
    background: none;
    color: var(--color-text-secondary);
    cursor: pointer;
    opacity: 0;
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .x-select__clear svg {
    width: 10px;
    height: 10px;
  }

  .x-select--clearable:hover .x-select__clear {
    opacity: 1;
  }

  .x-select__clear:hover {
    color: var(--color-text);
  }

  /* 下拉箭头 */
  .x-select__arrow {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    width: 14px;
    height: 14px;
    color: var(--color-text-secondary);
    transition: transform 0.2s ease;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .x-select__arrow svg {
    width: 12px;
    height: 12px;
  }

  .x-select__arrow--open {
    transform: translateY(-50%) rotate(180deg);
  }

  /* 下拉面板 */
  .x-select__dropdown {
    background-color: var(--color-background);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    overflow: hidden;
    animation: x-select-dropdown-enter 0.2s ease;
  }

  .x-select__dropdown--top {
    transform-origin: center bottom;
  }

  .x-select__dropdown--bottom {
    transform-origin: center top;
  }

  @keyframes x-select-dropdown-enter {
    from {
      opacity: 0;
      transform: scaleY(0.8);
    }
    to {
      opacity: 1;
      transform: scaleY(1);
    }
  }

  /* 搜索框 */
  .x-select__filter {
    padding: 8px;
    border-bottom: 1px solid var(--color-border);
  }

  .x-select__filter-input {
    width: 100%;
    padding: 6px 8px;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    background-color: var(--color-background);
    color: var(--color-text);
    font-size: 12px;
    outline: none;
    transition: border-color 0.2s ease;
  }

  .x-select__filter-input:focus {
    border-color: var(--color-primary);
  }

  /* 选项列表 */
  .x-select__options {
    max-height: 200px;
    overflow-y: auto;
    padding: 4px 0;
  }

  .x-select__options::-webkit-scrollbar {
    width: 6px;
  }

  .x-select__options::-webkit-scrollbar-track {
    background: transparent;
  }

  .x-select__options::-webkit-scrollbar-thumb {
    background-color: var(--color-border);
    border-radius: 3px;
  }

  .x-select__options::-webkit-scrollbar-thumb:hover {
    background-color: var(--color-text-secondary);
  }

  /* 选项 */
  .x-select__option {
    display: flex;
    align-items: center;
    padding: 6px 8px;
    cursor: pointer;
    transition: background-color 0.2s ease;
    min-height: 28px;
    box-sizing: border-box;
    font-size: 12px;
  }

  .x-select__option:hover,
  .x-select__option--highlighted {
    background-color: var(--color-primary-background, rgba(59, 130, 246, 0.1));
  }

  .x-select__option--selected {
    background-color: var(--color-primary-background, rgba(59, 130, 246, 0.1));
    color: var(--color-primary, #3b82f6);
    font-weight: 500;
  }

  .x-select__option--disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .x-select__option--disabled:hover {
    background-color: transparent;
  }

  /* 选项图标 */
  .x-select__option-icon {
    margin-right: 8px;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .x-select__option-icon svg {
    width: 14px;
    height: 14px;
  }

  /* 选项内容 */
  .x-select__option-content {
    flex: 1;
    min-width: 0;
  }

  .x-select__option-label {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }

  .x-select__option-description {
    display: block;
    font-size: 12px;
    color: var(--color-text-secondary);
    margin-top: 2px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* 选中标记 */
  .x-select__option-check {
    margin-left: 8px;
    width: 16px;
    height: 16px;
    color: var(--color-primary);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .x-select__option-check svg {
    width: 14px;
    height: 14px;
  }

  /* 无数据提示 */
  .x-select__no-data {
    padding: 16px 12px;
    text-align: center;
    color: var(--color-text-secondary);
    font-size: 12px;
  }

  /* 响应式调整 */
  @media (max-width: 768px) {
    .x-select__dropdown {
      max-width: calc(100vw - 32px);
    }

    .x-select__option {
      padding: 12px;
      min-height: 44px;
    }

    .x-select__option-icon {
      width: 20px;
      height: 20px;
    }

    .x-select__option-icon svg {
      width: 16px;
      height: 16px;
    }

    .x-select--borderless {
      width: 45%;
      max-width: none;
    }
  }
</style>
