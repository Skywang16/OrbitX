//! Transform 层：Anthropic 格式到其他 Provider 格式的转换
//!
//! ## 设计原则
//!
//! 1. **单向转换**：只做 Anthropic → Other，不做反向
//! 2. **集中管理**：每个 provider 一个文件，易于维护和测试
//! 3. **无状态**：所有转换函数都是纯函数
//!
//! ## 对应关系
//!
//! | 模块 | 转换目标 | 参考 |
//! |------|---------|------|
//! | `openai` | OpenAI Chat Completions API | Cline: transform/openai-format.ts |
//! | `gemini` | Google Gemini API | Cline: transform/gemini-format.ts |

pub mod openai;
// pub mod gemini;  // TODO: Phase 2

pub use openai::*;
