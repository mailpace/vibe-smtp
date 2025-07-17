# Justfile for Vibe Gateway
# Run with: just <command>
# Install just: cargo install just

# Justfile for Vibe Gateway
# Run with: just <command>
# Install just: cargo install just

# Default recipe
default: test

# Install dependencies
install:
    cargo install cargo-nextest --locked
    cargo install cargo-tarpaulin
    cargo install cargo-watch

# Build the project
build:
    cargo build

# Build for release
build-release:
    cargo build --release

# Run formatting check
format:
    cargo fmt --all -- --check

# Format code
format-fix:
    cargo fmt --all

# Run clippy
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
test: format clippy unit integration performance
    echo "All tests completed successfully!"

# Run unit tests
unit:
    RUST_LOG=info cargo nextest run --profile unit

# Run integration tests
integration:
    RUST_LOG=info cargo nextest run --profile integration

# Run performance tests
performance:
    RUST_LOG=info cargo nextest run --profile performance --release

# Run tests matching a pattern
pattern PATTERN:
    RUST_LOG=info cargo nextest run -E 'test({{PATTERN}})'

# Run tests with coverage
coverage:
    cargo tarpaulin --engine llvm --out Html --output-dir coverage/ --skip-clean
    echo "Coverage report generated in coverage/tarpaulin-report.html"

# Run tests in watch mode
watch:
    cargo watch -x "nextest run"

# Run security audit
security:
    cargo audit

# Show test results
results:
    echo "Test Results:"
    test -f "nextest/default/junit.xml" && echo "✓ JUnit XML: nextest/default/junit.xml" || true
    test -f "nextest/default/run-summary.json" && echo "✓ Run Summary: nextest/default/run-summary.json" || true
    test -f "coverage/tarpaulin-report.html" && echo "✓ Coverage: coverage/tarpaulin-report.html" || true

# Run tests with timing information
timing:
    RUST_LOG=info cargo nextest run --profile default --final-status-level slow

# Clean build artifacts
clean:
    cargo clean
    rm -rf coverage/
    rm -rf nextest/
    rm -rf target/nextest/

# CI workflow - optimized for continuous integration
ci: format clippy
    cargo nextest run --profile ci
    cargo tarpaulin --engine llvm --out xml --output-dir coverage/

# Development workflow - quick feedback
dev:
    cargo nextest run --profile unit
    cargo nextest run --profile integration

# Run specific test suite
test-suite SUITE:
    cargo nextest run --profile {{SUITE}}

# Generate documentation
docs:
    cargo doc --open

# Run benchmarks (if available)
bench:
    cargo bench

# Show help
help:
    just --list
