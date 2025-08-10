# Eko 2.0 概述

![Eko Logo](https://fellou.ai/eko/docs/_astro/eko-colorful.Cf3lDsAa_Zvvc8f.webp)

## 什么是 Eko？

Eko（发音类似"echo"）是一个强大的框架，专为构建生产就绪的代理工作流而设计。它为自动化工作流的规划和执行提供了高效的跨平台解决方案。此外，Eko 提供高度可定制的接口，使开发者能够自由设计工作流，确保满足生产级要求。

## 在 Online-mind2web 基准测试中的 SOTA 表现

![基准测试结果](https://fellou.ai/eko/docs/_astro/Fellouwithekov2_online_mind2web.Bllk5_2O_MQ4HI.webp)

## Eko 2.0 高级架构

![架构图](https://fellou.ai/eko/docs/_astro/architecture-new-placeholder.iZRdPanV_Z39vQo.webp)

## Eko 2.0 vs Eko 1.0

| 功能                        | Eko 2.0            | Eko 1.0  |
| --------------------------- | ------------------ | -------- |
| **速度**                    | **1.2倍快**        | 慢       |
| **多代理**                  | ✅                 | ❌       |
| **监听 DOM 事件和循环任务** | ✅                 | ❌       |
| **MCP / 工具**              | ✅                 | ❌       |
| **A2A**                     | ✅ (即将推出)      | ❌       |
| **动态 LLM 配置**           | ✅                 | ❌       |
| **规划**                    | 流式规划和重新规划 | 简单规划 |
| **ReAct**                   | ✅                 | ✅       |
| **回调**                    | 流式回调和人工回调 | 简单钩子 |
| **回调链**                  | 流式回调和人工回调 | 简单钩子 |
| **Node.js 与 Playwright**   | ✅                 | ✅       |

Eko 2.0 在 Online-Mind2web 基准测试中达到了 80% 的成功率，而 Eko 1.0 为 31%。这种性能改进反映了 Fellou 在新版本中实施的架构增强和优化，使 Eko 2.0 在生产工作流中显著更加可靠。

![Performance Comparison](https://fellou.ai/eko/docs/_astro/Fellouwithekov2_Fellouwithekov1_Browseruse.BcBu90DX_Of62G.webp)

## 框架对比

| 功能                     | Eko                 | Langchain | Browser-use | Dify.ai | Coze |
| ------------------------ | ------------------- | --------- | ----------- | ------- | ---- |
| **支持平台**             | **所有平台**        | 服务器端  | 浏览器      | Web     | Web  |
| **一句话到多步骤工作流** | ✅                  | ❌        | ✅          | ❌      | ❌   |
| **可干预性**             | ✅                  | ✅        | ❌          | ❌      | ❌   |
| **开发效率**             | **高**              | 低        | 中等        | 中等    | 低   |
| **开源**                 | ✅                  | ✅        | ✅          | ✅      | ❌   |
| **访问私有网络资源**     | ✅ **（即将推出）** | ❌        | ❌          | ❌      | ❌   |

## 支持的环境

![支持的环境](https://fellou.ai/eko/docs/_astro/envs.DR-uHsxW_Z1UoIuG.webp)

了解更多：

- [浏览器扩展环境](installation.md#浏览器扩展)
- [Web 环境](installation.md#web环境)
- [Node.js 环境](installation.md#nodejs环境)
- [下一代 AI 浏览器 Fellou 环境](https://fellou.ai/)

## 开始使用

- [快速开始](quickstart.md)
- [安装](installation.md)
- [配置](configuration.md)

## 支持和社区

- [GitHub Issues](https://github.com/FellouAI/eko/issues) 用于错误报告和功能请求
- [文档](https://eko.fellou.ai/docs) 详细指南和 API 参考
