# ck - Semantic Grep by Embedding

**ck (seek)** finds code by meaning, not just keywords. It's a drop-in replacement for `grep` that understands what you're looking for — search for "error handling" and find try/catch blocks, error returns, and exception handling code even when those exact words aren't present.

## Quick Start

```bash
# Install from crates.io
cargo install ck-search

# Or build from source
git clone https://github.com/BeaconBay/ck
cd ck
cargo build --release
```

```bash
# Index your project for semantic search (one-time setup)
ck --index src/

# Search by meaning - automatically updates index for changed files
ck --sem "error handling" src/
ck --sem "authentication logic" src/
ck --sem "database connection pooling" src/

# Traditional grep-compatible search still works
ck -n "TODO" *.rs
ck -R "TODO|FIXME" .

# Combine both: semantic relevance + keyword filtering
ck --hybrid "connection timeout" src/
```

## Why ck?

**For Developers:** Stop hunting through thousands of regex false positives. Find the code you actually need by describing what it does.

**For AI Agents:** Get structured, semantic search results in JSONL format. Stream-friendly, error-resilient output perfect for LLM workflows, code analysis, documentation generation, and automated refactoring.

## Core Features

### 🔍 **Semantic Search**

Find code by concept, not keywords. Searches understand synonyms, related terms, and conceptual similarity.

```bash
# These find related code even without exact keywords:
ck --sem "retry logic"           # finds backoff, circuit breakers
ck --sem "user authentication"   # finds login, auth, credentials
ck --sem "data validation"       # finds sanitization, type checking

# Get complete functions/classes containing matches (NEW!)
ck --sem --full-section "error handling"  # returns entire functions
ck --full-section "async def" src/        # works with regex too
```

### ⚡ **Drop-in grep Compatibility**

All your muscle memory works. Same flags, same behavior, same output format.

```bash
ck -i "warning" *.log              # Case-insensitive
ck -n -A 3 -B 1 "error" src/       # Line numbers + context
ck --no-filename "TODO" src/        # Suppress filenames (grep -h equivalent)
ck -l "error" src/                  # List files with matches only (NEW!)
ck -L "TODO" src/                   # List files without matches (NEW!)
ck -R --exclude "*.test.js" "bug"  # Recursive with exclusions
ck "pattern" file1.txt file2.txt   # Multiple files
```

### 🎯 **Hybrid Search**

Combine keyword precision with semantic understanding using Reciprocal Rank Fusion.

```bash
ck --hybrid "async timeout" src/    # Best of both worlds
ck --hybrid --scores "cache" src/   # Show relevance scores with color highlighting
ck --hybrid --threshold 0.02 query  # Filter by minimum relevance
ck -l --hybrid "database" src/      # List files using hybrid search
```

### 🤖 **Agent-Friendly Output**

Perfect structured output for LLMs, scripts, and automation. JSONL format provides superior parsing reliability for AI agents.

```bash
# JSONL format - one JSON object per line (recommended for agents)
ck --jsonl --sem "error handling" src/
ck --jsonl --no-snippet "function" .        # Metadata only
ck --jsonl --topk 5 --threshold 0.7 "auth"  # High-confidence results

# Traditional JSON (single array)
ck --json --sem "error handling" src/ | jq '.file'
ck --json --topk 5 "TODO" . | jq -r '.preview'
ck --json --full-section --sem "database" . | jq -r '.preview'  # Complete functions
```

**Why JSONL for AI agents?**

- ✅ **Streaming friendly**: Process results as they arrive, no waiting for complete response
- ✅ **Memory efficient**: Parse one result at a time, not entire array into memory
- ✅ **Error resilient**: One malformed line doesn't break entire response
- ✅ **Composable**: Works perfectly with Unix pipes and stream processing
- ✅ **Standard format**: Used by OpenAI API, Anthropic API, and modern ML pipelines

**JSONL Output Format:**

```json
{"path":"./src/auth.rs","span":{"byte_start":1203,"byte_end":1456,"line_start":42,"line_end":58},"language":"rust","snippet":"fn authenticate(user: User) -> Result<Token> { ... }","score":0.89}
{"path":"./src/error.rs","span":{"byte_start":234,"byte_end":678,"line_start":15,"line_end":25},"language":"rust","snippet":"impl Error for AuthError { ... }","score":0.76}
```

**Agent Processing Example:**

