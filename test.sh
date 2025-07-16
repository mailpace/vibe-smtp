#!/bin/bash

# Test runner script for Vibe Gateway
# This script provides convenient commands for running different test suites

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${GREEN}=== $1 ===${NC}"
}

print_warning() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

print_error() {
    echo -e "${RED}ERROR: $1${NC}"
}

print_success() {
    echo -e "${GREEN}SUCCESS: $1${NC}"
}

# Function to run tests with proper error handling
run_test() {
    local test_name=$1
    local test_command=$2
    
    print_header "Running $test_name"
    
    if eval "$test_command"; then
        print_success "$test_name completed successfully"
        return 0
    else
        print_error "$test_name failed"
        return 1
    fi
}

# Function to check if cargo is available
check_cargo() {
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed. Please install Rust and Cargo first."
        exit 1
    fi
}

# Function to build the project
build_project() {
    print_header "Building project"
    cargo build --release
}

# Function to run formatting check
check_format() {
    run_test "Format Check" "cargo fmt --all -- --check"
}

# Function to run clippy
check_clippy() {
    run_test "Clippy Check" "cargo clippy --all-targets --all-features -- -D warnings"
}

# Function to run unit tests
run_unit_tests() {
    run_test "Unit Tests" "cargo test --lib"
}

# Function to run integration tests
run_integration_tests() {
    run_test "Integration Tests" "RUST_LOG=info cargo test --test integration_tests"
}

# Function to run performance tests
run_performance_tests() {
    run_test "Performance Tests" "cargo test --test performance_tests --release"
}

# Function to run all tests
run_all_tests() {
    local failed=0
    
    check_format || failed=1
    check_clippy || failed=1
    run_unit_tests || failed=1
    run_integration_tests || failed=1
    run_performance_tests || failed=1
    
    if [ $failed -eq 0 ]; then
        print_success "All tests passed!"
    else
        print_error "Some tests failed. Check the output above."
        exit 1
    fi
}

# Function to run tests with coverage
run_coverage() {
    print_header "Running tests with coverage"
    
    if ! command -v cargo-tarpaulin &> /dev/null; then
        print_warning "cargo-tarpaulin not found. Installing..."
        cargo install cargo-tarpaulin
    fi
    
    cargo tarpaulin --out Html --output-dir coverage/
    print_success "Coverage report generated in coverage/tarpaulin-report.html"
}

# Function to run security audit
run_security_audit() {
    print_header "Running security audit"
    
    if ! command -v cargo-audit &> /dev/null; then
        print_warning "cargo-audit not found. Installing..."
        cargo install cargo-audit
    fi
    
    cargo audit
}

# Function to clean up test artifacts
cleanup() {
    print_header "Cleaning up"
    cargo clean
    rm -rf coverage/
    print_success "Cleanup completed"
}

# Function to show help
show_help() {
    cat << EOF
Vibe Gateway Test Runner

Usage: $0 [COMMAND]

Commands:
    all             Run all tests and checks (default)
    format          Check code formatting
    clippy          Run clippy linting
    unit            Run unit tests
    integration     Run integration tests
    performance     Run performance tests
    coverage        Run tests with coverage report
    security        Run security audit
    build           Build the project
    clean           Clean up build artifacts
    help            Show this help message

Examples:
    $0                  # Run all tests
    $0 integration      # Run only integration tests
    $0 coverage         # Generate coverage report
    $0 security         # Run security audit

Environment Variables:
    RUST_LOG           Set logging level (error, warn, info, debug, trace)
    
For more detailed information, see TESTING.md
EOF
}

# Main script logic
main() {
    check_cargo
    
    case "${1:-all}" in
        "all")
            run_all_tests
            ;;
        "format")
            check_format
            ;;
        "clippy")
            check_clippy
            ;;
        "unit")
            run_unit_tests
            ;;
        "integration")
            run_integration_tests
            ;;
        "performance")
            run_performance_tests
            ;;
        "coverage")
            run_coverage
            ;;
        "security")
            run_security_audit
            ;;
        "build")
            build_project
            ;;
        "clean")
            cleanup
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            print_error "Unknown command: $1"
            show_help
            exit 1
            ;;
    esac
}

# Run main function with all arguments
main "$@"
