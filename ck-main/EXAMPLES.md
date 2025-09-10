# ck (seek) - Usage Examples

This guide walks through practical examples of ck's capabilities, from basic grep replacement to advanced semantic search.

## Quick Setup

First, create a test project to explore ck's features:

```bash
mkdir ck-demo && cd ck-demo

# Create sample files
cat > auth.rs << EOF
use std::collections::HashMap;

pub struct AuthService {
    users: HashMap<String, User>,
}

impl AuthService {
    pub fn authenticate(&self, username: &str, password: &str) -> Result<Token, AuthError> {
        match self.users.get(username) {
            Some(user) if user.verify_password(password) => {
                Ok(Token::new(user.id))
            }
            _ => Err(AuthError::InvalidCredentials)
        }
    }

    pub fn handle_login_error(&self, error: AuthError) {
        log::error!("Authentication failed: {:?}", error);
    }
}
EOF

cat > database.py << EOF
import sqlite3
from typing import Optional

class DatabaseConnection:
    def __init__(self, db_path: str):
        self.connection = sqlite3.connect(db_path)
        
    def authenticate_user(self, username: str, password_hash: str) -> Optional[dict]:
        cursor = self.connection.cursor()
        try:
            cursor.execute(
                "SELECT id, username FROM users WHERE username = ? AND password_hash = ?",
                (username, password_hash)
            )
            return cursor.fetchone()
        except Exception as e:
            print(f"Database error: {e}")
            return None
            
    def handle_connection_error(self, error):
        log.error(f"Database connection failed: {error}")
EOF

cat > server.js << EOF
const express = require('express');
const bcrypt = require('bcrypt');

class AuthController {
    async login(req, res) {
        try {
            const { username, password } = req.body;
            const user = await this.findUser(username);
            
            if (!user || !bcrypt.compare(password, user.password)) {
                return res.status(401).json({ error: 'Authentication failed' });
            }
            
            const token = this.generateToken(user);
            res.json({ token });
        } catch (error) {
            this.handleError(error, res);
        }
    }
    
    handleError(error, res) {
        console.error('Login error:', error);
        res.status(500).json({ error: 'Internal server error' });
    }
}
EOF

cat > README.md << EOF
# Authentication Service

A multi-language authentication system with:

- Rust backend service
- Python database layer  
- JavaScript API endpoints

## Error Handling

All components implement proper error handling for authentication failures.
EOF
```

## 1. Basic Grep-Style Search (No Index Required)

ck works as a drop-in grep replacement:

```bash
# Find all mentions of "error"
ck "error" .

# Case-insensitive search for TODO items
ck -i "todo" .

# Show line numbers
ck -n "authentication" .

# Match whole words only
ck -w "error" .

# Fixed string search (no regex)
ck -F "AuthError::InvalidCredentials" .

# Show context around matches
ck -C 2 "authenticate" .
```

## 2. Create Search Index

Before using semantic or lexical search, create an index:

```bash
# Index current directory
ck index .

# Check what was indexed
ck status .

# Detailed index statistics
ck status . --verbose
```

## 3. Semantic Search - Find Similar Code

Semantic search understands meaning and finds conceptually related code:

```bash
# Find all authentication-related code
ck --sem "user authentication" .

# Find error handling patterns
ck --sem "error handling" .

# Find database operations
ck --sem "database connection" .

# Find login functionality
ck --sem "user login" .

# Limit to top 5 most relevant results
ck --sem "authentication" . --topk 5

# Filter by similarity score threshold (0.0-1.0)
ck --sem "authentication" . --threshold 0.7

# Show similarity scores in output
ck --sem "error handling" . --scores

# Combine threshold and score display
ck --sem "database" . --threshold 0.6 --scores

# Hybrid search with RRF threshold and scores
ck --hybrid "authentication" . --threshold 0.025 --scores
```

### What Makes This Powerful?

- Searching "error handling" finds `handle_login_error`, `handle_connection_error`, and `handleError`
- Searching "authentication" finds `authenticate`, `authenticate_user`, and `login` functions
- It understands that login, authentication, and user verification are related concepts

## 4. Lexical Search - BM25 Full-Text Ranking

Lexical search uses BM25 ranking for better phrase matching than regex:

```bash
# Full-text search with relevance ranking
ck --lex "authentication failed" .

# Better for multi-word phrases
ck --lex "database connection error" .

# Find documentation
ck --lex "error handling components" .
```

## 5. Hybrid Search - Best of Both Worlds