```python
# Stream-process JSONL results (memory efficient)
import json, subprocess

proc = subprocess.Popen(['ck', '--jsonl', '--sem', 'error handling', 'src/'],
                       stdout=subprocess.PIPE, text=True)

for line in proc.stdout:
    result = json.loads(line)
    if result['score'] > 0.8:  # High-confidence matches only
        print(f"📍 {result['path']}:{result['span']['line_start']}")
        print(f"🔍 {result['snippet'][:100]}...")
```

### 📁 **Smart File Filtering**

Automatically excludes cache directories, build artifacts, and respects `.gitignore` files.

```bash
# Respects .gitignore by default (NEW!)
ck "pattern" .                           # Follows .gitignore rules
ck --no-ignore "pattern" .               # Search all files including ignored ones

# These are also excluded by default:
# .git, node_modules, target/, __pycache__

# Override defaults:
ck --no-default-excludes "pattern" .     # Search everything
ck --exclude "dist" --exclude "logs" .   # Add custom exclusions

# Works with indexing too (NEW in v0.3.6!):
ck --index --exclude "node_modules" .    # Exclude from index
ck --index --exclude "*.test.js" .       # Support glob patterns
```

## How It Works

### 1. **Index Once, Search Many**

```bash
# Create semantic index (one-time setup)
ck --index /path/to/project

# Now search instantly by meaning
ck --sem "database queries" .
ck --sem "error handling" .
ck --sem "authentication" .
```

### 2. **Embedding Model Selection**

Choose the right model for your needs when creating the index:

```bash
# Default: BGE-Small (fast, precise chunking)
ck --index .

# Enhanced: Nomic V1.5 (8K context, optimal for large functions)
ck --index --model nomic-v1.5 .

# Code-specialized: Jina Code (optimized for programming languages)
ck --index --model jina-code .
```

**Model Comparison:**

- **`bge-small`** (default): 400-token chunks, fast indexing, good for most code
- **`nomic-v1.5`**: 1024-token chunks with 8K model capacity, better for large functions and classes
- **`jina-code`**: 1024-token chunks with 8K model capacity, specialized for code understanding

**New in v0.4.5:** Token-aware chunking uses actual model tokenizers for precise sizing, with model-specific chunk configurations balancing precision vs context.

**Note:** Model choice is set during indexing. Existing indexes will automatically use their original model.

### 3. **Three Search Modes**

- **`--regex`** (default): Classic grep behavior, no indexing required
- **`--sem`**: Pure semantic search using embeddings (requires index)
- **`--hybrid`**: Combines regex + semantic with intelligent ranking

### 4. **Relevance Scoring**

```bash
ck --sem --scores "machine learning" docs/
# [0.847] ./ai_guide.txt: Machine learning introduction...
# [0.732] ./statistics.txt: Statistical learning methods...
# [0.681] ./algorithms.txt: Classification algorithms...
```

## Advanced Usage

### Search Specific Files

```bash
# Glob patterns work
ck --sem "authentication" *.py *.js *.rs

# Multiple files
ck --sem "error handling" src/auth.rs src/db.rs

# Quoted patterns prevent shell expansion
ck --sem "auth" "src/**/*.ts"
```

### Threshold Filtering

```bash
# Only high-confidence semantic matches
ck --sem --threshold 0.7 "query"

# Low-confidence hybrid matches (good for exploration)
ck --hybrid --threshold 0.01 "concept"

# Get complete code sections instead of snippets (NEW!)
ck --sem --full-section "database queries"
ck --full-section "class.*Error" src/     # Complete classes
```

### Top-K Results

```bash
# Limit results for focused analysis
ck --sem --topk 5 "authentication patterns"

# Great for AI agent consumption
ck --json --topk 10 "error handling" | process_results.py
```

### Directory Management

```bash
# Check index status
ck --status .

# Clean up and rebuild
ck --clean .
ck --index .

# Add single file to index
ck --add new_file.rs
```

### File Inspection (New in v0.4.5)

Analyze how files will be chunked for embedding with the enhanced `--inspect` command:

```bash
# Inspect file chunking and token usage
ck --inspect src/main.rs
# Output: File info, chunk count, token statistics, and chunk details

# Example output:
# File: src/main.rs (49.6 KB, 1378 lines, 12083 tokens)
# Language: rust
#
# Chunks: 17 (tokens: min=4, max=3942, avg=644)
#    1. mod: 4 tokens | L9-9 | mod progress;
#    2. func: 1185 tokens | L88-294 | struct Cli { ... }
#    3. func: 442 tokens | L296-341 | fn expand_glob_patterns(...

# Check different model configurations
ck --inspect --model bge-small src/main.rs      # 400-token chunking
ck --inspect --model nomic-v1.5 src/main.rs    # 1024-token chunking
```

