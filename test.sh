#!/bin/bash

# Test runner script for Vibe Gateway using cargo-nextest
# This script provides convenient commands for running different test suites with nextest

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_header() {
    echo -e "${BLUE}=== $1 ===${NC}"
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

# Function to check if cargo and cargo-nextest are available
check_dependencies() {
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed. Please install Rust and Cargo first."
        exit 1
    fi
    
    if ! command -v cargo-nextest &> /dev/null; then
        print_warning "cargo-nextest is not installed. Installing..."
        cargo install cargo-nextest --locked
    fi
}

# Function to install cargo-nextest if not present
install_nextest() {
    if ! command -v cargo-nextest &> /dev/null; then
        print_header "Installing cargo-nextest"
        cargo install cargo-nextest --locked
        print_success "cargo-nextest installed successfully"
    else
        print_success "cargo-nextest is already installed"
    fi
}

# Function to build the project
build_project() {
    print_header "Building project"
    cargo build --release
}

# Function to run unit tests
run_unit_tests() {
    run_test "Unit Tests" "RUST_LOG=info cargo nextest run --profile unit"
}

# Function to run integration tests
run_integration_tests() {
    # Build the project first to avoid cargo lock issues during test
    print_header "Building project before integration tests"
    cargo build --release
    
    run_test "Integration Tests" "RUST_LOG=info cargo nextest run --profile integration --jobs 1"
}

# Function to run performance tests
run_performance_tests() {
    run_test "Performance Tests" "RUST_LOG=info cargo nextest run --profile performance --release"
}

# Function to run all tests with nextest
run_all_tests_nextest() {
    local failed=0
    
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

# Function to run specific test patterns
run_test_pattern() {
    local pattern=$1
    if [ -z "$pattern" ]; then
        print_error "Test pattern is required"
        exit 1
    fi
    
    run_test "Tests matching pattern '$pattern'" "RUST_LOG=info cargo nextest run '$pattern'"
}

# Function to run tests with timing information
run_tests_with_timing() {
    run_test "Tests with timing" "RUST_LOG=info cargo nextest run --profile default --final-status-level slow"
}

# Function to run all tests
run_all_tests() {
    run_all_tests_nextest
}

# Function to run tests with coverage using nextest
run_coverage() {
    print_header "Running tests with coverage using nextest"
    
    if ! command -v cargo-tarpaulin &> /dev/null; then
        print_warning "cargo-tarpaulin not found. Installing..."
        cargo install cargo-tarpaulin
    fi
    
    # Use tarpaulin with nextest for better coverage
    cargo tarpaulin --engine llvm --out Html --output-dir coverage/ --skip-clean
    print_success "Coverage report generated in coverage/tarpaulin-report.html"
}

# Function to run tests in watch mode
run_watch() {
    print_header "Running tests in watch mode"
    
    if ! command -v cargo-watch &> /dev/null; then
        print_warning "cargo-watch not found. Installing..."
        cargo install cargo-watch
    fi
    
    cargo watch -x "nextest run"
}

# Function to show test results in different formats
show_test_results() {
    print_header "Showing detailed test results"
    
    if [ -f "nextest/default/junit.xml" ]; then
        print_success "JUnit XML report available at: nextest/default/junit.xml"
    fi
    
    if [ -f "nextest/default/binaries-list.json" ]; then
        print_success "Binary list available at: nextest/default/binaries-list.json"
    fi
    
    # Show recent test run summary
    if [ -f "nextest/default/run-summary.json" ]; then
        print_success "Run summary available at: nextest/default/run-summary.json"
    fi
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
    rm -rf nextest/
    rm -rf target/nextest/
    print_success "Cleanup completed"
}

# Function to show help
show_help() {
    cat << EOF
Vibe Gateway Test Runner (using cargo-nextest)

Usage: $0 [COMMAND] [OPTIONS]

Commands:
    all             Run all tests and checks (default)
    unit            Run unit tests only
    integration     Run integration tests only
    performance     Run performance tests only
    coverage        Run tests with coverage report
    security        Run security audit
    build           Build the project
    clean           Clean up build artifacts
    watch           Run tests in watch mode
    pattern <PATTERN> Run tests matching pattern
    timing          Run tests with detailed timing information
    results         Show test results and reports
    install         Install cargo-nextest
    help            Show this help message

Nextest Features:
    - Faster test execution with intelligent parallelization
    - Better test isolation and retry mechanisms
    - Detailed timing and performance metrics
    - Multiple output formats (JSON, JUnit XML)
    - Configurable test profiles for different scenarios

Examples:
    $0                          # Run all tests
    $0 integration              # Run only integration tests
    $0 pattern "smtp"           # Run tests matching "smtp"
    $0 coverage                 # Generate coverage report
    $0 watch                    # Run tests in watch mode
    $0 timing                   # Show detailed timing information

Environment Variables:
    RUST_LOG                    Set logging level (error, warn, info, debug, trace)
    NEXTEST_PROFILE             Override the nextest profile to use
    
Configuration:
    Nextest configuration is in .config/nextest.toml
    Test profiles: default, ci, integration, performance, unit
    
For more detailed information, see TESTING.md
EOF
}

# Main script logic
main() {
    check_dependencies
    
    case "${1:-all}" in
        "all")
            run_all_tests
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
        "watch")
            run_watch
            ;;
        "pattern")
            run_test_pattern "$2"
            ;;
        "timing")
            run_tests_with_timing
            ;;
        "results")
            show_test_results
            ;;
        "install")
            install_nextest
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
