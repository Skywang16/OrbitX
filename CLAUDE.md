# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

OrbitX is a cross-platform terminal application with built-in AI assistant capabilities, built with Vue 3 and Tauri. Currently adapted for macOS only, with Windows/Linux support in development.

## Core Development Commands

### Frontend Development

```bash
# Start frontend dev server
npm run dev

# Build frontend with type checking
npm run build

# Type checking only
vue-tsc --noEmit

# Linting (check and fix)
npm run lint:check
npm run lint

# Formatting (check and fix)
npm run format:check
npm run format
```

### Tauri Development

```bash
# Run Tauri in development mode (start this after npm run dev)
npm run tauri dev

# Build Tauri application
npm run tauri build

# Build with specific target (macOS universal)
npm run tauri build -- --target universal-apple-darwin
```

### Rust/Backend Development

```bash
# Run all tests
cd src-tauri && cargo test

# Run specific test
cd src-tauri && cargo test test_name

# Check compilation without building
cd src-tauri && cargo check

# Build release version
cd src-tauri && cargo build --release
```

## Architecture Overview

### Frontend Architecture (Vue 3 + TypeScript)

The frontend uses a modular architecture with clear separation of concerns:

- **State Management**: Pinia stores in `src/stores/` manage application state for tabs, terminals, windows, sessions, themes, and tasks
- **Component Structure**: Vue components in `src/components/` with major subsystems:
  - `AIChatSidebar/`: AI assistant interface with message rendering, task management, and tool visualization
  - Terminal components for xterm.js integration
  - Theme and configuration management
- **AI Core System** (`src/eko-core/`): Agent-based AI system with:
  - Task tree planning and execution
  - Tool registry and execution framework
  - Memory management and context handling
  - Event-driven architecture with state management
- **API Layer** (`src/api/`): Modular TypeScript interfaces for Tauri commands
  - Each domain has its own module (terminal, ai, completion, workspace, etc.)
  - Centralized in `src/api/index.ts` for re-exports
  - Direct command invocations without intermediate layers

### Backend Architecture (Rust/Tauri)

The backend follows a Mux-centric architecture for terminal management:

- **Mux Core** (`src-tauri/src/mux/`): Centralized terminal multiplexer managing all terminal sessions
  - Thread-safe session management with RwLock
  - Event-driven notification system
  - High-performance I/O with dedicated thread pool
  - Batch processing optimizations

- **Domain Modules**:
  - `terminal/`: Terminal context and event handling
  - `ai/`: AI service integration with tool adaptors
  - `llm/`: LLM provider management and streaming
  - `completion/`: Smart command completion system
    - `providers/`: Multiple completion sources (history, filesystem, git, npm, system commands)
    - `engine`: Core completion orchestration
    - `scoring`: Unified scoring system with metadata-driven registry
    - `prediction`: Context-aware command prediction with Frecency algorithm
    - `output_analyzer`: Terminal output analysis for intelligent suggestions
  - `storage/`: SQLite-based persistence with repositories pattern
  - `shell/`: Shell integration commands
  - `config/`: Theme and configuration management

- **Command Registration**: All Tauri commands are centrally registered in `src-tauri/src/commands/mod.rs`

### Data Flow

1. **Frontend → Backend**: Vue components invoke Tauri commands through `@tauri-apps/api`
2. **Terminal I/O**: Mux manages PTY sessions, handling input/output through dedicated I/O threads
3. **Events**: Backend emits events via Tauri's event system, frontend subscribes and reacts
4. **AI Processing**: Eko core orchestrates AI tasks with tool execution and state management

## Key Technical Details

### Terminal Management

- Uses `portable-pty` for cross-platform pseudo-terminal support
- xterm.js for frontend terminal rendering with plugins (search, links, ligatures)
- Efficient batch processing for terminal output

### AI Integration

- Multi-provider LLM support (OpenAI, Claude, Gemini, etc.)
- Tree-based task planning with hierarchical execution
- Tool system for extending AI capabilities
- Persistent conversation history in SQLite

### Smart Completion System

- **Providers Architecture**: Modular completion sources (history, filesystem, git, npm, system commands)
- **Unified Scoring**: Metadata-driven command registry with weighted scoring
- **Frecency Algorithm**: Frequency + Recency based ranking for command suggestions
- **Context Detection**: Analyzes terminal output to provide context-aware completions
- **Prediction Engine**: Learns command patterns and suggests next commands based on history

### Performance Optimizations

- Rust async runtime (Tokio) for concurrent operations
- Message batching for terminal output
- LRU caching for file system operations
- Thread-safe shared state with minimal locking contention

## Testing Guidelines

### Frontend Testing

```bash
# Currently no test runner configured
# Type checking serves as primary validation
npm run build
```

### Backend Testing

```bash
cd src-tauri

# Run all tests
cargo test

# Run specific test file
cargo test --test mux_integration_test

# Run with output
cargo test -- --nocapture
```

## Database Schema

The application uses SQLite with migrations in `src-tauri/sql/`. Key tables:

- Conversations and messages for AI chat history
- Tasks with hierarchical structure
- Configuration and theme storage
- Agent execution logs and context snapshots

## Important Conventions

### Frontend Code Standards

1. **No Dynamic Imports**: Never use `await import()` or dynamic imports. All imports must be static at the top of the file.
   ```typescript
   // ✅ Correct
   import { workspaceApi } from '@/api/workspace'

   // ❌ Wrong
   const { workspaceApi } = await import('@/api/workspace')
   ```

2. **No Try-Catch Around API Calls**: API layer already handles errors uniformly. Do not wrap API calls with try-catch.
   ```typescript
   // ✅ Correct
   workspaceApi.maintainWorkspaces()

   // ❌ Wrong
   try {
     await workspaceApi.maintainWorkspaces()
   } catch (error) {
     console.warn('Failed:', error)
   }
   ```

3. **Error Handling**: Use proper error boundaries in Vue components when needed, but trust the API layer's error handling.

### Backend Code Standards

1. **Error Handling**: Always use `Result<T, E>` types, never panic in production code.
2. **Logging**: Use structured logging with `tracing` crate (debug, info, warn, error levels).
3. **State Management**: Shared state must use `Arc<RwLock<T>>` or `Arc<Mutex<T>>` with clear ownership.
4. **Events**: Use Tauri's event system for frontend-backend communication, avoid polling.
5. **File Paths**: All file paths in Tauri commands must be absolute paths, never relative.

## Development Workflow

1. Start frontend dev server: `npm run dev`
2. In another terminal: `npm run tauri dev`
3. Make changes - both frontend and Rust will hot-reload
4. Before committing:
   - Run `npm run lint:check` and `npm run format:check`
   - Run `npm run build` to verify TypeScript compilation
   - Run `cd src-tauri && cargo test` for backend tests