## File Support

| Language   | Indexing | Tree-sitter Parsing | Semantic Chunking                          |
| ---------- | -------- | ------------------- | ------------------------------------------ |
| Python     | ✅       | ✅                  | ✅ Functions, classes                      |
| JavaScript | ✅       | ✅                  | ✅ Functions, classes, methods             |
| TypeScript | ✅       | ✅                  | ✅ Functions, classes, methods             |
| Haskell    | ✅       | ✅                  | ✅ Functions, types, instances             |
| Rust       | ✅       | ✅                  | ✅ Functions, structs, traits              |
| Ruby       | ✅       | ✅                  | ✅ Classes, methods, modules               |
| Go         | ✅       | ✅                  | ✅ Functions, types, methods, variables    |
| C#         | ✅       | ✅                  | ✅ Classes, interfaces, methods, variables |

**Text Formats:** Markdown, JSON, YAML, TOML, XML, HTML, CSS, shell scripts, SQL, log files, config files, and any other text format.

**Smart Binary Detection:** Uses ripgrep-style content analysis (NUL byte detection) instead of extension-based filtering, automatically indexing any text file regardless of extension while correctly excluding binary files.

**Smart Exclusions:** Automatically skips `.git`, `node_modules`, `target/`, `build/`, `dist/`, `__pycache__/`, `.venv`, `venv`, and other common build/cache/virtual environment directories.

## Installation

### From Source

```bash
git clone https://github.com/BeaconBay/ck
cd ck
cargo install --path ck-cli
```

### From crates.io

```bash
# Install latest release from crates.io
cargo install ck-search
```

### Package Managers (Planned)

```bash
# Coming soon:
brew install ck-search
apt install ck-search
```

## Architecture

ck uses a modular Rust workspace:

- **`ck-cli`** - Command-line interface and argument parsing
- **`ck-core`** - Shared types, configuration, and utilities
- **`ck-search`** - Search engine implementations (regex, BM25, semantic)
- **`ck-index`** - File indexing, hashing, and sidecar management
- **`ck-embed`** - Text embedding providers (FastEmbed, API backends)
- **`ck-ann`** - Approximate nearest neighbor search indices
- **`ck-chunk`** - Text segmentation and language-aware parsing
- **`ck-models`** - Model registry and configuration management

### Index Storage

Indexes are stored in `.ck/` directories alongside your code:

```
project/
├── src/
├── docs/
└── .ck/           # Semantic index (can be safely deleted)
    ├── embeddings.json
    ├── ann_index.bin
    └── tantivy_index/
```

The `.ck/` directory is a cache — safe to delete and rebuild anytime.

## Examples

### Finding Code Patterns

```bash
# Find authentication/authorization code
ck --sem "user permissions" src/
ck --sem "access control" src/
ck --sem "login validation" src/

# Find error handling strategies
ck --sem "exception handling" src/
ck --sem "error recovery" src/
ck --sem "fallback mechanisms" src/

# Find performance-related code
ck --sem "caching strategies" src/
ck --sem "database optimization" src/
ck --sem "memory management" src/
```

### Integration Examples

```bash
# Git hooks
git diff --name-only | xargs ck --sem "TODO"

# CI/CD pipeline
ck --json --sem "security vulnerability" . | security_scanner.py

# Code review prep
ck --hybrid --scores "performance" src/ > review_notes.txt

# Documentation generation
ck --json --sem "public API" src/ | generate_docs.py
```

### Team Workflows

```bash
# Find related test files
ck --sem "unit tests for authentication" tests/
ck -l --sem "test" tests/           # List test files by semantic content

# Identify refactoring candidates
ck --sem "duplicate logic" src/
ck --sem "code complexity" src/
ck -L "test" src/                   # Find source files without tests

# Security audit
ck --hybrid "password|credential|secret" src/
ck --sem "input validation" src/
ck -l --hybrid --scores "security" src/  # Files with security-related code
```

## Configuration

### Default Exclusions

```bash
# View current exclusion patterns
ck --help | grep -A 20 exclude

# These directories are excluded by default:
# .git, .svn, .hg                    # Version control
# node_modules, target, build        # Build artifacts
# .cache, __pycache__  # Caches
# .vscode, .idea                     # IDE files
```

### Custom Configuration (Planned)

```toml
# .ck/config.toml
[search]
default_mode = "hybrid"
default_threshold = 0.05

[indexing]
exclude_patterns = ["*.log", "temp/"]
chunk_size = 512
overlap = 64

[models]
embedding_model = "BAAI/bge-small-en-v1.5"
```

