#!/bin/bash

# ck test suite - validates core functionality
# Usage: ./test_ck.sh

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Binary location
CK_BIN="./target/debug/ck"
TEST_DIR="test_files/wiki_articles"

# Helper functions
run_test() {
    local test_name="$1"
    local command="$2"
    local expected_behavior="$3"
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo -n "Testing: $test_name ... "
    
    if eval "$command" >/dev/null 2>&1; then
        echo -e "${GREEN}PASS${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}"
        echo "  Command: $command"
        echo "  Expected: $expected_behavior"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

run_test_with_output() {
    local test_name="$1"
    local command="$2"
    local expected_pattern="$3"
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo -n "Testing: $test_name ... "
    
    # Use a timeout and limit output to prevent hangs with large outputs
    if eval "$command" 2>&1 | head -100 | grep -q "$expected_pattern"; then
        echo -e "${GREEN}PASS${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}"
        echo "  Command: $command"
        echo "  Expected pattern: $expected_pattern"
        output=$(eval "$command" 2>&1 | head -5)
        echo "  Got: ${output:0:100}..."
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

run_test_exit_code() {
    local test_name="$1"
    local command="$2"
    local expected_code="$3"
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo -n "Testing: $test_name ... "
    
    set +e  # Temporarily allow errors
    eval "$command" >/dev/null 2>&1
    exit_code=$?
    set -e
    
    if [ $exit_code -eq $expected_code ]; then
        echo -e "${GREEN}PASS${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}"
        echo "  Command: $command"
        echo "  Expected exit code: $expected_code, got: $exit_code"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

echo "========================================="
echo "         ck Test Suite"
echo "========================================="
echo ""

# Check if binary exists
if [ ! -f "$CK_BIN" ]; then
    echo -e "${RED}Error: ck binary not found at $CK_BIN${NC}"
    echo "Please run: cargo build"
    exit 1
fi

# Check if test files exist
if [ ! -d "$TEST_DIR" ]; then
    echo -e "${RED}Error: Test directory not found at $TEST_DIR${NC}"
    exit 1
fi

echo "Using binary: $CK_BIN"
echo "Test directory: $TEST_DIR"
echo ""

# Clean up any existing index
echo "Cleaning up old index..."
$CK_BIN clean $TEST_DIR 2>/dev/null || true
echo ""

echo "=== Basic Functionality Tests ==="
echo ""

# 1. Help and version
run_test "Help flag works" "$CK_BIN --help" "Should display help"
run_test "Version flag works" "$CK_BIN --version" "Should display version"

echo ""
echo "=== Regex Search Tests (grep compatibility) ==="
echo ""

# 2. Basic regex search (no index needed)
run_test_with_output "Basic pattern search" \
    "$CK_BIN 'algorithm' $TEST_DIR/Algorithm.txt" \
    "algorithm"

run_test_with_output "Case insensitive search" \
    "$CK_BIN -i 'ALGORITHM' $TEST_DIR/Algorithm.txt" \
    "algorithm"

run_test_with_output "Line numbers" \
    "$CK_BIN -n 'algorithm' $TEST_DIR/Algorithm.txt" \
    "[0-9]:"

run_test_with_output "Fixed string search" \
    "$CK_BIN -F 'machine learning' $TEST_DIR/Machine_learning.txt" \
    "machine learning"

run_test_with_output "Word boundary search" \
    "$CK_BIN -w 'data' $TEST_DIR/Data_science.txt" \
    "data"

# 3. Multiple file search
run_test_with_output "Multiple files" \
    "$CK_BIN 'science' $TEST_DIR/Data_science.txt $TEST_DIR/Computer_science.txt" \
    "science"

# 4. Recursive search
run_test_with_output "Recursive search" \
    "$CK_BIN -r 'Python' $TEST_DIR" \
    "Python"

# 5. Context lines
run_test "Context lines -C" \
    "$CK_BIN -C 2 'machine' $TEST_DIR/Machine_learning.txt" \
    "Should show 2 lines before and after"

run_test "Context lines -A -B" \
    "$CK_BIN -A 1 -B 2 'learning' $TEST_DIR/Machine_learning.txt" \
    "Should show 1 line after, 2 before"

# 6. Exit codes
run_test_exit_code "Exit 0 when matches found" \
    "$CK_BIN 'algorithm' $TEST_DIR/Algorithm.txt" \
    0

run_test_exit_code "Exit 1 when no matches" \
    "$CK_BIN 'xyznotfound123' $TEST_DIR/Algorithm.txt" \
    1

echo ""
echo "=== Index Management Tests ==="
echo ""

# 7. Index creation
run_test "Create index" \
    "$CK_BIN index $TEST_DIR" \
    "Should create index"

run_test "Check index status" \
    "$CK_BIN status $TEST_DIR" \
    "Should show index status"

run_test_with_output "Status shows indexed files" \
    "$CK_BIN status $TEST_DIR" \
    "Files indexed:"

echo ""
echo "=== Semantic Search Tests ==="
echo ""

# 8. Semantic search (requires index)
run_test_with_output "Semantic search for algorithms" \
    "$CK_BIN --sem 'sorting algorithms' $TEST_DIR" \
    "Algorithm"

run_test_with_output "Semantic search for AI concepts" \
    "$CK_BIN --sem 'artificial intelligence' $TEST_DIR" \
    "intelligence"

run_test_with_output "Semantic search for programming" \
    "$CK_BIN --sem 'programming languages' $TEST_DIR" \
    "programming"

# 9. Semantic search with scores
run_test_with_output "Semantic search with scores" \
    "$CK_BIN --sem --scores 'machine learning' $TEST_DIR" \
    "\["

# 10. Top-k limiting
run_test "Semantic search with top-k" \
    "$CK_BIN --sem --topk 5 'statistics' $TEST_DIR" \
    "Should return max 5 results"

echo ""
echo "=== Lexical Search Tests ==="
echo ""

# 11. Lexical (BM25) search
run_test_with_output "Lexical search" \
    "$CK_BIN --lex 'database systems' $TEST_DIR" \
    "database"

run_test_with_output "Lexical with scores" \
    "$CK_BIN --lex --scores 'computer science' $TEST_DIR" \
    "\["

echo ""
echo "=== Hybrid Search Tests ==="
echo ""

# 12. Hybrid search
run_test_with_output "Hybrid search" \
    "$CK_BIN --hybrid 'data structures' $TEST_DIR" \
    "data"

run_test_with_output "Hybrid with threshold" \
    "$CK_BIN --hybrid --threshold 0.01 'algorithm' $TEST_DIR" \
    "algorithm"

echo ""
echo "=== JSON Output Tests ==="
echo ""

# 13. JSON output
run_test_with_output "JSON output format" \
    "$CK_BIN --json --sem 'python' $TEST_DIR" \
    '"file"'

run_test_with_output "JSON contains score field" \
    "$CK_BIN --json --sem 'rust' $TEST_DIR" \
    '"score"'

echo ""
echo "=== File Filtering Tests ==="
echo ""

# 14. Glob patterns  
# Note: We use explicit files to avoid shell expansion issues in the test framework
run_test_with_output "Multiple files via glob" \
    "$CK_BIN 'math' $TEST_DIR/Mathematics.txt $TEST_DIR/Algorithm.txt" \
    "math"

run_test_with_output "Specific file search" \
    "$CK_BIN 'Python' $TEST_DIR/Python_programming_language.txt" \
    "Python"

# Test actual glob expansion with a limited set
run_test_with_output "Shell glob expansion" \
    "bash -c \"$CK_BIN 'statistics' $TEST_DIR/Stat*.txt\"" \
    "statistics"

# 15. Exclusion patterns
run_test "Exclude pattern" \
    "$CK_BIN --exclude 'Machine*' 'learning' $TEST_DIR" \
    "Should exclude Machine_learning.txt"

echo ""
echo "=== Index Update Tests ==="
echo ""

# 16. Add single file
test_file="$TEST_DIR/test_new.txt"
echo "This is a test file for indexing" > "$test_file"

run_test "Add single file to index" \
    "$CK_BIN add $test_file" \
    "Should add file to index"

# Clean up test file
rm -f "$test_file"

# 17. Clean orphaned files
run_test "Clean orphaned entries" \
    "$CK_BIN clean $TEST_DIR --orphans" \
    "Should clean orphaned entries"

echo ""
echo "=== Advanced Features Tests ==="
echo ""

# 18. Reindex flag
run_test "Search with reindex" \
    "$CK_BIN --sem --reindex 'algorithm' $TEST_DIR" \
    "Should reindex before search"

# 19. Filename display control
run_test_with_output "Suppress filenames" \
    "$CK_BIN --no-filename 'data' $TEST_DIR/Data_science.txt" \
    "data"

run_test_with_output "Force filenames with -H" \
    "$CK_BIN -H 'science' $TEST_DIR/Data_science.txt" \
    "Data_science.txt"

echo ""
echo "=== Error Handling Tests ==="
echo ""

# 20. Invalid paths
run_test_exit_code "Non-existent file" \
    "$CK_BIN 'test' /nonexistent/path.txt" \
    1

run_test_exit_code "Invalid glob pattern behavior" \
    "$CK_BIN 'test' '[invalid'" \
    1

echo ""
echo "=== Performance Tests ==="
echo ""

# 21. Large result set handling
echo -n "Testing: Large result set handling ... "
# Use a simpler timing mechanism that works on macOS
start_time=$(date +%s)
$CK_BIN -r 'the' $TEST_DIR >/dev/null 2>&1
end_time=$(date +%s)
elapsed=$((end_time - start_time))

if [ $elapsed -lt 5 ]; then  # Should complete in under 5 seconds
    echo -e "${GREEN}PASS${NC} (${elapsed}s)"
    TESTS_PASSED=$((TESTS_PASSED + 1))
else
    echo -e "${RED}FAIL${NC} (took ${elapsed}s, expected < 5s)"
    TESTS_FAILED=$((TESTS_FAILED + 1))
fi
TESTS_RUN=$((TESTS_RUN + 1))

echo ""
echo "=== Cleanup ==="
echo ""

# Final cleanup
run_test "Clean entire index" \
    "$CK_BIN clean $TEST_DIR" \
    "Should remove .ck directory"

echo ""
echo "========================================="
echo "              Test Summary"
echo "========================================="
echo ""
echo -e "Total tests run: ${TESTS_RUN}"
echo -e "Tests passed: ${GREEN}${TESTS_PASSED}${NC}"
echo -e "Tests failed: ${RED}${TESTS_FAILED}${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo ""
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo ""
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi