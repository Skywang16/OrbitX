# X-UI 组件使用示例

这里提供了所有X-UI组件的详细使用示例和最佳实践。

## 🔘 XButton - 按钮组件

### 基础用法

```vue
<template>
  <div class="button-examples">
    <!-- 不同样式的按钮 -->
    <x-button variant="primary">主要按钮</x-button>
    <x-button variant="secondary">次要按钮</x-button>
    <x-button variant="danger">危险按钮</x-button>
    <x-button variant="ghost">幽灵按钮</x-button>
    <x-button variant="link">链接按钮</x-button>

    <!-- 不同尺寸的按钮 -->
    <x-button size="small">小按钮</x-button>
    <x-button size="medium">中按钮</x-button>
    <x-button size="large">大按钮</x-button>

    <!-- 状态按钮 -->
    <x-button :loading="isLoading" @click="handleAsyncAction">
      {{ isLoading ? '处理中...' : '异步操作' }}
    </x-button>
    <x-button disabled>禁用按钮</x-button>

    <!-- 形状按钮 -->
    <x-button round>圆角按钮</x-button>
    <x-button circle>圆</x-button>
    <x-button block>块级按钮</x-button>
  </div>
</template>

<script setup>
  import { ref } from 'vue'

  const isLoading = ref(false)

  const handleAsyncAction = async () => {
    isLoading.value = true
    await new Promise(resolve => setTimeout(resolve, 2000))
    isLoading.value = false
  }
</script>
```

### 带图标的按钮

```vue
<template>
  <div class="icon-button-examples">
    <!-- 带图标和文字 -->
    <x-button variant="primary">
      <template #icon>
        <svg width="16" height="16" viewBox="0 0 24 24">
          <path d="M12 5v14m7-7H5" stroke="currentColor" stroke-width="2" />
        </svg>
      </template>
      添加项目
    </x-button>

    <!-- 仅图标按钮 -->
    <x-button variant="ghost" circle>
      <template #icon>
        <svg width="16" height="16" viewBox="0 0 24 24">
          <path d="M6 18L18 6M6 6l12 12" stroke="currentColor" stroke-width="2" />
        </svg>
      </template>
    </x-button>
  </div>
</template>
```

## 🔄 XSwitch - 开关组件

### 基础用法

```vue
<template>
  <div class="switch-examples">
    <!-- 基础开关 -->
    <div class="switch-item">
      <x-switch v-model="basicSwitch" />
      <span>基础开关: {{ basicSwitch ? '开启' : '关闭' }}</span>
    </div>

    <!-- 不同尺寸 -->
    <div class="switch-item">
      <x-switch v-model="smallSwitch" size="small" />
      <span>小尺寸开关</span>
    </div>

    <div class="switch-item">
      <x-switch v-model="largeSwitch" size="large" />
      <span>大尺寸开关</span>
    </div>

    <!-- 加载状态 -->
    <div class="switch-item">
      <x-switch v-model="loadingSwitch" :loading="isSwitchLoading" @change="handleSwitchChange" />
      <span>异步开关</span>
    </div>

    <!-- 禁用状态 -->
    <div class="switch-item">
      <x-switch v-model="disabledSwitch" disabled />
      <span>禁用开关</span>
    </div>
  </div>
</template>

<script setup>
  import { ref } from 'vue'

  const basicSwitch = ref(false)
  const smallSwitch = ref(true)
  const largeSwitch = ref(false)
  const loadingSwitch = ref(false)
  const disabledSwitch = ref(true)
  const isSwitchLoading = ref(false)

  const handleSwitchChange = async value => {
    isSwitchLoading.value = true
    // 模拟异步操作
    await new Promise(resolve => setTimeout(resolve, 1000))
    isSwitchLoading.value = false
  }
</script>

<style scoped>
  .switch-item {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }
</style>
```

## 🔍 XSearchInput - 搜索输入框

### 基础用法

