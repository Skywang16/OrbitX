# OrbitX

[中文](./README_zh.md) | English

**The open source AI coding agent with a beautiful desktop interface.**

![CI](https://img.shields.io/github/actions/workflow/status/Skywang16/OrbitX/ci.yml?branch=main&label=CI)
[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Release](https://img.shields.io/github/v/release/Skywang16/OrbitX)](https://github.com/Skywang16/OrbitX/releases)

> Platform Support: Currently macOS only (Windows/Linux support coming soon)

## Features

- **AI Coding Agent** - Powerful AI assistant for code generation, refactoring, and analysis
- **Multi-provider Support** - Works with OpenAI, Claude, Gemini, and OpenAI-compatible APIs
- **Built-in Terminal** - Full-featured terminal with xterm.js (search, links, auto-fit)
- **Agent Modes** - Switch between different agent modes for various tasks
- **Smart Completion** - Command completion, file path completion, Git/NPM integration
- **Customizable Themes** - Multiple built-in themes with light/dark mode support
- **Native Performance** - Built with Tauri for small size and low resource usage

## Subagents

OrbitX uses subagents to handle complex tasks:

- **General** - General-purpose subagent for complex searches and multi-step tasks
- **Plan** - Read-only subagent for code analysis and planning
- **Explore** - Fast subagent for exploring codebases and gathering information

## Installation

### Download Desktop App

Download directly from the [releases page](https://github.com/Skywang16/OrbitX/releases).

| Platform              | Download                      |
| --------------------- | ----------------------------- |
| macOS (Apple Silicon) | `OrbitX_x.x.x_aarch64.dmg` |
| macOS (Intel)         | `OrbitX_x.x.x_x64.dmg`     |
| Windows               | Coming soon                   |
| Linux                 | Coming soon                   |

### Build from Source

```bash
git clone https://github.com/Skywang16/OrbitX.git
cd OrbitX
npm install
npm run tauri build
```

## Development

```bash
# Start frontend dev server
npm run dev

# In another terminal, start Tauri in dev mode
npm run tauri dev
```

## Tech Stack

- **Frontend**: Vue 3 + TypeScript + Vite
- **Desktop Framework**: Tauri 2
- **Terminal**: xterm.js
- **State Management**: Pinia
- **Backend**: Rust

## Configuration

- Themes: `config/themes/*.json`
- Workspace settings: `.orbitx/settings.json`

## FAQ

### How is this different from other AI coding tools?

- **100% open source** - Full transparency and community-driven development
- **Provider agnostic** - Not locked to any single AI provider
- **Native desktop app** - Fast, lightweight, and works offline (with local models)
- **Beautiful UI** - Modern interface designed for developers

## Contributing

If you're interested in contributing, please read our [contributing docs](./CONTRIBUTING.md) before submitting a pull request.

## Acknowledgments

- [Tauri](https://tauri.app/)
- [Vue.js](https://vuejs.org/)
- [xterm.js](https://xtermjs.org/)

## License

This project is licensed under GPL-3.0-or-later. See the `LICENSE` file for details.

---

**Contact**: For issues and suggestions, please create an [Issue](https://github.com/Skywang16/OrbitX/issues).

⭐ If this project helps you, please give it a star!
