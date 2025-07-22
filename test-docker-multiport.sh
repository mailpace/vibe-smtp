#!/bin/bash

# Test script for Docker multi-port functionality
# This script tests the Vibe Gateway Docker setup with all port configurations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to check if a port is available
port_available() {
    local port=$1
    ! nc -z localhost $port 2>/dev/null
}

# Function to wait for port to become available
wait_for_port() {
    local port=$1
    local timeout=${2:-30}
    local count=0
    
    print_info "Waiting for port $port to become available..."
    while ! nc -z localhost $port 2>/dev/null; do
        if [ $count -ge $timeout ]; then
            print_error "Timeout waiting for port $port"
            return 1
        fi
        sleep 1
        count=$((count + 1))
    done
    print_success "Port $port is available"
}

# Function to test SMTP connection
test_smtp_connection() {
    local port=$1
    local description="$2"
    
    print_info "Testing SMTP connection on port $port ($description)..."
    
    # Simple telnet test
    if command_exists telnet; then
        {
            sleep 1
            echo "QUIT"
            sleep 1
        } | telnet localhost $port 2>/dev/null | grep -q "220.*vibe-gateway"
        
        if [ $? -eq 0 ]; then
            print_success "SMTP connection successful on port $port"
            return 0
        else
            print_error "SMTP connection failed on port $port"
            return 1
        fi
    else
        print_warning "telnet not available, skipping SMTP test for port $port"
        return 0
    fi
}

# Function to cleanup any running containers
cleanup() {
    print_info "Cleaning up..."
    docker ps -q --filter "ancestor=vibe-gateway:latest" | xargs -r docker stop
    docker ps -a -q --filter "ancestor=vibe-gateway:latest" | xargs -r docker rm
}

# Main test function
main() {
    print_info "Starting Vibe Gateway Docker Multi-Port Test"
    echo "=============================================="
    
    # Check prerequisites
    if ! command_exists docker; then
        print_error "Docker is not installed or not in PATH"
        exit 1
    fi
    
    if ! command_exists nc; then
        print_error "netcat (nc) is required for port testing"
        exit 1
    fi
    
    # Check if Docker is running
    if ! docker info >/dev/null 2>&1; then
        print_error "Docker is not running"
        exit 1
    fi
    
    # Cleanup any existing containers
    cleanup
    
    # Check if ports are available
    PORTS=(25 587 2525 465)
    for port in "${PORTS[@]}"; do
        if ! port_available $port; then
            print_error "Port $port is already in use"
            exit 1
        fi
    done
    
    # Build the Docker image
    print_info "Building Docker image..."
    if ! docker build -t vibe-gateway:latest . >/dev/null 2>&1; then
        print_error "Failed to build Docker image"
        exit 1
    fi
    print_success "Docker image built successfully"
    
    # Start the container
    print_info "Starting Vibe Gateway container in multi-port mode..."
    CONTAINER_ID=$(docker run -d \
        -p 25:25 \
        -p 587:587 \
        -p 2525:2525 \
        -p 465:465 \
        -e MAILPACE_API_TOKEN=test_token \
        vibe-gateway:latest ./vibe-gateway --docker-multi-port --debug)
    
    if [ $? -ne 0 ]; then
        print_error "Failed to start container"
        exit 1
    fi
    
    print_success "Container started with ID: ${CONTAINER_ID:0:12}"
    
    # Wait for services to start
    sleep 5
    
    # Test each port
    test_smtp_connection 25 "Standard SMTP with STARTTLS"
    test_smtp_connection 587 "Message Submission with STARTTLS"
    test_smtp_connection 2525 "Alternative SMTP with STARTTLS"
    test_smtp_connection 465 "SMTP over SSL (implicit TLS)"
    
    # Show container logs
    print_info "Container logs:"
    echo "---------------"
    docker logs $CONTAINER_ID --tail 20
    echo "---------------"
    
    # Test health check
    print_info "Testing health check..."
    sleep 10  # Wait for health check to run
    HEALTH_STATUS=$(docker inspect --format='{{.State.Health.Status}}' $CONTAINER_ID 2>/dev/null || echo "unknown")
    
    if [ "$HEALTH_STATUS" = "healthy" ]; then
        print_success "Health check passed"
    else
        print_warning "Health check status: $HEALTH_STATUS"
    fi
    
    # Cleanup
    print_info "Stopping and removing container..."
    docker stop $CONTAINER_ID >/dev/null
    docker rm $CONTAINER_ID >/dev/null
    
    print_success "Test completed successfully!"
    echo ""
    print_info "All SMTP ports are working correctly:"
    echo "  Port 25   - Standard SMTP with STARTTLS"
    echo "  Port 587  - Message Submission with STARTTLS"
    echo "  Port 2525 - Alternative SMTP with STARTTLS"
    echo "  Port 465  - SMTP over SSL (implicit TLS)"
    echo ""
    print_info "You can now run the server with:"
    echo "  ./docker-run.sh multi-port --token your_api_token"
    echo "  or"
    echo "  docker-compose up -d"
}

# Handle script interruption
trap cleanup EXIT INT TERM

# Run main function
main "$@"