```vue
<template>
  <div class="search-examples">
    <!-- 基础搜索框 -->
    <x-search-input v-model="searchValue" placeholder="搜索内容..." @search="handleSearch" @clear="handleClear" />

    <!-- 带防抖的搜索框 -->
    <x-search-input
      v-model="debounceSearchValue"
      placeholder="防抖搜索 (300ms)..."
      :debounce="300"
      @search="handleDebounceSearch"
    />

    <!-- 不可清除的搜索框 -->
    <x-search-input v-model="noClearSearchValue" placeholder="不可清除的搜索框" :clearable="false" />

    <!-- 禁用状态 -->
    <x-search-input v-model="disabledSearchValue" placeholder="禁用状态" disabled />

    <!-- 搜索结果展示 -->
    <div v-if="searchResults.length > 0" class="search-results">
      <h4>搜索结果：</h4>
      <ul>
        <li v-for="result in searchResults" :key="result">{{ result }}</li>
      </ul>
    </div>
  </div>
</template>

<script setup>
  import { ref } from 'vue'

  const searchValue = ref('')
  const debounceSearchValue = ref('')
  const noClearSearchValue = ref('')
  const disabledSearchValue = ref('禁用的值')
  const searchResults = ref([])

  const mockData = ['苹果', '香蕉', '橙子', '葡萄', '草莓', '蓝莓', '樱桃']

  const handleSearch = value => {
    console.log('搜索:', value)
    searchResults.value = mockData.filter(item => item.includes(value))
  }

  const handleDebounceSearch = value => {
    console.log('防抖搜索:', value)
  }

  const handleClear = () => {
    console.log('清除搜索')
    searchResults.value = []
  }
</script>

<style scoped>
  .search-examples > * {
    margin-bottom: 16px;
  }

  .search-results {
    padding: 16px;
    background: var(--color-background-soft);
    border-radius: 6px;
  }

  .search-results ul {
    margin: 8px 0 0 0;
    padding-left: 20px;
  }
</style>
```

## 💬 XMessage - 消息提示

### 函数式调用

```vue
<template>
  <div class="message-examples">
    <h3>消息提示示例</h3>

    <div class="button-group">
      <x-button @click="showSuccess">成功消息</x-button>
      <x-button @click="showError">错误消息</x-button>
      <x-button @click="showWarning">警告消息</x-button>
      <x-button @click="showInfo">信息消息</x-button>
    </div>

    <div class="button-group">
      <x-button @click="showCustomMessage">自定义消息</x-button>
      <x-button @click="showPersistentMessage">持久消息</x-button>
      <x-button @click="closeAllMessages">关闭所有消息</x-button>
    </div>
  </div>
</template>

<script setup>
  import { createMessage } from '@/ui'

  const showSuccess = () => {
    createMessage.success('操作成功！')
  }

  const showError = () => {
    createMessage.error('操作失败，请重试')
  }

  const showWarning = () => {
    createMessage.warning('这是一个警告消息')
  }

  const showInfo = () => {
    createMessage.info('这是一条信息提示')
  }

  const showCustomMessage = () => {
    createMessage({
      message: '这是自定义消息',
      type: 'success',
      duration: 5000,
      closable: true,
    })
  }

  const showPersistentMessage = () => {
    const instance = createMessage({
      message: '这是一个持久消息，不会自动关闭',
      type: 'info',
      duration: 0,
      closable: true,
    })

    // 5秒后手动关闭
    setTimeout(() => {
      instance.close()
    }, 5000)
  }

  const closeAllMessages = () => {
    createMessage.closeAll()
  }
</script>

<style scoped>
  .button-group {
    display: flex;
    gap: 12px;
    margin-bottom: 16px;
  }
</style>
```

## 📋 XModal - 模态框

### 基础用法

