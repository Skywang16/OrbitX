# Scripts Directory

This directory contains utility scripts for the OrbitX project.

## Available Scripts

### code-quality-check.sh

A comprehensive code quality check script that runs various checks on the Rust backend code.

**Usage:**

```bash
# Run all checks
./scripts/code-quality-check.sh

# Run specific checks
./scripts/code-quality-check.sh --format-only
./scripts/code-quality-check.sh --clippy-only
./scripts/code-quality-check.sh --test-only

# Show help
./scripts/code-quality-check.sh --help
```

**Features:**

- Code formatting check with `cargo fmt`
- Clippy linting analysis
- Build verification
- Test execution
- Documentation generation check
- Security audit (if `cargo-audit` is installed)
- Dependency outdated check (if `cargo-outdated` is installed)
- Code coverage report (if `cargo-tarpaulin` is installed)

### generate-color-scale.js

Generates color scales for the frontend theme system.

## Git Hooks

### Pre-commit Hook

The project includes a pre-commit hook (`.git/hooks/pre-commit`) that automatically runs:

- Code formatting check
- Clippy analysis
- Test execution

This ensures code quality before commits are made.

## Configuration Files

### .rustfmt.toml

Located in `src-tauri/.rustfmt.toml`, this file configures Rust code formatting standards:

- 100 character line width
- 4 spaces for indentation
- Unix newline style
- Import reordering enabled
- Various formatting preferences

### .clippy.toml

Located in `src-tauri/.clippy.toml`, this file configures Clippy linting rules:

- Cognitive complexity threshold: 30
- Maximum function arguments: 8
- Type complexity threshold: 250
- Various other code quality thresholds

## Installation of Optional Tools

For enhanced functionality, install these optional tools:

```bash
# Security audit
cargo install cargo-audit

# Dependency checking
cargo install cargo-outdated

# Code coverage
cargo install cargo-tarpaulin
```

## Integration with Development Workflow

1. **Before committing**: The pre-commit hook automatically runs quality checks
2. **During development**: Use `./scripts/code-quality-check.sh` for comprehensive analysis
3. **CI/CD**: These scripts can be integrated into continuous integration pipelines
4. **Code reviews**: Use the quality check script to ensure consistent code standards
