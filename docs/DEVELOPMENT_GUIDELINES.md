# 开发规范指南

## 概述

本文档为AI助手和开发者提供TermX项目的开发规范，确保代码质量、架构一致性和最佳实践的遵循。

## 🎯 核心原则

### 1. 代码质量优先

- 所有代码必须通过Clippy检查（无警告）
- 使用统一的代码格式化标准
- 保持高测试覆盖率
- 遵循Rust最佳实践

### 2. 架构一致性

- 遵循现有模块结构
- 使用统一的错误处理体系
- 保持API设计的一致性
- 避免过度抽象

### 3. 渐进式改进

- 在现有架构基础上优化
- 避免破坏性变更
- 保持向后兼容性
- 优先解决实际问题

## 📁 项目结构规范

### 后端结构 (src-tauri/src/)

```
src/
├── ai/                 # AI集成模块
│   ├── mod.rs         # 模块导出
│   ├── commands.rs    # Tauri命令
│   ├── client.rs      # AI客户端
│   ├── cache.rs       # 缓存管理
│   ├── config.rs      # 配置管理
│   ├── error.rs       # 错误定义
│   └── types.rs       # 类型定义
├── mux/               # 终端多路复用器
├── completion/        # 补全功能
├── window/           # 窗口管理
├── shell/            # Shell管理
├── utils/            # 工具模块
│   ├── error.rs      # 统一错误处理
│   ├── logging.rs    # 日志系统
│   └── mod.rs        # 模块导出
└── commands/         # Tauri命令导出
```

### 前端结构 (src/)

```
src/
├── api/              # API接口层
├── components/       # Vue组件
├── stores/          # Pinia状态管理
├── views/           # 页面视图
├── ui/              # 组件库
├── utils/           # 工具函数
├── types/           # TypeScript类型
├── constants/       # 常量定义
└── styles/          # 样式文件
```

## 🔧 开发规范

### Rust后端开发规范

#### 1. 模块组织

```rust
// 每个模块推荐包含的文件
mod.rs          // 模块导出和公共接口
commands.rs     // Tauri命令定义（如果需要）
types.rs        // 数据类型定义
// 注意：不再使用独立的 error.rs 文件，统一使用 anyhow
```

#### 2. 错误处理统一（基于Rust最佳实践）

```rust
// 使用 anyhow 作为统一错误处理（应用程序最佳实践）
use anyhow::{Context, Result as AppResult, anyhow, bail};

// 统一类型别名
pub type AppResult<T> = anyhow::Result<T>;
pub type AppError = anyhow::Error;

// 基本用法示例
pub fn some_operation() -> AppResult<String> {
    do_something()
        .context("操作失败，请检查输入参数")?;
    Ok("success".to_string())
}

// 创建自定义错误
pub fn validate_input(input: &str) -> AppResult<()> {
    if input.is_empty() {
        bail!("输入不能为空");
    }
    Ok(())
}

// 添加上下文信息
pub fn read_config_file(path: &Path) -> AppResult<Config> {
    std::fs::read_to_string(path)
        .with_context(|| format!("无法读取配置文件: {}", path.display()))?
        .parse()
        .context("配置文件格式错误")
}
```

#### 3. Tauri命令规范

```rust
// 命令函数签名
#[tauri::command]
pub async fn command_name(
    param1: Type1,
    param2: Type2,
    state: tauri::State<'_, SomeState>,
) -> AppResult<ReturnType> {
    // 实现
}

// 序列化配置
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SomeStruct {
    field_name: String,
}
```

#### 4. 测试规范

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[tokio::test]
    async fn test_function_name() {
        // 测试实现
    }
}
```

### TypeScript前端开发规范

#### 1. API接口定义

```typescript
// 使用统一的请求函数
import { invoke } from '@/utils/request'

// 接口函数示例
export async function someApiCall(params: SomeParams): Promise<SomeResponse> {
  return invoke('command_name', params)
}
```

#### 2. 类型定义

```typescript
// 与后端保持一致的类型定义
export interface SomeType {
  fieldName: string // camelCase格式
  anotherField: number
}
```

#### 3. 组件规范

```vue
<script setup lang="ts">
  // 使用组合式API
  import { ref, computed } from 'vue'

  // 类型定义
  interface Props {
    title: string
  }

  const props = defineProps<Props>()
</script>
```

## 🛠️ 代码质量工具

### 1. 自动化检查

```bash
# 运行完整的代码质量检查
./scripts/code-quality-check.sh

