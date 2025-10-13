# 前端架构设计规范

## 设计哲学

好的前端架构应该：**简单、明确、统一、可预测**

## 核心原则

### 1. 前后端协同设计

**原则**：前后端不是独立的两个系统，而是协同工作的整体。

#### 职责划分

- **后端职责**：数据处理、业务逻辑、顺序保证、时间标记
- **前端职责**：数据展示、用户交互、状态管理

#### 数据流向

```
后端（数据源）→ API约定 → 前端（消费者）
```

**关键**：前端应该信任后端提供的数据，不要重复处理。

#### 示例

```typescript
// ❌ 前端不信任后端
const sortedList = apiData.sort((a, b) => a.order - b.order) // 后端应该保证顺序
const formatted = formatDate(Date.now()) // 应该用后端时间

// ✅ 信任后端
const list = apiData // 直接使用，后端保证顺序
const formatted = formatDate(item.timestamp) // 使用后端提供的时间
```

### 2. 状态管理要清晰

**原则**：应用的状态管理要有明确的结构和流向。

#### 状态分层

- **全局状态** - 跨组件共享（Pinia Store）
- **页面状态** - 页面级别（setup/data）
- **组件状态** - 组件内部（ref/reactive）

#### 示例

```typescript
// ✅ 清晰的状态管理
// 全局：用户信息
const userStore = useUserStore()

// 页面：列表数据
const messages = ref<Message[]>([])

// 组件：UI状态
const isExpanded = ref(false)
```

```typescript
// ❌ 混乱的状态
// 所有状态都放全局
const globalStore = useGlobalStore()
globalStore.isDialogOpen = true
globalStore.currentTab = 'messages'
globalStore.tempInputValue = 'hello'
```

**原则**：同一份数据只有一个权威来源。

#### 数据源统一

**区分业务数据和UI状态**：

**业务数据**（需要前后端同步）：

- ✅ 时间戳由后端提供
- ✅ ID由后端生成
- ✅ 排序由后端完成

**前端UI状态**（本地功能）：

- ✅ 前端生成时间戳（如动画时间、本地缓存时间）
- ✅ 前端生成临时ID（如pending状态的临时标识）
- ✅ 前端排序（如UI展示需要）

#### 示例

````typescript
// ❌ 业务数据混乱
const message = {
  id: Date.now(),              // 业务ID应该由后端生成
  content: '...',
  createdAt: Date.now()        // 业务时间应该由后端提供
}

// ✅ 业务数据统一
const message = await api.createMessage(content)  // 后端返回完整数据
// message.id 和 message.createdAt 都是后端提供的

// ✅ 前端UI状态
const uiState = {
  lastClickTime: Date.now(),   // 前端功能，可以自己生成
  animationStartTime: Date.now(),
  tempId: `temp-${Date.now()}` // 临时ID，提交后会被后端ID替换
}

#### 一致性原则

**原则**：同一个数据在不同场景下应该用同样的方式处理。

```typescript
// ✅ 统一处理函数
function processData(data: RawData): ProcessedData {
  // 同一逻辑处理不同来源的数据
}

// 实时数据
const processed = processData(liveData)

// 历史数据
const processed = processData(historyData)

// 缓存数据
const processed = processData(cachedData)
````

### 4. 简单明确的实现

**原则**：代码应该一眼看懂在做什么，不需要注释也能理解。

#### 避免过度防御

```typescript
// ❌ 过度防御，逻辑晦涩
const shouldSkipAppend = !delta || (streamDone && delta === existing.content)
if (!shouldSkipAppend) {
  existing.content += delta
}

// ✅ 简单明确
if (!delta) return
existing.content += delta
```

#### 相信约定

如果前后端已经约定好行为，就不要在代码中再次验证：

```typescript
// ❌ 不信任后端，反复检查
if (data && data.items && Array.isArray(data.items) && data.items.length > 0) {
  // 处理
}

// ✅ 相信约定，简单处理
if (data.items.length > 0) {
  // 处理
}
```

### 5. 注释规范

#### 保留的注释

- 函数/类的功能说明（简洁）
- 复杂算法的思路说明
- 非常规操作的原因说明

```typescript
/**
 * 批量合并步骤
 */
export function mergeBatchSteps(steps: Step[]): Step[] {
  // 实现...
}
```

#### 删除的注释

- 代码本身已经表达清楚的注释
- 实现过程中的调试注释
- 过时的注释

```typescript
// ❌ 多余的注释
// 循环遍历所有步骤
for (const step of steps) {
  // 如果是工具使用步骤
  if (step.type === 'tool_use') {
    // 添加到结果数组
    result.push(step)
  }
}