## Performance

- **Indexing:** ~1M LOC in under 2 minutes (with smart exclusions and token-aware chunking)
- **Search:** Sub-500ms queries on typical codebases
- **Index size:** ~2x source code size with compression
- **Memory:** Efficient streaming for large repositories with span-based content extraction
- **File filtering:** Automatic exclusion of virtual environments and build artifacts
- **Output:** Clean stdout/stderr separation for reliable piping and scripting
- **Token precision:** HuggingFace tokenizers for exact model-specific token counting (v0.4.5+)

## Testing

Run the comprehensive test suite:

```bash
# Full test suite (40+ tests)
./test_ck.sh

# Quick smoke test (14 core tests)
./test_ck_simple.sh
```

Tests cover grep compatibility, semantic search, index management, file filtering, and more.

## Contributing

ck is actively developed and welcomes contributions:

1. **Issues:** Report bugs, request features
2. **Code:** Submit PRs for bug fixes, new features
3. **Documentation:** Improve examples, guides, tutorials
4. **Testing:** Help test on different codebases and languages

### Development Setup

```bash
git clone https://github.com/your-org/ck
cd ck
cargo build
cargo test
./target/debug/ck --index test_files/
./target/debug/ck --sem "test query" test_files/
```

## Roadmap

### Current (v0.4+)

- ✅ grep-compatible CLI with semantic search and file listing flags (`-l`, `-L`)
- ✅ FastEmbed integration with BGE models and enhanced model selection
- ✅ File exclusion patterns and glob support
- ✅ Threshold filtering and relevance scoring with visual highlighting
- ✅ Tree-sitter parsing and intelligent chunking (Python, TypeScript, JavaScript, Go, Haskell, Rust, Ruby)
- ✅ Complete code section extraction (`--full-section`)
- ✅ Enhanced indexing strategy with v3 semantic search optimization
- ✅ Clean stdout/stderr separation for reliable scripting
- ✅ Incremental index updates with hash-based change detection
- ✅ Token-aware chunking with HuggingFace tokenizers (v0.4.5)
- ✅ Model-specific chunk sizing and FastEmbed capacity utilization (v0.4.5)
- ✅ Enhanced `--inspect` command with token analysis (v0.4.5)
- ✅ Granular indexing progress with file-level and chunk-level progress bars (v0.4.5)

### Next (v0.5+)

- ✅ Published to crates.io (`cargo install ck-search`)
- 🚧 Configuration file support
- 🚧 Package manager distributions (brew, apt)

## FAQ

**Q: How is this different from grep/ripgrep/silver-searcher?**  
A: ck includes all the features of traditional search tools, but adds semantic understanding. Search for "error handling" and find relevant code even when those exact words aren't used.

**Q: Does it work offline?**  
A: Yes, completely offline. The embedding model runs locally with no network calls.

**Q: How big are the indexes?**  
A: Typically 1-3x the size of your source code, depending on content. The `.ck/` directory can be safely deleted to reclaim space.

**Q: Is it fast enough for large codebases?**  
A: Yes. Indexing is a one-time cost, and searches are sub-second even on large projects. Regex searches require no indexing and are as fast as grep.

**Q: Can I use it in scripts/automation?**  
A: Absolutely. The `--json` flag provides structured output perfect for automated processing. Use `--full-section` to get complete functions for AI analysis.

**Q: What about privacy/security?**  
A: Everything runs locally. No code or queries are sent to external services. The embedding model is downloaded once and cached locally.

**Q: Where are the embedding models cached?**  
A: The embedding models (ONNX format) are downloaded and cached in platform-specific directories:

- Linux/macOS: `~/.cache/ck/models/` (or `$XDG_CACHE_HOME/ck/models/` if set)
- Windows: `%LOCALAPPDATA%\ck\cache\models\`
- Fallback: `.ck_models/models/` in the current directory (only if no home directory is found)

The models are downloaded automatically on first use and reused for subsequent runs.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Credits

Built with:

- [Rust](https://rust-lang.org) - Systems programming language
- [FastEmbed](https://github.com/Anush008/fastembed-rs) - Fast text embeddings
- [Tantivy](https://github.com/quickwit-oss/tantivy) - Full-text search engine
- [clap](https://github.com/clap-rs/clap) - Command line argument parsing

Inspired by the need for better code search tools in the age of AI-assisted development.

---

**Start finding code by what it does, not what it says.**

```bash
cargo build --release
./target/release/ck --index .
./target/release/ck --sem "the code you're looking for"
```