# 单独运行各项检查
./scripts/code-quality-check.sh --format-only
./scripts/code-quality-check.sh --clippy-only
./scripts/code-quality-check.sh --test-only
```

### 2. Pre-commit钩子

项目已配置pre-commit钩子，会自动运行：

- 代码格式化检查
- Clippy静态分析
- 测试执行

### 3. 测试覆盖率

```bash
# 生成测试覆盖率报告
cd src-tauri
cargo tarpaulin --config tarpaulin.toml
```

## 📋 开发检查清单

### 新功能开发前

- [ ] 阅读相关模块的现有代码
- [ ] 了解现有的错误处理模式
- [ ] 检查是否有类似功能可以复用
- [ ] 确认API设计与现有模式一致

### 代码编写时

- [ ] 使用统一的错误处理类型
- [ ] 遵循现有的命名约定
- [ ] 添加适当的文档注释
- [ ] 保持函数复杂度合理

### 代码提交前

- [ ] 运行`cargo fmt`格式化代码
- [ ] 运行`cargo clippy`检查代码质量
- [ ] 运行`cargo test`确保测试通过
- [ ] 添加或更新相关测试
- [ ] 更新文档（如需要）

### 代码审查时

- [ ] 检查是否遵循项目规范
- [ ] 验证错误处理的一致性
- [ ] 确认测试覆盖充分
- [ ] 检查性能影响

## 🚫 常见错误避免

### 1. 不要做的事情

- ❌ 创建自定义错误类型（统一使用 anyhow）
- ❌ 使用 thiserror（应用程序应使用 anyhow）
- ❌ 直接使用panic!（使用Result返回错误）
- ❌ 忽略Clippy警告
- ❌ 跳过测试编写
- ❌ 破坏现有API兼容性

### 2. 推荐做法

- ✅ 统一使用 anyhow::Result<T> 作为返回类型
- ✅ 使用 .context() 添加错误上下文
- ✅ 使用 bail! 宏创建简单错误
- ✅ 编写充分的测试
- ✅ 添加清晰的文档注释
- ✅ 保持代码简洁明了

## 📚 参考资源

### 项目文档

- [后端API标准](./BACKEND_API_STANDARDS.md)
- [前端API标准](./FRONTEND_API_STANDARDS.md)
- [架构文档](../src-tauri/ARCHITECTURE.md)

### 配置文件

- [Clippy配置](../src-tauri/.clippy.toml)
- [测试覆盖率配置](../src-tauri/tarpaulin.toml)
- [Tauri配置](../src-tauri/tauri.conf.json)

### 工具脚本

- [代码质量检查](../scripts/code-quality-check.sh)
- [测试覆盖率生成](../src-tauri/scripts/generate_coverage.sh)

## 🔄 持续改进

### 定期检查

- 每月运行依赖安全检查：`cargo audit`
- 定期更新依赖版本
- 监控代码质量指标
- 收集开发者反馈

### 规范更新

- 根据项目发展更新规范
- 记录重要的设计决策
- 分享最佳实践经验
- 持续优化开发流程

## 🎨 代码风格指南

### Rust代码风格

#### 1. 命名约定

```rust
// 模块名：snake_case
mod terminal_mux;

// 结构体：PascalCase
struct TerminalConfig;

// 函数名：snake_case
fn create_terminal() -> Result<Terminal, AppError>;

// 常量：SCREAMING_SNAKE_CASE
const MAX_TERMINALS: usize = 100;

// 变量：snake_case
let terminal_id = generate_id();
```

#### 2. 文档注释

````rust
/// 创建新的终端实例
///
/// # 参数
/// - `config`: 终端配置
/// - `size`: 终端尺寸
///
/// # 返回值
/// 返回创建的终端实例或错误
///
/// # 错误
/// - `AppError::Terminal`: 当终端创建失败时
///
/// # 示例
/// ```rust
/// let terminal = create_terminal(config, size)?;
/// ```
pub fn create_terminal(config: TerminalConfig, size: PtySize) -> AppResult<Terminal> {
    // 实现
}
````

#### 3. 错误处理模式

```rust
// 推荐：使用?操作符
pub fn process_data() -> AppResult<ProcessedData> {
    let raw_data = fetch_data()?;
    let validated_data = validate_data(raw_data)?;
    Ok(transform_data(validated_data)?)
}

// 推荐：提供上下文信息
pub fn read_config_file(path: &Path) -> AppResult<Config> {
    std::fs::read_to_string(path)
        .map_err(|e| AppError::Io(format!("无法读取配置文件 {}: {}", path.display(), e)))?
        .parse()
        .map_err(|e| AppError::Configuration(format!("配置文件格式错误: {}", e)))
}
```

### TypeScript代码风格

#### 1. 命名约定

```typescript
// 接口：PascalCase
interface TerminalConfig {
  shellPath: string
  workingDirectory: string
}

// 类型别名：PascalCase
type TerminalId = string

// 函数：camelCase
function createTerminal(config: TerminalConfig): Promise<Terminal>

// 变量：camelCase
const terminalId = generateId()

// 常量：SCREAMING_SNAKE_CASE
const MAX_TERMINALS = 100
```

#### 2. 组件组织