// ✅ 代码自解释
for (const step of steps) {
  if (step.type === 'tool_use') {
    result.push(step)
  }
}
```

## 设计检查清单

设计新功能时，检查以下几点：

### 数据流设计

- [ ] 数据的权威来源明确
- [ ] 前后端职责划分清晰
- [ ] 实时和历史逻辑统一

### 顺序性设计

- [ ] 后端能保证顺序输出
- [ ] 前端利用顺序性简化逻辑
- [ ] 避免不必要的查找操作

### 代码质量

- [ ] 逻辑简单明确，一眼看懂
- [ ] 不包含过度防御的代码
- [ ] 注释简洁必要
- [ ] 变量命名清晰

## 重构指南

### 识别需要重构的代码

**信号1：前后端逻辑不一致**

```typescript
// 实时渲染：直接修改
lastStep.content += delta

// 历史渲染：查找后修改
const step = steps.find(s => s.id === targetId)
step.content = fullContent
```

→ 应该统一为同一种逻辑

**信号2：复杂的查找逻辑**

```typescript
const targetStep = steps.find(s => s.metadata?.streamId === streamId && s.type === stepType)
```

→ 考虑让后端保证顺序，改用`lastStep`

**信号3：过度的null检查**

```typescript
if (data && data.field && data.field.subfield) {
  // 多层检查
}
```

→ 检查是否真的需要，或者改进后端返回格式

### 重构步骤

1. **明确数据流**：画出数据从产生到消费的完整流程
2. **统一时间源**：检查所有timestamp的来源
3. **简化查找**：将find/filter改为顺序处理
4. **清理注释**：删除解释性注释，保留必要说明
5. **测试验证**：确保实时和历史渲染一致

## 反模式总结

| 反模式             | 问题       | 解决方案         |
| ------------------ | ---------- | ---------------- |
| 前端重新排序       | 不信任后端 | 后端保证顺序     |
| 前端生成时间戳     | 时间不一致 | 统一使用后端时间 |
| 通过ID查找         | 逻辑复杂   | 利用顺序性       |
| 实时vs历史不同逻辑 | 难以维护   | 统一处理函数     |
| 过度防御           | 代码晦涩   | 相信约定         |
| 解释性注释         | 代码冗余   | 让代码自解释     |

## 最佳实践案例

### 案例1：列表实时更新

**需求**：聊天消息列表实时添加新消息

**✅ 好的设计**：

```typescript
// 后端：保证消息按时间顺序发送

// 前端：顺序添加
const handleNewMessage = (message: Message) => {
  messages.push(message) // 简单直接
}
```

**❌ 不好的设计**：

```typescript
// 前端自己排序
const handleNewMessage = (message: Message) => {
  messages.push(message)
  messages.sort((a, b) => a.timestamp - b.timestamp) // 多余
}
```

### 案例2：状态流转

**需求**：订单状态从pending → processing → completed

**✅ 好的设计**：

```typescript
// 后端：保证状态事件顺序

// 前端：直接更新当前订单
const handleStatusChange = (newStatus: Status) => {
  currentOrder.status = newStatus
  currentOrder.updatedAt = newStatus.timestamp
}
```

### 案例3：表单数据处理

**需求**：表单提交后显示结果

**✅ 好的设计**：

```typescript
// 后端：返回完整的处理结果（包含timestamp, id等）

// 前端：直接使用
const handleSubmit = async () => {
  const result = await api.submit(formData)
  // 使用后端返回的数据，不自己加工
  showResult(result)
}
```

### 6. 不要重复造轮子

**原则**：在实现新功能前，先检查项目中是否已有可用的工具函数或第三方库。

#### 检查顺序

1. **查看工具清单** - 查看 [utility-functions.md](./utility-functions.md) 文档
2. **考虑新增** - 确实没有才考虑新写或引入新库

#### 实际案例

**场景**：需要格式化时间显示

```typescript
// ❌ 自己写一个
function formatTime(date: Date): string {
  const hours = date.getHours().toString().padStart(2, '0')
  const minutes = date.getMinutes().toString().padStart(2, '0')
  return `${hours}:${minutes}`
}

// ✅ 使用项目已有的工具
import { formatTime } from '@/utils/dateFormatter'
const formatted = formatTime(date)
```

#### 为什么重要

1. **代码一致性** - 同样的功能用同样的实现
2. **减少维护** - 一个bug只需修复一次
3. **避免重复** - 不浪费时间重复实现
4. **利用经验** - 现有工具可能已经处理了边界情况

#### 如何避免

开发前必看 [utility-functions.md](./utility-functions.md) 文档，代码审查时检查是否有现成工具。

## 总结

好的架构设计应该是：

1. **简单** - 一眼看懂在做什么
2. **明确** - 职责清晰，不模糊
3. **统一** - 同样的数据用同样的处理
4. **顺序** - 利用顺序性，避免查找
5. **信任** - 前后端相互信任约定
6. **复用** - 先找轮子，再造轮子

记住：**复杂的代码往往源于不清晰的设计**。如果代码写得很复杂，先检查设计是否有问题。
