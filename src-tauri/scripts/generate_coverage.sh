#!/bin/bash

# 测试覆盖率生成脚本
# 
# 使用 cargo-tarpaulin 生成详细的测试覆盖率报告

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 打印带颜色的消息
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

# 检查是否安装了 cargo-tarpaulin
check_tarpaulin() {
    if ! command -v cargo-tarpaulin &> /dev/null; then
        print_warning "cargo-tarpaulin 未安装，正在安装..."
        cargo install cargo-tarpaulin
        if [ $? -eq 0 ]; then
            print_success "cargo-tarpaulin 安装成功"
        else
            print_error "cargo-tarpaulin 安装失败"
            exit 1
        fi
    else
        print_info "cargo-tarpaulin 已安装"
    fi
}

# 清理之前的覆盖率报告
cleanup_previous() {
    print_info "清理之前的覆盖率报告..."
    rm -rf target/tarpaulin
    rm -f cobertura.xml
    rm -f tarpaulin-report.html
    rm -f lcov.info
}

# 运行测试覆盖率分析
run_coverage() {
    print_info "开始生成测试覆盖率报告..."
    
    # 使用 tarpaulin 生成覆盖率报告
    cargo tarpaulin \
        --config tarpaulin.toml \
        --out Html \
        --out Lcov \
        --out Json \
        --output-dir target/tarpaulin \
        --timeout 120 \
        --all-features \
        --all-targets \
        --verbose \
        --fail-under 70 \
        --branch \
        --count \
        --ignore-panics \
        --inline \
        --macros \
        --orphaned \
        --unused
    
    if [ $? -eq 0 ]; then
        print_success "测试覆盖率报告生成成功"
    else
        print_error "测试覆盖率报告生成失败"
        exit 1
    fi
}

# 生成覆盖率摘要
generate_summary() {
    print_info "生成覆盖率摘要..."
    
    # 检查是否存在 JSON 报告
    if [ -f "target/tarpaulin/tarpaulin-report.json" ]; then
        # 使用 jq 解析 JSON 报告（如果安装了 jq）
        if command -v jq &> /dev/null; then
            echo "=== 测试覆盖率摘要 ===" > target/tarpaulin/coverage_summary.txt
            echo "" >> target/tarpaulin/coverage_summary.txt
            
            # 提取总体覆盖率
            total_coverage=$(jq -r '.files | map(.coverage) | add / length' target/tarpaulin/tarpaulin-report.json 2>/dev/null || echo "N/A")
            echo "总体覆盖率: ${total_coverage}%" >> target/tarpaulin/coverage_summary.txt
            echo "" >> target/tarpaulin/coverage_summary.txt
            
            # 提取各模块覆盖率
            echo "各模块覆盖率:" >> target/tarpaulin/coverage_summary.txt
            jq -r '.files[] | "\(.name): \(.coverage)%"' target/tarpaulin/tarpaulin-report.json 2>/dev/null >> target/tarpaulin/coverage_summary.txt || echo "无法解析模块覆盖率" >> target/tarpaulin/coverage_summary.txt
            
            print_success "覆盖率摘要已生成: target/tarpaulin/coverage_summary.txt"
        else
            print_warning "jq 未安装，跳过 JSON 报告解析"
        fi
    else
        print_warning "未找到 JSON 报告文件"
    fi
}

# 显示报告位置
show_report_locations() {
    print_info "测试覆盖率报告位置:"
    echo "  HTML 报告: target/tarpaulin/tarpaulin-report.html"
    echo "  LCOV 报告: target/tarpaulin/lcov.info"
    echo "  JSON 报告: target/tarpaulin/tarpaulin-report.json"
    
    if [ -f "target/tarpaulin/coverage_summary.txt" ]; then
        echo "  覆盖率摘要: target/tarpaulin/coverage_summary.txt"
    fi
}

# 打开 HTML 报告（可选）
open_html_report() {
    if [ "$1" = "--open" ] || [ "$1" = "-o" ]; then
        if [ -f "target/tarpaulin/tarpaulin-report.html" ]; then
            print_info "正在打开 HTML 覆盖率报告..."
            
            # 根据操作系统选择打开命令
            case "$(uname -s)" in
                Darwin)
                    open target/tarpaulin/tarpaulin-report.html
                    ;;
                Linux)
                    xdg-open target/tarpaulin/tarpaulin-report.html
                    ;;
                CYGWIN*|MINGW32*|MSYS*|MINGW*)
                    start target/tarpaulin/tarpaulin-report.html
                    ;;
                *)
                    print_warning "无法自动打开报告，请手动打开: target/tarpaulin/tarpaulin-report.html"
                    ;;
            esac
        else
            print_error "HTML 报告文件不存在"
        fi
    fi
}

# 主函数
main() {
    print_info "开始生成测试覆盖率报告..."
    echo "========================================"
    
    # 检查依赖
    check_tarpaulin
    
    # 清理之前的报告
    cleanup_previous
    
    # 运行覆盖率分析
    run_coverage
    
    # 生成摘要
    generate_summary
    
    # 显示报告位置
    echo "========================================"
    show_report_locations
    
    # 打开 HTML 报告（如果指定了参数）
    open_html_report "$1"
    
    print_success "测试覆盖率报告生成完成！"
}

# 显示帮助信息
show_help() {
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -h, --help    显示此帮助信息"
    echo "  -o, --open    生成报告后自动打开 HTML 报告"
    echo ""
    echo "示例:"
    echo "  $0              # 生成覆盖率报告"
    echo "  $0 --open       # 生成报告并打开 HTML 报告"
}

# 解析命令行参数
case "$1" in
    -h|--help)
        show_help
        exit 0
        ;;
    *)
        main "$1"
        ;;
esac