```vue
<template>
  <!-- 模板内容 -->
</template>

<script setup lang="ts">
  // 1. 导入
  import { ref, computed, onMounted } from 'vue'
  import type { TerminalConfig } from '@/types'

  // 2. 类型定义
  interface Props {
    config: TerminalConfig
  }

  interface Emits {
    (e: 'update', value: string): void
  }

  // 3. Props和Emits
  const props = defineProps<Props>()
  const emit = defineEmits<Emits>()

  // 4. 响应式数据
  const isLoading = ref(false)
  const terminalData = ref<string>('')

  // 5. 计算属性
  const formattedData = computed(() => {
    return terminalData.value.trim()
  })

  // 6. 方法
  const handleUpdate = (value: string) => {
    terminalData.value = value
    emit('update', value)
  }

  // 7. 生命周期
  onMounted(() => {
    // 初始化逻辑
  })
</script>

<style scoped>
  /* 样式 */
</style>
```

## 🧪 测试规范详解

### 1. 单元测试结构

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    // 测试数据准备
    fn setup_test_data() -> TestData {
        TestData::new()
    }

    // 成功场景测试
    #[tokio::test]
    async fn test_create_terminal_success() {
        let config = setup_test_data().terminal_config();
        let result = create_terminal(config).await;

        assert!(result.is_ok());
        let terminal = result.unwrap();
        assert_eq!(terminal.status(), TerminalStatus::Active);
    }

    // 错误场景测试
    #[tokio::test]
    async fn test_create_terminal_invalid_config() {
        let invalid_config = TerminalConfig::default();
        let result = create_terminal(invalid_config).await;

        assert!(result.is_err());
        assert_error_contains(result, "配置无效");
    }

    // 边界条件测试
    #[tokio::test]
    async fn test_terminal_max_capacity() {
        // 测试最大容量限制
    }
}
```

### 2. 集成测试模式

```rust
// tests/integration/terminal_integration_test.rs
use terminal_lib::*;
use test_utils::*;

#[tokio::test]
async fn test_full_terminal_lifecycle() {
    let test_env = TestEnvironment::new().await;

    // 1. 创建终端
    let terminal_id = test_env.create_terminal().await?;

    // 2. 发送命令
    test_env.send_command(terminal_id, "echo hello").await?;

    // 3. 验证输出
    let output = test_env.read_output(terminal_id).await?;
    assert!(output.contains("hello"));

    // 4. 清理
    test_env.cleanup().await?;
}
```

### 3. 前端测试规范

```typescript
// tests/components/Terminal.test.ts
import { mount } from '@vue/test-utils'
import Terminal from '@/components/Terminal.vue'

describe('Terminal组件', () => {
  it('应该正确渲染', () => {
    const wrapper = mount(Terminal, {
      props: {
        config: {
          shellPath: '/bin/bash',
          workingDirectory: '/home/user',
        },
      },
    })

    expect(wrapper.find('.terminal').exists()).toBe(true)
  })

  it('应该处理用户输入', async () => {
    const wrapper = mount(Terminal)
    const input = wrapper.find('input')

    await input.setValue('echo test')
    await input.trigger('keydown.enter')

    expect(wrapper.emitted('command')).toBeTruthy()
  })
})
```

## 🔍 代码审查指南

### 审查检查点

#### 1. 架构层面

- [ ] 是否遵循现有的模块结构？
- [ ] 是否使用了统一的错误处理？
- [ ] 是否避免了不必要的抽象？
- [ ] 是否保持了API的一致性？

#### 2. 代码质量

- [ ] 是否通过了所有Clippy检查？
- [ ] 是否有充分的测试覆盖？
- [ ] 是否有清晰的文档注释？
- [ ] 是否遵循了命名约定？

#### 3. 性能考虑

- [ ] 是否避免了不必要的内存分配？
- [ ] 是否正确处理了异步操作？
- [ ] 是否考虑了并发安全？
- [ ] 是否有潜在的性能瓶颈？

#### 4. 安全性

- [ ] 是否正确验证了输入参数？
- [ ] 是否避免了潜在的安全漏洞？
- [ ] 是否正确处理了敏感数据？
- [ ] 是否遵循了最小权限原则？

### 常见问题和解决方案

#### 1. 错误处理不一致

```rust
// ❌ 错误做法
fn bad_function() -> Result<String, Box<dyn std::error::Error>> {
    // 使用了不同的错误类型
}

// ❌ 错误做法
fn bad_function2() -> Result<String, CustomError> {
    // 使用了自定义错误类型
}

// ✅ 正确做法
fn good_function() -> AppResult<String> {
    // 统一使用 anyhow::Result
}
```

#### 2. 过度抽象

```rust
// ❌ 过度抽象
trait GenericProcessor<T, U, V> {
    fn process(&self, input: T) -> Result<U, V>;
}

// ✅ 简单直接
fn process_terminal_data(data: &str) -> AppResult<ProcessedData> {
    // 直接的实现
}
```

#### 3. 缺少错误上下文

```rust
// ❌ 缺少上下文
file.read_to_string(&mut content)?;

// ✅ 提供上下文
file.read_to_string(&mut content)
    .context("读取文件失败")?;

// ✅ 提供详细上下文
file.read_to_string(&mut content)
    .with_context(|| format!("读取配置文件失败: {}", file_path.display()))?;
```

---

**记住：保持代码质量和一致性是每个开发者的责任。遵循这些规范将帮助我们构建更好的软件。**