```vue
<template>
  <div class="modal-examples">
    <h3>模态框示例</h3>

    <div class="button-group">
      <x-button @click="showBasicModal">基础模态框</x-button>
      <x-button @click="showCustomModal">自定义模态框</x-button>
      <x-button @click="showFormModal">表单模态框</x-button>
      <x-button @click="showFullModal">全屏模态框</x-button>
    </div>

    <!-- 基础模态框 -->
    <x-modal
      v-model:visible="basicModalVisible"
      title="基础模态框"
      show-footer
      @confirm="handleBasicConfirm"
      @cancel="basicModalVisible = false"
    >
      <p>这是一个基础的模态框示例。</p>
      <p>您可以在这里放置任何内容。</p>
    </x-modal>

    <!-- 自定义模态框 -->
    <x-modal
      v-model:visible="customModalVisible"
      title="自定义模态框"
      size="large"
      :show-cancel-button="false"
      confirm-text="知道了"
      @confirm="customModalVisible = false"
    >
      <div class="custom-content">
        <h4>自定义内容</h4>
        <p>这个模态框只有确认按钮，没有取消按钮。</p>
        <ul>
          <li>支持自定义尺寸</li>
          <li>支持自定义按钮文字</li>
          <li>支持隐藏特定按钮</li>
        </ul>
      </div>
    </x-modal>

    <!-- 表单模态框 -->
    <x-modal
      v-model:visible="formModalVisible"
      title="表单模态框"
      show-footer
      :loading="formLoading"
      loading-text="提交中..."
      @confirm="handleFormSubmit"
      @cancel="handleFormCancel"
    >
      <form @submit.prevent="handleFormSubmit">
        <div class="form-group">
          <label>姓名：</label>
          <input v-model="formData.name" type="text" required />
        </div>
        <div class="form-group">
          <label>邮箱：</label>
          <input v-model="formData.email" type="email" required />
        </div>
      </form>
    </x-modal>

    <!-- 全屏模态框 -->
    <x-modal
      v-model:visible="fullModalVisible"
      title="全屏模态框"
      size="full"
      show-footer
      @confirm="fullModalVisible = false"
      @cancel="fullModalVisible = false"
    >
      <div class="full-content">
        <h4>全屏模态框内容</h4>
        <p>这是一个全屏显示的模态框，适合展示大量内容。</p>
        <div class="content-grid">
          <div v-for="i in 12" :key="i" class="grid-item">内容块 {{ i }}</div>
        </div>
      </div>
    </x-modal>
  </div>
</template>

<script setup>
  import { ref, reactive } from 'vue'
  import { createMessage } from '@/ui'

  const basicModalVisible = ref(false)
  const customModalVisible = ref(false)
  const formModalVisible = ref(false)
  const fullModalVisible = ref(false)
  const formLoading = ref(false)

  const formData = reactive({
    name: '',
    email: '',
  })

  const showBasicModal = () => {
    basicModalVisible.value = true
  }

  const showCustomModal = () => {
    customModalVisible.value = true
  }

  const showFormModal = () => {
    formModalVisible.value = true
  }

  const showFullModal = () => {
    fullModalVisible.value = true
  }

  const handleBasicConfirm = () => {
    createMessage.success('确认操作成功！')
    basicModalVisible.value = false
  }

  const handleFormSubmit = async () => {
    if (!formData.name || !formData.email) {
      createMessage.error('请填写完整信息')
      return
    }

    formLoading.value = true

    // 模拟提交
    await new Promise(resolve => setTimeout(resolve, 2000))

    formLoading.value = false
    formModalVisible.value = false
    createMessage.success('表单提交成功！')

    // 重置表单
    formData.name = ''
    formData.email = ''
  }

  const handleFormCancel = () => {
    formModalVisible.value = false
    formData.name = ''
    formData.email = ''
  }
</script>

<style scoped>
  .button-group {
    display: flex;
    gap: 12px;
    margin-bottom: 24px;
  }

  .custom-content h4 {
    margin-top: 0;
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    margin-bottom: 4px;
    font-weight: 500;
  }

  .form-group input {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid var(--border-color);
    border-radius: 4px;
  }

  .content-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 16px;
    margin-top: 24px;
  }

  .grid-item {
    padding: 24px;
    background: var(--color-background-soft);
    border-radius: 8px;
    text-align: center;
  }
</style>
```

## 🎯 XPopover - 弹出框

### 基础用法

```vue
<template>
  <div class="popover-examples">
    <h3>弹出框示例</h3>

    <!-- 基础弹出框 -->
    <x-popover content="这是一个基础的弹出框提示">
      <x-button>悬停显示</x-button>
    </x-popover>

    <!-- 菜单弹出框 -->
    <x-popover v-model="menuVisible" trigger="click" :menu-items="menuItems" @menu-item-click="handleMenuClick">
      <template #trigger>
        <x-button>点击菜单</x-button>
      </template>
    </x-popover>

    <!-- 不同位置的弹出框 -->
    <div class="placement-examples">
      <x-popover v-for="placement in placements" :key="placement" :content="`${placement} 位置`" :placement="placement">
        <x-button size="small">{{ placement }}</x-button>
      </x-popover>
    </div>
  </div>
</template>

<script setup>
  import { ref } from 'vue'
  import { createMessage } from '@/ui'

  const menuVisible = ref(false)

  const menuItems = [
    { label: '编辑', value: 'edit', icon: '✏️' },
    { label: '复制', value: 'copy', icon: '📋' },
    { label: '删除', value: 'delete', icon: '🗑️', disabled: false },
    { label: '禁用项', value: 'disabled', disabled: true },
  ]

  const placements = [
    'top',
    'top-start',
    'top-end',
    'bottom',
    'bottom-start',
    'bottom-end',
    'left',
    'left-start',
    'left-end',
    'right',
    'right-start',
    'right-end',
  ]

  const handleMenuClick = item => {
    createMessage.info(`点击了: ${item.label}`)
    menuVisible.value = false
  }
</script>

<style scoped>
  .popover-examples > * {
    margin-bottom: 24px;
  }

  .placement-examples {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    max-width: 600px;
  }
</style>
```

