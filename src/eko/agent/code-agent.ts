/**
 * 代码专用Agent
 * 为代码开发提供专门的AI代理功能
 */

import { Agent } from '@eko-ai/eko'
import type { CodeAgentConfig } from '../types'
import { getToolsForMode } from '../tools'

/**
 * 代码Agent类
 * 继承自Eko的Agent基类，专门为代码开发优化
 */
export class CodeAgent extends Agent {
  private config: CodeAgentConfig
  private baseDescription: string

  // 静态实例引用，允许工具访问当前活跃的Agent
  private static currentInstance: CodeAgent | null = null

  constructor(config: Partial<CodeAgentConfig> = {}) {
    // 默认配置
    const defaultConfig: CodeAgentConfig = {
      name: ' OrbitCode',
      description: `你是  OrbitCode，OrbitX 中的专业代码开发AI助手。你是一个强大的代码智能体，专注于高质量的软件开发。

# 身份与角色
你是  OrbitCode，一个专业的代码开发AI助手，具备以下特征：
- 专注于代码开发、分析、重构和优化
- 深度理解软件工程最佳实践和设计模式
- 能够进行复杂的代码推理和架构设计
- 始终以代码质量和可维护性为优先考虑

# 工作模式
## chat 模式（只读）
- 仅使用只读工具：文件读取、代码分析、语法检查
- 禁止任何写入、修改或创建操作
- 可以提供代码建议和分析报告
- 如需执行修改，提示用户切换到 agent 模式

## agent 模式（全权限）
- 可使用全部工具：代码编写、文件修改、重构、测试
- 在执行重要操作前进行影响分析
- 遵循渐进式修改原则，避免大规模破坏性变更
- 每次修改后验证代码完整性

# 核心能力矩阵

## 代码生成与编写
- 支持多语言代码生成：JavaScript/TypeScript, Python, Java, Go, Rust, C++等
- 遵循语言特定的最佳实践和惯用法
- 生成高质量、可读性强的代码
- 自动添加适当的注释和文档

## 代码分析与理解
- 深度分析代码结构和依赖关系
- 识别代码异味和潜在问题
- 理解业务逻辑和设计意图
- 提供架构级别的洞察

## 重构与优化
- 安全的代码重构，保持功能不变
- 性能优化和内存管理改进
- 代码结构优化和模块化改进
- 遵循SOLID原则和设计模式

## 错误诊断与修复
- 快速定位和分析错误根因
- 提供多种修复方案
- 预防性错误检测
- 代码健壮性改进

# 技术专长领域

## 前端开发
- React/Vue/Angular生态系统
- TypeScript/JavaScript高级特性
- 现代CSS/Sass/Less
- 前端工程化和构建工具

## 后端开发
- RESTful API和GraphQL设计
- 数据库设计和ORM
- 微服务架构
- 性能优化和缓存策略

## 全栈开发
- 前后端集成
- 状态管理
- 实时通信
- 部署和DevOps

# 工作原则

## 代码质量标准
1. **可读性优先**：代码应该像文档一样清晰
2. **可维护性**：易于修改和扩展
3. **性能考虑**：在不牺牲可读性的前提下优化性能
4. **安全意识**：始终考虑安全最佳实践

## 开发流程
1. **理解需求**：深入理解用户意图和业务需求
2. **分析现状**：评估现有代码结构和约束
3. **设计方案**：提出清晰的实现方案
4. **渐进实施**：分步骤实现，确保每步都可验证
5. **验证测试**：确保修改不破坏现有功能

## 沟通风格
- 直接、专业、技术导向
- 提供具体的代码示例
- 解释技术决策的原因
- 主动识别潜在风险和替代方案

# 安全与约束
- 在执行破坏性操作前必须警告用户
- 保护重要配置文件和数据
- 遵循最小权限原则
- 智能识别危险操作模式
`,
      defaultWorkingDirectory: undefined,
      safeMode: true,
      supportedLanguages: [
        'javascript',
        'typescript',
        'python',
        'java',
        'go',
        'rust',
        'cpp',
        'c',
        'html',
        'css',
        'scss',
        'sass',
        'vue',
        'react',
        'angular',
        'svelte',
        'php',
        'ruby',
        'swift',
        'kotlin',
        'dart',
        'shell',
        'sql',
        'json',
        'yaml',
        'xml',
      ],
      codeStyle: {
        indentSize: 2,
        indentType: 'spaces',
        maxLineLength: 100,
        insertFinalNewline: true,
        trimTrailingWhitespace: true,
      },
      enabledFeatures: {
        codeGeneration: true,
        codeAnalysis: true,
        refactoring: true,
        formatting: true,
        linting: true,
        testing: true,
        documentation: true,
      },
    }

    // 合并配置
    const finalConfig = { ...defaultConfig, ...config }

    // 调用父类构造函数
    super({
      name: finalConfig.name,
      description: finalConfig.description,
      tools: getToolsForMode('chat') as any, // 初始化为chat模式的只读工具
      llms: ['default'], // 使用默认模型
    })

    this.config = finalConfig
    this.baseDescription = finalConfig.description

    // 设置为当前活跃实例
    CodeAgent.currentInstance = this
  }

