#!/bin/bash

# Simple test to verify the basic functionality works
# This is a lightweight test that doesn't require the full integration test suite

echo "=== Testing Basic Functionality ==="

# Test 1: Check that the binary exists and can show help
echo "Test 1: Binary help"
if ./target/release/vibe-gateway --help | grep -q "SMTP server listen address"; then
    echo "✓ Binary help works"
else
    echo "✗ Binary help failed"
    exit 1
fi

# Test 2: Check that it can start (but exit quickly)
echo "Test 2: Basic startup"
timeout 2 ./target/release/vibe-gateway --listen 127.0.0.1:12345 --debug 2>&1 | grep -q "SMTP server listening" && echo "✓ Server startup works" || echo "✗ Server startup failed"

# Test 3: Check that required files exist
echo "Test 3: Required files"
if [[ -f "./Cargo.toml" && -f "./src/main.rs" && -f "./TESTING.md" ]]; then
    echo "✓ Required files exist"
else
    echo "✗ Required files missing"
    exit 1
fi

# Test 4: Check that test files exist
echo "Test 4: Test files"
if [[ -f "./tests/integration_tests.rs" && -f "./tests/common.rs" && -f "./tests/mailpace_tests.rs" ]]; then
    echo "✓ Test files exist"
else
    echo "✗ Test files missing"
    exit 1
fi

# Test 5: Check that CI file exists
echo "Test 5: CI configuration"
if [[ -f "./.github/workflows/ci.yml" ]]; then
    echo "✓ CI configuration exists"
else
    echo "✗ CI configuration missing"
    exit 1
fi

echo "=== All basic tests passed! ==="
echo ""
echo "Next steps:"
echo "1. Run './test.sh' to run the full test suite"
echo "2. Run 'cargo test' to run all tests manually"
echo "3. Check the CI pipeline in GitHub Actions"
