#!/bin/bash

# Debug script to understand the cargo lock issue

set -e

echo "=== Debugging cargo lock issue ==="

echo "1. Checking for existing cargo processes..."
ps aux | grep -i cargo | grep -v grep || echo "No cargo processes found"

echo "2. Checking for lock files..."
find . -name "*.lock" -type f | head -10

echo "3. Checking target directory..."
ls -la target/ || echo "No target directory"

echo "4. Running simple cargo check..."
timeout 30 cargo check || echo "Cargo check failed or timed out"

echo "5. Trying to build..."
timeout 60 cargo build --release || echo "Cargo build failed or timed out"

echo "6. Checking if binary exists..."
ls -la target/release/vibe-gateway || echo "Binary not found"

echo "=== Debug complete ==="