  /**
   * 获取Agent配置
   */
  getConfig(): CodeAgentConfig {
    return { ...this.config }
  }

  /**
   * 切换工作模式并更新工具/提示词
   */
  setMode(mode: 'chat' | 'agent'): void {
    // 更新工具权限
    this.tools = getToolsForMode(mode) as any

    // 根据模式强化描述中的权限提醒
    const modeNotice =
      mode === 'chat'
        ? `\n\n🔐 当前模式：chat（只读）\n- 仅可使用读取类工具（读取文件/代码分析）\n- 禁止写入、修改代码、创建文件\n- 如需编写代码，请用户切换到 agent 模式`
        : `\n\n🛠️ 当前模式：agent（全权限）\n- 可使用全部工具（含代码编写/文件修改）\n- 重要代码修改前需给出影响分析并征得确认`

    this.description = `${this.baseDescription}${modeNotice}`
  }

  /**
   * 更新Agent配置
   */
  updateConfig(updates: Partial<CodeAgentConfig>): void {
    this.config = { ...this.config, ...updates }

    // 更新描述
    if (updates.description) {
      this.description = updates.description
    }
  }

  /**
   * 设置支持的编程语言
   */
  setSupportedLanguages(languages: string[]): void {
    this.config.supportedLanguages = languages
  }

  /**
   * 获取支持的编程语言
   */
  getSupportedLanguages(): string[] {
    return [...this.config.supportedLanguages]
  }

  /**
   * 设置代码风格配置
   */
  setCodeStyle(style: Partial<CodeAgentConfig['codeStyle']>): void {
    this.config.codeStyle = { ...this.config.codeStyle, ...style }
  }

  /**
   * 获取代码风格配置
   */
  getCodeStyle(): CodeAgentConfig['codeStyle'] {
    return { ...this.config.codeStyle }
  }

  /**
   * 启用/禁用特定功能
   */
  setFeatureEnabled(feature: keyof CodeAgentConfig['enabledFeatures'], enabled: boolean): void {
    this.config.enabledFeatures[feature] = enabled
  }

  /**
   * 检查功能是否启用
   */
  isFeatureEnabled(feature: keyof CodeAgentConfig['enabledFeatures']): boolean {
    return this.config.enabledFeatures[feature]
  }

  /**
   * 获取Agent状态信息
   */
  getStatus(): {
    name: string
    description: string
    toolsCount: number
    safeMode: boolean
    defaultWorkingDirectory?: string
    supportedLanguagesCount: number
    enabledFeaturesCount: number
    codeStyle: CodeAgentConfig['codeStyle']
  } {
    const enabledFeaturesCount = Object.values(this.config.enabledFeatures).filter(Boolean).length

    return {
      name: this.name,
      description: this.description,
      toolsCount: this.tools.length,
      safeMode: this.config.safeMode || false,
      defaultWorkingDirectory: this.config.defaultWorkingDirectory,
      supportedLanguagesCount: this.config.supportedLanguages.length,
      enabledFeaturesCount,
      codeStyle: this.getCodeStyle(),
    }
  }

  /**
   * 获取当前活跃的Agent实例（供工具使用）
   */
  static getCurrentInstance(): CodeAgent | null {
    return CodeAgent.currentInstance
  }
}

/**
 * 创建代码Agent实例
 */
export const createCodeAgent = (config?: Partial<CodeAgentConfig>): CodeAgent => {
  return new CodeAgent(config)
}
