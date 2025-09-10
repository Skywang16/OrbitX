#!/bin/bash

# ck test suite - simplified version
# Usage: ./test_ck_simple.sh

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
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo -n "Testing: $test_name ... "
    
    if eval "$command" >/dev/null 2>&1; then
        echo -e "${GREEN}PASS${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}"
        echo "  Command: $command"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

run_test_with_output() {
    local test_name="$1"
    local command="$2"
    local expected_pattern="$3"
    
    TESTS_RUN=$((TESTS_RUN + 1))
    echo -n "Testing: $test_name ... "
    
    output=$(eval "$command" 2>&1)
    if echo "$output" | grep -q "$expected_pattern"; then
        echo -e "${GREEN}PASS${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}FAIL${NC}"
        echo "  Command: $command"
        echo "  Expected pattern: $expected_pattern"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
}

echo "========================================="
echo "      ck Test Suite (Simplified)"
echo "========================================="
echo ""

# Check if binary exists; build if missing
if [ ! -f "$CK_BIN" ]; then
    echo -e "${YELLOW}ck binary not found at $CK_BIN, attempting to build...${NC}"
    if ! cargo build >/dev/null 2>&1; then
        echo -e "${RED}Error: build failed${NC}"
        exit 1
    fi
fi

echo "Using binary: $CK_BIN"
echo "Test directory: $TEST_DIR"
echo ""

# Clean up any existing index
$CK_BIN clean $TEST_DIR 2>/dev/null || true

echo "=== Core Grep Tests ==="
run_test_with_output "Basic search" "$CK_BIN 'algorithm' $TEST_DIR/Algorithm.txt" "algorithm"
run_test_with_output "Case insensitive" "$CK_BIN -i 'ALGORITHM' $TEST_DIR/Algorithm.txt" "algorithm"
run_test_with_output "Line numbers" "$CK_BIN -n 'algorithm' $TEST_DIR/Algorithm.txt" "[0-9]:"
run_test_with_output "Recursive search" "$CK_BIN -r 'Python' $TEST_DIR" "Python"
run_test "Context lines" "$CK_BIN -C 2 'machine' $TEST_DIR/Machine_learning.txt"

echo ""
echo "=== Index Management ==="
run_test "Create index" "$CK_BIN index $TEST_DIR"
run_test_with_output "Check status" "$CK_BIN status $TEST_DIR" "Files indexed:"

echo ""
echo "=== Semantic Search ==="
run_test_with_output "Semantic search" "$CK_BIN --sem 'artificial intelligence' $TEST_DIR" "intelligence"
run_test_with_output "Semantic with scores" "$CK_BIN --sem --scores 'machine learning' $TEST_DIR" "\["
run_test "Semantic with top-k" "$CK_BIN --sem --topk 5 'statistics' $TEST_DIR"

echo ""
echo "=== Lexical Search ==="
run_test_with_output "Lexical search" "$CK_BIN --lex 'database' $TEST_DIR" "database"

echo ""
echo "=== Hybrid Search ==="
run_test_with_output "Hybrid search" "$CK_BIN --hybrid 'algorithm' $TEST_DIR" "algorithm"

echo ""
echo "=== JSON Output ==="
run_test_with_output "JSON format" "$CK_BIN --json --sem 'python' $TEST_DIR" '"file"'

echo ""
echo "=== Cleanup ==="
run_test "Clean index" "$CK_BIN clean $TEST_DIR"

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
    echo -e "${RED}✗ ${TESTS_FAILED} tests failed${NC}"
    exit 1
fi