Combines regex pattern matching with semantic understanding:

```bash
# Find functions with "auth" that are semantically related to authentication
ck --hybrid "auth" .

# Pattern match + semantic relevance
ck --hybrid "error" --topk 10 .

# Filter hybrid results by RRF score threshold  
ck --hybrid "auth" --threshold 0.02 .

# Show scores for hybrid search (RRF scores ~0.01-0.05)
ck --hybrid "error" --scores .
```

## 6. JSON Output for Tools/Scripts

Get structured output for integration with other tools:

```bash
# JSON output with relevance scores
ck --json --sem "authentication" .

# Pipe to jq for processing
ck --json --sem "error" . | jq '.score'

# Get top 3 results as JSON
ck --json --topk 3 --lex "user login" .
```

## 7. Index Management

### Monitor Index Health

```bash
# Check index status
ck status .

# Detailed statistics
ck status . --verbose
```

### Update Index After Changes

```bash
# Edit a file
echo "// Added new auth method" >> auth.rs

# Index automatically updates on search, or force update:
ck index .

# Add single file to index
ck add auth.rs
```

### Clean Up Index

```bash
# Remove a file to create orphaned index entries
rm server.js

# Clean up orphaned entries only
ck clean . --orphans

# Remove entire index (will need to reindex for semantic search)
ck clean .
```

## 8. Advanced Examples

### Find All Error Handling Patterns

```bash
# Semantic search finds diverse error handling approaches
ck --sem "error handling" . --json | jq -r '.preview'
```

This finds:
- Rust: `Err(AuthError::InvalidCredentials)`
- Python: `except Exception as e:`
- JavaScript: `catch (error)`
- All `handle_*_error` functions

### Compare Search Methods

```bash
echo "Searching for 'auth' with different methods:"

echo "=== Regex (exact text matching) ==="
ck "auth" .

echo "=== Lexical (BM25 ranking) ==="  
ck --lex "auth" .

echo "=== Semantic (conceptual similarity) ==="
ck --sem "auth" .

echo "=== Hybrid (regex + semantic) ==="
ck --hybrid "auth" .
```

### Integration with Shell Scripts

```bash
#!/bin/bash
# Find security-related TODOs

echo "Security TODOs found:"
ck --sem "security vulnerability" . --json | \
    jq -r '"\(.file):\(.span.line_start): \(.preview)"'

echo -e "\nAuthentication issues:"
ck --hybrid "auth.*TODO" . --json | \
    jq -r '"\(.file): \(.preview)"'
```

## Performance and Use Cases

### When to Use Each Mode

- **Regex** (`ck "pattern"`): Exact text matching, grep replacement, no index needed
- **Lexical** (`--lex`): Multi-word phrases, document search, ranked results
- **Semantic** (`--sem`): Conceptual similarity, find related functionality, code exploration
- **Hybrid** (`--hybrid`): When you want both exact matches and similar concepts

### Index Management

- Index updates automatically during search (if >1 minute old)
- Use `ck status` to monitor index size and health
- Use `ck clean --orphans` after major file reorganization
- Index persists in `.ck/` directory (add to `.gitignore`)

### File Type Support

ck indexes these file types:
- Code: `.rs`, `.py`, `.js`, `.ts`, `.go`, `.java`, `.c`, `.cpp`, `.rb`, `.php`, `.swift`, `.kt`
- Docs: `.md`, `.txt`, `.rst`  
- Config: `.json`, `.yaml`, `.toml`, `.xml`
- Scripts: `.sh`, `.bash`, `.ps1`, `.sql`

## Tips and Tricks

1. **Start with regex** for exact matching, then explore with semantic search
2. **Use `--topk`** to limit results when exploring large codebases  
3. **Use `--threshold`** to filter low-relevance results (semantic/lexical: 0.6-0.8, hybrid RRF: 0.02-0.05)  
4. **Use `--scores`** to see match quality and fine-tune your threshold
5. **Combine with shell tools**: `ck --json --sem "query" | jq`
6. **Index smaller directories** for faster semantic search
7. **Use `ck status -v`** to monitor index growth over time

## Troubleshooting

```bash
# No results from semantic search?
ck status .                    # Check if index exists
ck index . && ck --sem "query" # Create index first

# Index too large?
ck status . --verbose         # Check size statistics  
ck clean . --orphans          # Remove orphaned files

# Stale results?
ck index .                    # Force reindex
```

Happy seeking! 🔍