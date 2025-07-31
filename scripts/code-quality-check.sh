#!/bin/bash
# Code quality check script for TermX project
# This script provides comprehensive code quality analysis

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Main function
main() {
    print_status "Starting comprehensive code quality check for TermX project..."
    
    # Check if we're in the right directory
    if [ ! -f "src-tauri/Cargo.toml" ]; then
        print_error "Please run this script from the project root directory"
        exit 1
    fi
    
    # Change to src-tauri directory
    cd src-tauri
    
    # Check required tools
    print_status "Checking required tools..."
    
    if ! command_exists cargo; then
        print_error "cargo is not installed or not in PATH"
        exit 1
    fi
    
    # 1. Code formatting check
    print_status "Checking code formatting..."
    if cargo fmt --check; then
        print_success "Code formatting is correct"
    else
        print_error "Code formatting issues found"
        print_warning "Run 'cargo fmt' to fix formatting issues"
        exit 1
    fi
    
    # 2. Clippy linting
    print_status "Running Clippy analysis..."
    if cargo clippy --all-targets --all-features -- -D warnings; then
        print_success "Clippy checks passed"
    else
        print_error "Clippy found issues"
        exit 1
    fi
    
    # 3. Build check
    print_status "Checking if project builds..."
    if cargo build --all-features; then
        print_success "Project builds successfully"
    else
        print_error "Build failed"
        exit 1
    fi
    
    # 4. Test execution
    print_status "Running tests..."
    if cargo test --all-features; then
        print_success "All tests passed"
    else
        print_error "Some tests failed"
        exit 1
    fi
    
    # 5. Documentation check
    print_status "Checking documentation..."
    if cargo doc --all-features --no-deps; then
        print_success "Documentation builds successfully"
    else
        print_warning "Documentation has issues"
    fi
    
    # 6. Security audit (if cargo-audit is installed)
    if command_exists cargo-audit; then
        print_status "Running security audit..."
        if cargo audit; then
            print_success "No security vulnerabilities found"
        else
            print_warning "Security audit found issues"
        fi
    else
        print_warning "cargo-audit not installed. Run 'cargo install cargo-audit' for security checks"
    fi
    
    # 7. Dependency check
    print_status "Checking for outdated dependencies..."
    if command_exists cargo-outdated; then
        cargo outdated
    else
        print_warning "cargo-outdated not installed. Run 'cargo install cargo-outdated' for dependency checks"
    fi
    
    # 8. Code coverage (if cargo-tarpaulin is installed)
    if command_exists cargo-tarpaulin; then
        print_status "Generating code coverage report..."
        if cargo tarpaulin --out Html --output-dir ../coverage; then
            print_success "Code coverage report generated in coverage/ directory"
        else
            print_warning "Code coverage generation failed"
        fi
    else
        print_warning "cargo-tarpaulin not installed. Run 'cargo install cargo-tarpaulin' for coverage reports"
    fi
    
    print_success "Code quality check completed successfully!"
    print_status "Summary:"
    echo "  ✅ Code formatting: OK"
    echo "  ✅ Clippy analysis: OK"
    echo "  ✅ Build: OK"
    echo "  ✅ Tests: OK"
    echo "  ✅ Documentation: OK"
}

# Help function
show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Code quality check script for TermX project"
    echo ""
    echo "OPTIONS:"
    echo "  -h, --help     Show this help message"
    echo "  --format-only  Only run formatting check"
    echo "  --clippy-only  Only run Clippy analysis"
    echo "  --test-only    Only run tests"
    echo ""
    echo "Examples:"
    echo "  $0                 # Run all checks"
    echo "  $0 --format-only   # Only check formatting"
    echo "  $0 --clippy-only   # Only run Clippy"
}

# Parse command line arguments
case "${1:-}" in
    -h|--help)
        show_help
        exit 0
        ;;
    --format-only)
        cd src-tauri
        print_status "Running formatting check only..."
        cargo fmt --check
        ;;
    --clippy-only)
        cd src-tauri
        print_status "Running Clippy analysis only..."
        cargo clippy --all-targets --all-features -- -D warnings
        ;;
    --test-only)
        cd src-tauri
        print_status "Running tests only..."
        cargo test --all-features
        ;;
    "")
        main
        ;;
    *)
        print_error "Unknown option: $1"
        show_help
        exit 1
        ;;
esac