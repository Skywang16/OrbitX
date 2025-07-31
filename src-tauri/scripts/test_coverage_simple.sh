#!/bin/bash

# 简化的测试覆盖率检查脚本
# 
# 使用 cargo test 和基本的统计来评估测试覆盖情况

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_info() {
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

# 运行所有测试并收集统计信息
run_tests_with_stats() {
    print_info "运行所有测试并收集统计信息..."
    
    # 创建输出目录
    mkdir -p target/test-reports
    
    # 运行测试并保存输出
    cargo test --all-features --all-targets -- --nocapture > target/test-reports/test_output.txt 2>&1
    local test_exit_code=$?
    
    if [ $test_exit_code -eq 0 ]; then
        print_success "所有测试通过"
    else
        print_warning "部分测试失败，但继续生成报告"
    fi
    
    return $test_exit_code
}

# 分析测试覆盖情况
analyze_test_coverage() {
    print_info "分析测试覆盖情况..."
    
    local output_file="target/test-reports/test_output.txt"
    local report_file="target/test-reports/coverage_analysis.txt"
    
    echo "=== 测试覆盖分析报告 ===" > "$report_file"
    echo "生成时间: $(date)" >> "$report_file"
    echo "" >> "$report_file"
    
    # 统计测试数量
    local total_tests=$(grep -c "test result:" "$output_file" 2>/dev/null || echo "0")
    local passed_tests=$(grep "test result:" "$output_file" | grep -o "[0-9]\+ passed" | grep -o "[0-9]\+" | head -1 || echo "0")
    local failed_tests=$(grep "test result:" "$output_file" | grep -o "[0-9]\+ failed" | grep -o "[0-9]\+" | head -1 || echo "0")
    local ignored_tests=$(grep "test result:" "$output_file" | grep -o "[0-9]\+ ignored" | grep -o "[0-9]\+" | head -1 || echo "0")
    
    echo "=== 测试执行统计 ===" >> "$report_file"
    echo "总测试数: $total_tests" >> "$report_file"
    echo "通过测试: $passed_tests" >> "$report_file"
    echo "失败测试: $failed_tests" >> "$report_file"
    echo "忽略测试: $ignored_tests" >> "$report_file"
    echo "" >> "$report_file"
    
    # 计算通过率
    if [ "$total_tests" -gt 0 ]; then
        local pass_rate=$(echo "scale=2; $passed_tests * 100 / $total_tests" | bc -l 2>/dev/null || echo "N/A")
        echo "测试通过率: ${pass_rate}%" >> "$report_file"
    else
        echo "测试通过率: N/A" >> "$report_file"
    fi
    echo "" >> "$report_file"
    
    # 分析测试模块
    echo "=== 测试模块分析 ===" >> "$report_file"
    grep "running.*tests" "$output_file" | while read line; do
        echo "$line" >> "$report_file"
    done
    echo "" >> "$report_file"
    
    # 查找失败的测试
    if [ "$failed_tests" -gt 0 ]; then
        echo "=== 失败的测试 ===" >> "$report_file"
        grep -A 5 "FAILED" "$output_file" >> "$report_file" 2>/dev/null || echo "无法提取失败测试详情" >> "$report_file"
        echo "" >> "$report_file"
    fi
    
    print_success "测试覆盖分析完成: $report_file"
}

# 检查源代码文件和测试文件的比例
analyze_code_test_ratio() {
    print_info "分析代码与测试文件比例..."
    
    local report_file="target/test-reports/coverage_analysis.txt"
    
    echo "=== 代码与测试文件比例 ===" >> "$report_file"
    
    # 统计源代码文件
    local src_files=$(find src -name "*.rs" | wc -l)
    local src_lines=$(find src -name "*.rs" -exec wc -l {} + | tail -1 | awk '{print $1}' || echo "0")
    
    # 统计测试文件
    local test_files=$(find tests -name "*.rs" 2>/dev/null | wc -l || echo "0")
    local test_lines=$(find tests -name "*.rs" -exec wc -l {} + 2>/dev/null | tail -1 | awk '{print $1}' || echo "0")
    
    # 统计内联测试
    local inline_tests=$(grep -r "#\[test\]" src 2>/dev/null | wc -l || echo "0")
    
    echo "源代码文件数: $src_files" >> "$report_file"
    echo "源代码行数: $src_lines" >> "$report_file"
    echo "测试文件数: $test_files" >> "$report_file"
    echo "测试代码行数: $test_lines" >> "$report_file"
    echo "内联测试数: $inline_tests" >> "$report_file"
    
    # 计算比例
    if [ "$src_files" -gt 0 ]; then
        local file_ratio=$(echo "scale=2; $test_files * 100 / $src_files" | bc -l 2>/dev/null || echo "N/A")
        echo "测试文件覆盖率: ${file_ratio}%" >> "$report_file"
    fi
    
    if [ "$src_lines" -gt 0 ]; then
        local line_ratio=$(echo "scale=2; $test_lines * 100 / $src_lines" | bc -l 2>/dev/null || echo "N/A")
        echo "测试代码比例: ${line_ratio}%" >> "$report_file"
    fi
    
    echo "" >> "$report_file"
}

# 检查未覆盖的模块
check_uncovered_modules() {
    print_info "检查可能未覆盖的模块..."
    
    local report_file="target/test-reports/coverage_analysis.txt"
    
    echo "=== 模块覆盖检查 ===" >> "$report_file"
    
    # 查找所有源代码模块
    find src -name "*.rs" | while read file; do
        local module_name=$(basename "$file" .rs)
        local module_path=$(dirname "$file")
        
        # 检查是否有对应的测试
        local has_test=false
        
        # 检查是否有专门的测试文件
        if [ -f "tests/${module_name}_test.rs" ] || [ -f "tests/${module_path}/${module_name}_test.rs" ]; then
            has_test=true
        fi
        
        # 检查是否有内联测试
        if grep -q "#\[test\]" "$file" 2>/dev/null; then
            has_test=true
        fi
        
        # 检查是否在集成测试中被测试
        if grep -r "use.*${module_name}" tests/ 2>/dev/null | grep -q "\.rs:"; then
            has_test=true
        fi
        
        if [ "$has_test" = false ]; then
            echo "可能未覆盖: $file" >> "$report_file"
        fi
    done
    
    echo "" >> "$report_file"
}

# 生成改进建议
generate_recommendations() {
    print_info "生成测试改进建议..."
    
    local report_file="target/test-reports/coverage_analysis.txt"
    
    echo "=== 测试改进建议 ===" >> "$report_file"
    
    # 基于分析结果生成建议
    local passed_tests=$(grep "通过测试:" "$report_file" | grep -o "[0-9]\+" || echo "0")
    local failed_tests=$(grep "失败测试:" "$report_file" | grep -o "[0-9]\+" || echo "0")
    local test_files=$(grep "测试文件数:" "$report_file" | grep -o "[0-9]\+" || echo "0")
    
    if [ "$failed_tests" -gt 0 ]; then
        echo "1. 修复失败的测试用例" >> "$report_file"
    fi
    
    if [ "$test_files" -lt 5 ]; then
        echo "2. 增加更多的测试文件来覆盖不同模块" >> "$report_file"
    fi
    
    echo "3. 考虑添加集成测试来验证模块间的交互" >> "$report_file"
    echo "4. 添加性能测试来确保代码性能" >> "$report_file"
    echo "5. 增加错误场景的测试覆盖" >> "$report_file"
    echo "6. 考虑使用 cargo-tarpaulin 获取更详细的覆盖率信息" >> "$report_file"
    
    echo "" >> "$report_file"
}

# 显示报告摘要
show_summary() {
    local report_file="target/test-reports/coverage_analysis.txt"
    
    print_info "测试覆盖分析摘要:"
    echo "========================================"
    
    if [ -f "$report_file" ]; then
        # 显示关键统计信息
        grep -E "(总测试数|通过测试|失败测试|测试通过率|测试文件数|源代码文件数)" "$report_file" | while read line; do
            echo "  $line"
        done
    fi
    
    echo "========================================"
    echo "详细报告: $report_file"
    echo "测试输出: target/test-reports/test_output.txt"
}

# 主函数
main() {
    print_info "开始测试覆盖分析..."
    echo "========================================"
    
    # 运行测试
    run_tests_with_stats
    local test_result=$?
    
    # 分析覆盖情况
    analyze_test_coverage
    analyze_code_test_ratio
    check_uncovered_modules
    generate_recommendations
    
    # 显示摘要
    show_summary
    
    if [ $test_result -eq 0 ]; then
        print_success "测试覆盖分析完成！"
    else
        print_warning "测试覆盖分析完成，但有测试失败"
    fi
    
    return $test_result
}

# 显示帮助信息
show_help() {
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help    显示此帮助信息"
    echo ""
    echo "此脚本会:"
    echo "  1. 运行所有测试"
    echo "  2. 分析测试覆盖情况"
    echo "  3. 检查代码与测试的比例"
    echo "  4. 识别可能未覆盖的模块"
    echo "  5. 生成改进建议"
}

# 解析命令行参数
case "$1" in
    -h|--help)
        show_help
        exit 0
        ;;
    *)
        main
        ;;
esac
