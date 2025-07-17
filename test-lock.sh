#!/bin/bash

# Test synchronization helper to prevent cargo lock conflicts

set -e

LOCK_FILE="/tmp/vibe-gateway-test.lock"

# Function to acquire lock
acquire_lock() {
    local timeout=300  # 5 minutes timeout
    local elapsed=0
    
    while [ $elapsed -lt $timeout ]; do
        if (set -C; echo $$ > "$LOCK_FILE") 2>/dev/null; then
            return 0
        fi
        sleep 1
        elapsed=$((elapsed + 1))
    done
    
    echo "Failed to acquire lock after $timeout seconds"
    return 1
}

# Function to release lock
release_lock() {
    rm -f "$LOCK_FILE"
}

# Trap to ensure lock is released on exit
trap release_lock EXIT

# Main function
main() {
    echo "Acquiring test lock..."
    acquire_lock
    
    echo "Running test command: $*"
    exec "$@"
}

main "$@"