## 🔔 确认对话框

### 函数式调用

```vue
<template>
  <div class="confirm-examples">
    <h3>确认对话框示例</h3>

    <div class="button-group">
      <x-button @click="showBasicConfirm">基础确认</x-button>
      <x-button @click="showWarningConfirm">警告确认</x-button>
      <x-button @click="showDangerConfirm">危险确认</x-button>
      <x-button @click="showCustomConfirm">自定义确认</x-button>
    </div>
  </div>
</template>

<script setup>
  import { confirm, confirmWarning, confirmDanger, createConfirm } from '@/ui'
  import { createMessage } from '@/ui'

  const showBasicConfirm = async () => {
    const result = await confirm('确定要执行这个操作吗？')
    createMessage.info(`用户选择: ${result ? '确定' : '取消'}`)
  }

  const showWarningConfirm = async () => {
    const result = await confirmWarning('这个操作可能会影响系统性能，确定继续吗？')
    if (result) {
      createMessage.success('操作已执行')
    }
  }

  const showDangerConfirm = async () => {
    const result = await confirmDanger('此操作不可撤销，确定要删除所有数据吗？')
    if (result) {
      createMessage.success('数据已删除')
    }
  }

  const showCustomConfirm = async () => {
    const result = await createConfirm({
      title: '自定义确认对话框',
      message: '这是一个完全自定义的确认对话框，您可以自定义标题、按钮文字等。',
      confirmText: '同意',
      cancelText: '拒绝',
      type: 'warning',
    })

    createMessage.info(`用户选择: ${result ? '同意' : '拒绝'}`)
  }
</script>

<style scoped>
  .button-group {
    display: flex;
    gap: 12px;
  }
</style>
```

## 🎨 主题集成示例

```vue
<template>
  <div class="theme-examples">
    <h3>主题集成示例</h3>

    <div class="theme-controls">
      <x-button @click="toggleTheme">切换到 {{ isDark ? '浅色' : '深色' }} 主题</x-button>
    </div>

    <div class="component-showcase">
      <x-button variant="primary">主要按钮</x-button>
      <x-switch v-model="switchValue" />
      <x-search-input v-model="searchValue" placeholder="搜索..." />
    </div>
  </div>
</template>

<script setup>
  import { ref } from 'vue'

  const isDark = ref(false)
  const switchValue = ref(true)
  const searchValue = ref('')

  const toggleTheme = () => {
    isDark.value = !isDark.value
    document.documentElement.setAttribute('data-theme', isDark.value ? 'dark' : 'light')
  }
</script>

<style scoped>
  .theme-controls {
    margin-bottom: 24px;
  }

  .component-showcase {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 24px;
    background: var(--color-background-soft);
    border-radius: 8px;
  }
</style>
```

## 📱 响应式设计示例

```vue
<template>
  <div class="responsive-examples">
    <h3>响应式设计示例</h3>

    <!-- 响应式按钮组 -->
    <div class="responsive-buttons">
      <x-button v-for="i in 6" :key="i" variant="primary">按钮 {{ i }}</x-button>
    </div>

    <!-- 响应式表单 -->
    <div class="responsive-form">
      <x-search-input v-model="searchValue" placeholder="响应式搜索框" />
      <x-button variant="primary" block>块级按钮</x-button>
    </div>
  </div>
</template>

<script setup>
  import { ref } from 'vue'

  const searchValue = ref('')
</script>

<style scoped>
  .responsive-buttons {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 24px;
  }

  .responsive-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    max-width: 400px;
  }

  @media (max-width: 768px) {
    .responsive-buttons {
      flex-direction: column;
    }

    .responsive-buttons > * {
      width: 100%;
    }
  }
</style>
```

---

## 💡 最佳实践

### 1. 性能优化

- 使用函数式API（如 `createMessage`）而不是组件形式
- 合理使用防抖功能避免频繁触发
- 及时清理不需要的消息实例

### 2. 用户体验

- 为异步操作提供加载状态
- 使用合适的消息类型和确认对话框
- 保持界面元素的一致性

### 3. 无障碍访问

- 为按钮提供合适的 `title` 属性
- 确保键盘导航的可用性
- 使用语义化的HTML结构

### 4. 主题适配

- 使用CSS变量确保主题一致性
- 测试不同主题下的显示效果
- 避免硬编码颜色值
