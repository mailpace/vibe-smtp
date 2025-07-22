#!/bin/bash

# Docker Startup Script for Vibe Gateway
# This script provides an easy way to run the Vibe Gateway in different Docker configurations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [MODE] [OPTIONS]"
    echo ""
    echo "MODES:"
    echo "  multi-port    Run with all SMTP ports (25, 587, 2525, 465) [DEFAULT]"
    echo "  single-port   Run with single port (2525)"
    echo "  build         Build the Docker image"
    echo "  help          Show this help message"
    echo ""
    echo "OPTIONS:"
    echo "  --token TOKEN Set the MailPace API token"
    echo "  --env-file    Use custom .env file"
    echo "  --dev         Run in development mode with debug logging"
    echo ""
    echo "EXAMPLES:"
    echo "  $0 multi-port --token your_api_token"
    echo "  $0 single-port --dev"
    echo "  $0 build"
    echo ""
    echo "SMTP PORT CONFIGURATIONS:"
    echo "  Port 25   - Standard SMTP with STARTTLS support"
    echo "  Port 587  - Message Submission with STARTTLS support"
    echo "  Port 2525 - Alternative SMTP with STARTTLS support" 
    echo "  Port 465  - SMTP over SSL (implicit TLS, no STARTTLS)"
}

# Function to check if Docker is running
check_docker() {
    if ! docker info > /dev/null 2>&1; then
        print_error "Docker is not running. Please start Docker and try again."
        exit 1
    fi
}

# Function to build the image
build_image() {
    print_status "Building Vibe Gateway Docker image..."
    docker build -t vibe-gateway:latest .
    print_status "Image built successfully!"
}

# Function to run multi-port mode
run_multi_port() {
    local token="$1"
    local env_file="$2"
    local debug="$3"
    
    print_status "Starting Vibe Gateway in multi-port mode..."
    print_status "Ports: 25 (SMTP+STARTTLS), 587 (Submission+STARTTLS), 2525 (Alt+STARTTLS), 465 (Implicit TLS)"
    
    local docker_args=()
    docker_args+=("-p" "25:25")
    docker_args+=("-p" "587:587") 
    docker_args+=("-p" "2525:2525")
    docker_args+=("-p" "465:465")
    
    if [[ -n "$token" ]]; then
        docker_args+=("-e" "MAILPACE_API_TOKEN=$token")
    elif [[ -f "$env_file" ]]; then
        docker_args+=("--env-file" "$env_file")
    elif [[ -f ".env" ]]; then
        docker_args+=("--env-file" ".env")
        print_status "Using .env file for configuration"
    else
        print_warning "No API token provided. Users must authenticate with their own tokens via SMTP AUTH."
    fi
    
    local cmd_args=("--docker-multi-port")
    if [[ "$debug" == "true" ]]; then
        cmd_args+=("--debug")
    fi
    
    docker run --rm -it "${docker_args[@]}" vibe-gateway:latest ./vibe-gateway "${cmd_args[@]}"
}

# Function to run single-port mode
run_single_port() {
    local token="$1"
    local env_file="$2"
    local debug="$3"
    
    print_status "Starting Vibe Gateway in single-port mode (port 2525)..."
    
    local docker_args=("-p" "2525:2525")
    
    if [[ -n "$token" ]]; then
        docker_args+=("-e" "MAILPACE_API_TOKEN=$token")
    elif [[ -f "$env_file" ]]; then
        docker_args+=("--env-file" "$env_file")
    elif [[ -f ".env" ]]; then
        docker_args+=("--env-file" ".env")
        print_status "Using .env file for configuration"
    else
        print_warning "No API token provided. Users must authenticate with their own tokens via SMTP AUTH."
    fi
    
    local cmd_args=("--listen" "0.0.0.0:2525" "--enable-tls")
    if [[ "$debug" == "true" ]]; then
        cmd_args+=("--debug")
    fi
    
    docker run --rm -it "${docker_args[@]}" vibe-gateway:latest ./vibe-gateway "${cmd_args[@]}"
}

# Parse command line arguments
MODE="multi-port"
TOKEN=""
ENV_FILE=""
DEBUG="false"

while [[ $# -gt 0 ]]; do
    case $1 in
        multi-port|single-port|build|help)
            MODE="$1"
            shift
            ;;
        --token)
            TOKEN="$2"
            shift 2
            ;;
        --env-file)
            ENV_FILE="$2"
            shift 2
            ;;
        --dev)
            DEBUG="true"
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Main execution
check_docker

case $MODE in
    help)
        show_usage
        ;;
    build)
        build_image
        ;;
    multi-port)
        build_image
        run_multi_port "$TOKEN" "$ENV_FILE" "$DEBUG"
        ;;
    single-port)
        build_image
        run_single_port "$TOKEN" "$ENV_FILE" "$DEBUG"
        ;;
    *)
        print_error "Invalid mode: $MODE"
        show_usage
        exit 1
        ;;
esac
