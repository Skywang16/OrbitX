# 函数定义规范

## 核心原则

**统一使用箭头函数语法定义所有函数**

## 为什么使用箭头函数？

### 1. **自动绑定 this 上下文**
箭头函数不会创建自己的 `this`，它会捕获其所在上下文的 `this` 值，避免了传统函数中 `this` 指向不明确的问题。

### 2. **代码风格统一**
在整个项目中使用统一的函数定义风格，提高代码可读性和维护性。

### 3. **更简洁的语法**
箭头函数语法更加简洁，特别是在单行函数和回调函数中。

### 4. **现代 JavaScript/TypeScript 最佳实践**
箭头函数是 ES6+ 的标准特性，是现代 JavaScript 开发的推荐做法。

## 规范详解

### 1. 类方法定义

#### ❌ 错误示例（传统方法）

```typescript
export class UserApi {
  async getUser(id: string): Promise<User> {
    return await invoke('get_user', { id })
  }

  async updateUser(id: string, data: UserData): Promise<void> {
    await invoke('update_user', { id, data })
  }
}
```

#### ✅ 正确示例（箭头函数属性）

```typescript
export class UserApi {
  getUser = async (id: string): Promise<User> => {
    return await invoke('get_user', { id })
  }

  updateUser = async (id: string, data: UserData): Promise<void> => {
    await invoke('update_user', { id, data })
  }
}
```

### 2. 对象方法定义

#### ❌ 错误示例（传统方法简写）

```typescript
const handlers = {
  async handleClick(event: Event): Promise<void> {
    // 处理点击
  },

  formatData(data: RawData): FormattedData {
    return transform(data)
  }
}
```

#### ✅ 正确示例（箭头函数）

```typescript
const handlers = {
  handleClick: async (event: Event): Promise<void> => {
    // 处理点击
  },

  formatData: (data: RawData): FormattedData => {
    return transform(data)
  }
}
```

### 3. 独立函数定义

#### ❌ 错误示例（function 关键字）

```typescript
function calculateTotal(items: Item[]): number {
  return items.reduce((sum, item) => sum + item.price, 0)
}

async function fetchData(url: string): Promise<Data> {
  return await fetch(url).then(res => res.json())
}
```

#### ✅ 正确示例（const + 箭头函数）

```typescript
const calculateTotal = (items: Item[]): number => {
  return items.reduce((sum, item) => sum + item.price, 0)
}

const fetchData = async (url: string): Promise<Data> => {
  return await fetch(url).then(res => res.json())
}
```

### 4. Vue Composition API

#### ✅ 正确示例（已经是箭头函数）

```typescript
// Vue 3 Composition API 中的函数定义
const handleSubmit = async () => {
  // 处理提交
}

const formatDate = (date: Date): string => {
  return date.toLocaleDateString()
}

// computed 和 watch 中的回调也使用箭头函数
const fullName = computed(() => {
  return `${firstName.value} ${lastName.value}`
})

watch(
  () => props.value,
  (newVal) => {
    // 处理变化
  }
)
```

### 5. 泛型函数

#### ✅ 正确示例

```typescript
// 类方法中的泛型
export class StorageApi {
  getConfig = async <S extends ConfigSection>(section: S): Promise<ConfigSectionMap[S]> => {
    return await invoke('storage_get_config', { section })
  }
}

// 独立泛型函数
const identity = <T>(value: T): T => {
  return value
}

const mapArray = <T, U>(arr: T[], fn: (item: T) => U): U[] => {
  return arr.map(fn)
}
```

### 6. 私有方法

#### ✅ 正确示例

```typescript
export class ThemeManager {
  // 公共方法
  switchTheme = async (themeName: string): Promise<void> => {
    await this.validateTheme(themeName)
    await this.applyTheme(themeName)
  }

  // 私有方法也使用箭头函数
  private validateTheme = (themeName: string): boolean => {
    return this.availableThemes.includes(themeName)
  }

  private applyTheme = async (themeName: string): Promise<void> => {
    await invoke('apply_theme', { themeName })
  }
}
```

## 特殊情况

### 1. 构造函数

构造函数必须使用传统语法，这是 JavaScript/TypeScript 的语言限制：

```typescript
export class MyClass {
  constructor(private config: Config) {
    // 构造函数使用传统语法
  }

  // 其他方法使用箭头函数
  init = async (): Promise<void> => {
    // 初始化逻辑
  }
}
```

### 2. Getter 和 Setter

Getter 和 Setter 使用传统语法：

```typescript
export class ConfigManager {
  private _config: Config | null = null

  get config(): Config | null {
    return this._config
  }

  set config(value: Config | null) {
    this._config = value
  }

  // 普通方法使用箭头函数
  loadConfig = async (): Promise<void> => {
    this._config = await invoke('load_config')
  }
}
```

### 3. 静态方法

静态方法也应使用箭头函数属性：

```typescript
export class APIClient {
  private static instance: APIClient

  // 静态方法使用箭头函数
  static getInstance = (): APIClient => {
    if (!APIClient.instance) {
      APIClient.instance = new APIClient()
    }
    return APIClient.instance
  }
}
```

## 迁移指南

### 从传统函数迁移到箭头函数

1. **类方法**：
   ```typescript
   // 之前
   async methodName(param: Type): Promise<Result> { }
   
   // 之后
   methodName = async (param: Type): Promise<Result> => { }
   ```

2. **对象方法**：
   ```typescript
   // 之前
   methodName(param: Type): Result { }
   
   // 之后
   methodName: (param: Type): Result => { }
   ```

3. **独立函数**：
   ```typescript
   // 之前
   function functionName(param: Type): Result { }
   
   // 之后
   const functionName = (param: Type): Result => { }
   ```

## 代码审查检查清单

在代码审查时，检查以下几点：

- [ ] 所有类方法都使用箭头函数属性
- [ ] 所有对象方法都使用箭头函数
- [ ] 所有独立函数都使用 const + 箭头函数
- [ ] 构造函数、getter/setter 使用传统语法（这是正确的）
- [ ] 没有使用 `function` 关键字定义函数（除了构造函数）
- [ ] 泛型函数的类型参数位置正确

## 常见问题

### Q: 为什么不使用 function 关键字？

A: 为了保持代码风格统一，并利用箭头函数的 `this` 绑定特性。在类方法中，箭头函数会自动绑定实例的 `this`，避免了 `this` 丢失的问题。

### Q: 箭头函数会影响性能吗？

A: 在现代 JavaScript 引擎中，箭头函数和传统函数的性能差异可以忽略不计。代码可读性和维护性的提升远大于微小的性能差异。

### Q: 什么时候必须使用传统函数？

A: 只有以下几种情况：
- 构造函数 `constructor`
- Getter 和 Setter
- 需要使用 `arguments` 对象的场景（但应该使用剩余参数 `...args` 代替）

### Q: 如何处理需要 hoisting 的函数？

A: 箭头函数不会被提升（hoisting），如果需要在定义前调用函数，应该调整代码结构，将函数定义移到调用之前，或者重新考虑代码组织方式。

## 总结

统一使用箭头函数语法是 OrbitX 项目的代码风格标准。这不仅提高了代码的一致性和可读性，还避免了 `this` 绑定相关的常见问题。在编写新代码或重构现有代码时，请遵循本规范。
