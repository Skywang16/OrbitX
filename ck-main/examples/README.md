# ck Examples

This folder demonstrates practical use cases for `ck` (semantic grep) with real code examples.

## Quick Start

1. **Index the examples first** (required for semantic/lexical/hybrid search):

   ```bash
   ck --index examples/
   ```

2. **Try the quick demo**:

   ```bash
   ./examples/quick_demo.sh
   ```

3. **Try the examples below** using the demo files in this folder.

## Demo Files

### Code Examples (`code/`)

- **`full_section_demo.py`** - Python code with classes, functions, error handling
- **`web_server.rs`** - Rust web server with authentication, database connections, error handling
- **`api_client.js`** - JavaScript API client with HTTP requests, retry logic, authentication

### Text Samples (`text_samples/`)

- **`fixtures/`** - Simple demo text files (demo1.txt to demo10.txt)
- **`wiki_articles/`** - Wikipedia articles on various topics (AI, science, technology, etc.)

## Search Mode Examples

### 1. **Regex Search** (Default - no indexing required)

Classic grep-style pattern matching:

```bash
# Basic text search
ck "error" examples/

# Case-insensitive search
ck -i "database" examples/

# Whole word matching
ck -w "class" examples/

# Fixed string (no regex)
ck -F "def __init__" examples/

# With line numbers and context
ck -n -C 2 "error" examples/
```

### 2. **Semantic Search** (Finds conceptually similar code)

Understands meaning, not just text matches:

```bash
# Find error handling patterns (try/catch, exception handling, etc.)
ck --sem "error handling" examples/

# Find database-related code
ck --sem "database connection" text_samples/

# Find authentication logic
ck --sem "user authentication" examples/

# Limit results and show scores
ck --sem --limit 5 --scores "data processing" examples/

# Higher precision filtering
ck --sem --threshold 0.8 "function" examples/
```

### 3. **Lexical Search** (BM25 full-text search with ranking)

Better than regex for phrase matching:

```bash
# Full-text search with relevance ranking
ck --lex "user authentication" examples/

# Multi-word phrases
ck --lex "error handling patterns" examples/
```

### 4. **Hybrid Search** (Best of both worlds)

Combines regex precision with semantic understanding:

```bash
# Most comprehensive search
ck --hybrid "async function" examples/

# Find specific patterns with semantic context
ck --hybrid "database" --limit 10 examples/

# Show relevance scores
ck --hybrid "error" --scores examples/
```

## Advanced Features

### Full Section Mode

Get complete code blocks instead of just matching lines:

```bash
# Return entire functions/classes containing matches
ck --sem --full-section "error handling" examples/

# Works with all search modes
ck --hybrid --full-section "database" examples/
```

### JSON Output

Machine-readable results for scripts and tools:

```bash
# JSON output for integration
ck --json --sem "error handling" examples/

# Pipe to jq for processing
ck --json --sem "database" examples/ | jq '.preview'
```

### Context and Formatting

```bash
# Show surrounding lines
ck -A 3 -B 1 "class" examples/    # 3 after, 1 before
ck -C 2 "def" examples/           # 2 lines of context

# File listing modes
ck -l "error" examples/           # List files with matches
ck -L "nonexistent" examples/     # List files without matches
```

## Practical Use Cases

### 1. **Code Review**

```bash
# Find potential security issues
ck --sem "sql injection" examples/
ck --sem "password storage" examples/

# Find error handling patterns
ck --sem "exception handling" examples/
```

### 2. **Code Exploration**

```bash
# Understand new codebase
ck --sem "main entry point" examples/
ck --sem "configuration setup" examples/

# Find similar implementations
ck --sem "data validation" examples/
```

### 3. **Refactoring**

```bash
# Find deprecated patterns
ck --hybrid "old api" examples/

# Locate specific implementations
ck --sem "async operations" examples/
```

### 4. **Documentation**

```bash
# Find examples of usage
ck --sem "example usage" examples/

# Locate API patterns
ck --sem "rest api" examples/
```

## Performance Tips

- **Index once, search many times**: Use `ck --index` before semantic searches
- **Use appropriate search modes**:
  - Regex for exact patterns
  - Semantic for concept-based searches
  - Hybrid for comprehensive results
- **Adjust thresholds**: Lower values = more results, higher = more precise
- **Use --limit**: Control result count for large codebases

## Try It Yourself

Run these commands to see `ck` in action:

```bash
# Index the examples
ck --index examples/

# Compare different search modes on the same query
echo "=== Regex ==="
ck "error" examples/
echo "=== Semantic ==="
ck --sem "error" examples/
echo "=== Hybrid ==="
ck --hybrid "error" examples/
```

Notice how each mode finds different but relevant results!
