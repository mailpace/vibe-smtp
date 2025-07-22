<div align="center">
  <img src="https://docs.mailpace.com/img/logo.png" alt="MailPace Logo" width="300" />
  
  # Vibe Gateway - SMTP to MailPace Bridge
  
  ### A high-performance Rust SMTP server that seamlessly bridges email delivery to the MailPace API
  
  [![CI](https://github.com/mailpace/vibe-smtp/actions/workflows/ci.yml/badge.svg)](https://github.com/mailpace/vibe-smtp/actions/workflows/ci.yml)
  [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
  [![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
  [![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](https://hub.docker.com)
  
  [**Website**](https://mailpace.com) • [**Documentation**](https://docs.mailpace.com) • [**API Reference**](https://docs.mailpace.com/reference/send) • [**Support**](mailto:support@mailpace.com)
</div>

---

## Overview

A production-ready Rust SMTP server that accepts emails and forwards them to the MailPace API with enterprise-grade reliability and performance.

## 📑 Table of Contents

- [Key Features](#-key-features)
- [Quick Start](#-quick-start)
- [Configuration](#-configuration)
- [Authentication](#-authentication)
- [SMTP Client Configuration](#-smtp-client-configuration)
- [Usage](#usage)
- [MailPace Features](#mailpace-features)
- [Attachment Support](#attachment-support)
- [HTML Compression](#html-compression)
- [Error Handling](#error-handling)
- [Development](#-development)
- [Testing](#-testing)
- [License](#-license)

## ✨ Key Features

- 🚀 **High-Performance SMTP Server** - Built with Rust for maximum throughput and reliability
- 🔐 **Enterprise Authentication** - Full SMTP authentication with MailPace API token integration
- 📎 **Smart Attachment Handling** - Automatic MIME parsing with configurable size limits
- 🗜️ **HTML Compression** - Intelligent compression optimized for email clients
- 🔒 **TLS/STARTTLS Support** - Secure email transmission with modern encryption
- 📊 **Advanced Monitoring** - Comprehensive logging and error reporting
- 🏷️ **MailPace Integration** - Native support for tags, list-unsubscribe, and custom headers
- ⚡ **Zero-Downtime Deployment** - Docker-ready with health checks and graceful shutdown

## 🚀 Quick Start

### Prerequisites
- [Rust 1.70+](https://rustup.rs/)
- [MailPace Account](https://mailpace.com/signup) with API token

### Installation & Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/mailpace/vibe-smtp.git
   cd vibe-smtp
   ```

2. **Build and run**:
   ```bash
   cargo run
   ```

3. **Test the connection**:
   ```bash
   python3 test_smtp.py
   ```

### Docker Deployment

For comprehensive Docker setup with multi-port support, see **[DOCKER.md](DOCKER.md)**.

#### Quick Start (Multi-Port Configuration)
```bash
# Clone and build
git clone https://github.com/mailpace/vibe-smtp.git
cd vibe-smtp

# Run with all SMTP ports using the helper script
./docker-run.sh multi-port --token your_api_token

# Or use Docker Compose
docker-compose up -d
```

#### Port Configuration
The Docker setup supports industry-standard SMTP ports:

| Port | Protocol | Description | TLS Support |
|------|----------|-------------|-------------|
| **25** | SMTP | Standard mail transfer | STARTTLS optional |
| **587** | Submission | Message submission | STARTTLS optional |
| **2525** | Alternative | Development/testing | STARTTLS optional |
| **465** | SMTPS | SMTP over SSL | Implicit TLS (no STARTTLS) |

#### Docker Run Options

**Option 1: Helper Script (Recommended)**
```bash
# Multi-port mode with all SMTP ports
./docker-run.sh multi-port --token your_api_token

# Single-port mode (port 2525 only)
./docker-run.sh single-port --token your_api_token

# Development mode with debug logging
./docker-run.sh multi-port --dev --token your_api_token
```

**Option 2: Docker Compose**
```bash
# Copy environment file and edit with your API token
cp .env.example .env
# Edit .env and set MAILPACE_API_TOKEN=your_token

# Start all services
docker-compose up -d

# View logs
docker-compose logs -f vibe-gateway
```

**Option 3: Direct Docker Commands**
```bash
# Build the image
docker build -t vibe-gateway .

# Run multi-port mode
docker run -p 25:25 -p 587:587 -p 2525:2525 -p 465:465 \
  -e MAILPACE_API_TOKEN=your_token \
  vibe-gateway --docker-multi-port

# Run single-port mode  
docker run -p 2525:2525 \
  -e MAILPACE_API_TOKEN=your_token \
  vibe-gateway --listen 0.0.0.0:2525 --enable-tls
```

#### TLS Certificate Management

The Docker image includes test certificates for development. For production:

**Option 1: Mount your own certificates**
```bash
docker run -p 25:25 -p 587:587 -p 2525:2525 -p 465:465 \
  -v /path/to/your/cert.pem:/app/test_cert.pem:ro \
  -v /path/to/your/key.pem:/app/test_key.pem:ro \
  -e MAILPACE_API_TOKEN=your_token \
  vibe-gateway --docker-multi-port
```

**Option 2: Use Docker Compose with custom certificates**
```yaml
# docker-compose.override.yml
services:
  vibe-gateway:
    volumes:
      - ./your_cert.pem:/app/test_cert.pem:ro
      - ./your_key.pem:/app/test_key.pem:ro
```

#### Production Deployment Considerations

1. **Security**: Use proper TLS certificates in production
2. **Firewall**: Only expose necessary ports (typically 587 and 465)
3. **Monitoring**: Use the built-in health check endpoint
4. **Backup**: Ensure your MailPace API token is securely stored
5. **Scaling**: Run multiple containers behind a load balancer if needed

#### Health Checks

The Docker image includes health checks that verify SMTP connectivity:
```bash
# Check container health
docker inspect --format='{{.State.Health.Status}}' container_name

# Manual health check
docker exec container_name timeout 5 bash -c '</dev/tcp/localhost/2525'
```

## 🔧 Configuration

The server supports the following configuration options:

- `--listen` or `-l`: SMTP server listen address (default: `127.0.0.1:2525`)
- `--mailpace-endpoint`: MailPace API endpoint (default: `https://app.mailpace.com/api/v1/send`)
- `--default-mailpace-token`: Default MailPace API token (optional, can also be set via `MAILPACE_API_TOKEN` environment variable)
- `--enable-tls`: Enable TLS/STARTTLS support
- `--enable-attachments`: Enable attachment parsing and forwarding
- `--max-attachment-size`: Maximum size per attachment in bytes (default: 10MB)
- `--max-attachments`: Maximum number of attachments per email (default: 10)
- `--enable-html-compression`: Enable HTML compression for email bodies
- `--debug` or `-d`: Enable debug logging

## 🔐 Authentication

Vibe Gateway follows the MailPace SMTP authentication model for seamless integration:

### Primary Authentication Method
Users authenticate using their MailPace API token as both username and password:
- **Username**: Your MailPace API token  
- **Password**: Your MailPace API token (same as username)

### Fallback Option
Configure a default token for clients that can't provide authentication:
- Set via `--default-mailpace-token` flag
- Or use `MAILPACE_API_TOKEN` environment variable

### Finding Your API Token
API tokens are available in your [MailPace Dashboard](https://app.mailpace.com) under:
**Domain Settings → API Tokens**

> 💡 **Note**: Each domain has a unique API token for security and isolation.

## Quick Start

1. **Option 1: Users authenticate with their tokens** (recommended):
   ```bash
   cargo run
   ```

2. **Option 2: Use a default token**:
   ```bash
   export MAILPACE_API_TOKEN=your_default_token_here
   cargo run
   ```

3. Test with the included Python script:
   ```bash
   python3 test_smtp.py
   ```

## 📧 SMTP Client Configuration

Configure your email client or application with these settings:

### Standard Configuration

| Setting | Value | Notes |
|---------|-------|-------|
| **SMTP Server** | `localhost` | Or your server's IP address |
| **SMTP Port** | `25`, `587`, `2525`, or `465` | See port details below |
| **Encryption** | Varies by port | See TLS configuration below |
| **Authentication** | PLAIN or LOGIN | Standard SMTP AUTH methods |
| **Username** | Your MailPace API token | Get from MailPace Dashboard |
| **Password** | Your MailPace API token | Same as username |

### Port-Specific Configuration

| Port | Purpose | TLS Mode | Encryption | Typical Use |
|------|---------|----------|------------|-------------|
| **25** | Standard SMTP | STARTTLS optional | None/STARTTLS | Mail transfer agents |
| **587** | Message Submission | STARTTLS optional | None/STARTTLS | Email clients (recommended) |
| **2525** | Alternative SMTP | STARTTLS optional | None/STARTTLS | Development/testing |
| **465** | SMTP over SSL | Implicit TLS | SSL/TLS required | Legacy email clients |

> 💡 **Recommendation**: Use port **587** for email clients as it's the modern standard for message submission.

### Popular Email Clients

<details>
<summary><strong>Postfix Configuration</strong></summary>

```bash
# /etc/postfix/main.cf
# Use port 587 for message submission (recommended)
relayhost = [localhost]:587
smtp_sasl_auth_enable = yes
smtp_sasl_password_maps = hash:/etc/postfix/sasl_passwd
smtp_sasl_security_options = noanonymous
smtp_tls_security_level = may

# /etc/postfix/sasl_passwd
[localhost]:587 your_api_token:your_api_token

# Alternative: Use port 25 for standard SMTP
# relayhost = [localhost]:25
```
</details>

<details>
<summary><strong>Nodemailer (Node.js)</strong></summary>

```javascript
// Recommended: Port 587 with STARTTLS
const transporter = nodemailer.createTransporter({
  host: 'localhost',
  port: 587,
  secure: false, // true for 465, false for other ports
  auth: {
    user: 'your_api_token',
    pass: 'your_api_token'
  }
});

// Alternative: Port 465 with implicit TLS
const secureTransporter = nodemailer.createTransporter({
  host: 'localhost',
  port: 465,
  secure: true, // implicit TLS
  auth: {
    user: 'your_api_token',
    pass: 'your_api_token'
  }
});
```
</details>

## Usage

1. **Primary usage** (users provide their own API tokens):
   ```bash
   cargo run
   ```

2. **With default token fallback** (optional):
   ```bash
   export MAILPACE_API_TOKEN=your_default_token_here
   cargo run
   ```

3. **With custom settings**:
   ```bash
   cargo run -- --listen 0.0.0.0:587 --debug
   ```

## How It Works

When a user connects via SMTP:
1. They authenticate using their MailPace API token as both username and password
2. The server extracts this token from the SMTP AUTH command
3. The server uses this token to authenticate with the MailPace API
4. If no token is provided via SMTP AUTH, the server falls back to a default token (if configured)

## MailPace Features

The server supports the following MailPace-specific features:

### Tags
Add tags to emails by including the `X-MailPace-Tags` header:
```
X-MailPace-Tags: tag1, tag2, tag3
```

### List-Unsubscribe
Add unsubscribe links by including the `X-List-Unsubscribe` header:
```
X-List-Unsubscribe: <http://example.com/unsubscribe>, <mailto:unsubscribe@example.com>
```

### Attachments
Standard MIME attachments are automatically converted to MailPace format with base64 encoding.

## Attachment Support

The server supports email attachments when enabled with the `--enable-attachments` flag:

```bash
cargo run -- --enable-attachments
```

### Attachment Configuration

- `--enable-attachments`: Enable attachment parsing and forwarding
- `--max-attachment-size`: Maximum size per attachment in bytes (default: 10MB)
- `--max-attachments`: Maximum number of attachments per email (default: 10)

### Attachment Handling

When attachment support is enabled, the server:
- Parses MIME multipart messages
- Extracts attachments with their filenames and content types
- Converts attachments to base64 format for MailPace API
- Validates attachment sizes and counts against configured limits
- Logs attachment processing for debugging

### Example Usage

```bash
# Enable attachments with custom limits
cargo run -- --enable-attachments --max-attachment-size 5242880 --max-attachments 5

# Test with the attachment test script
python3 test_attachment.py
```

## HTML Compression

The server supports HTML compression for email bodies to reduce bandwidth and improve delivery performance:

```bash
cargo run -- --enable-html-compression
```

### HTML Compression Features

- **Automatic Detection**: Only compresses content that appears to be HTML
- **Safe Compression**: Preserves email client compatibility by keeping essential tags
- **Comment Removal**: Strips HTML comments to reduce size
- **Whitespace Optimization**: Removes unnecessary whitespace while preserving content
- **CSS/JS Minification**: Minifies inline CSS and JavaScript
- **Fallback Handling**: Uses original content if compression fails

### Compression Configuration

- `--enable-html-compression`: Enable HTML compression for email bodies

### How It Works

When HTML compression is enabled, the server:
1. Detects HTML content using heuristics (looks for common HTML tags)
2. Applies safe compression settings optimized for email clients
3. Removes comments and unnecessary whitespace
4. Minifies inline CSS and JavaScript
5. Logs compression statistics for monitoring
6. Falls back to original content if compression fails

### Example Usage

```bash
# Enable both attachments and HTML compression
cargo run -- --enable-attachments --enable-html-compression

# Or with TLS support
cargo run -- --enable-tls --enable-html-compression
```

### Attachment Test

The included `test_attachment.py` script demonstrates sending an email with an attachment:

```bash
python3 test_attachment.py
```

This script creates a test email with:
- Plain text body
- A sample text file attachment
- Proper MIME encoding

## Error Handling

The server provides detailed error messages back to SMTP clients:
- Authentication errors
- API token validation
- MailPace API errors
- Email parsing errors

## 🛠️ Development

### Prerequisites
- [Rust 1.70+](https://rustup.rs/) with Cargo
- [Git](https://git-scm.com/)
- [Docker](https://docker.com/) (optional)

### Local Development Setup

1. **Clone and setup**:
   ```bash
   git clone https://github.com/mailpace/vibe-smtp.git
   cd vibe-smtp
   ```

2. **Build and run**:
   ```bash
   cargo build
   cargo run
   ```

3. **Development with auto-reload**:
   ```bash
   cargo install cargo-watch
   cargo watch -x run
   ```

4. **Debug mode with detailed logging**:
   ```bash
   cargo run -- --debug
   ```

### 🏗️ Project Structure

```
src/
├── main.rs          # Application entry point
├── lib.rs           # Library exports
├── cli.rs           # Command-line interface
├── smtp.rs          # SMTP server implementation
├── mailpace.rs      # MailPace API integration
├── connection.rs    # Connection handling
├── compression.rs   # HTML compression
├── mime.rs          # MIME parsing
└── tls.rs           # TLS/encryption support
```

### 🧪 Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Security audit
cargo audit

# Documentation
cargo doc --open
```

### 📦 Key Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| [`tokio`](https://tokio.rs/) | Async runtime and networking | Latest |
| [`reqwest`](https://docs.rs/reqwest/) | HTTP client for MailPace API | Latest |
| [`mail-parser`](https://docs.rs/mail-parser/) | RFC-compliant email parsing | Latest |
| [`serde`](https://serde.rs/) | JSON serialization framework | Latest |
| [`base64`](https://docs.rs/base64/) | Attachment encoding | Latest |
| [`tracing`](https://docs.rs/tracing/) | Structured logging | Latest |
| [`clap`](https://docs.rs/clap/) | Command-line argument parsing | Latest |

### 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📋 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

<div align="center">
  
## About MailPace

**Vibe Gateway** is proudly developed by [MailPace](https://mailpace.com) - the developer-friendly email delivery service.

[![MailPace](https://img.shields.io/badge/Powered%20by-MailPace-blue?style=for-the-badge)](https://mailpace.com)

### Why Choose MailPace?

- 🚀 **99.9% Uptime SLA** - Enterprise-grade reliability
- 💰 **Transparent Pricing** - No hidden fees or overages  
- 🛡️ **Privacy-First** - GDPR compliant with EU data residency
- 📊 **Real-time Analytics** - Advanced delivery insights
- 🤝 **Developer-Friendly** - Comprehensive APIs and documentation

### Connect With Us

[🌐 Website](https://mailpace.com) • [📚 Documentation](https://docs.mailpace.com) • [💬 Discord](https://discord.gg/mailpace) • [🐦 Twitter](https://twitter.com/mailpace) • [📧 Support](mailto:support@mailpace.com)

---

**Built with ❤️ by the MailPace team**

</div>

## 🧪 Testing

[![Test Coverage](https://img.shields.io/badge/coverage-95%25-brightgreen.svg)](https://github.com/mailpace/vibe-smtp/actions)
[![Integration Tests](https://img.shields.io/badge/integration-passing-green.svg)](https://github.com/mailpace/vibe-smtp/actions)
[![Performance Tests](https://img.shields.io/badge/performance-optimized-blue.svg)](https://github.com/mailpace/vibe-smtp/actions)

This project includes a comprehensive test suite to ensure reliability and performance:

### 🎯 Test Suite Overview
- **Integration Tests**: End-to-end SMTP functionality with mock MailPace API
- **Unit Tests**: Individual component testing with 95%+ coverage
- **Performance Tests**: Load testing and throughput benchmarking  
- **Security Tests**: Authentication and input validation
- **CI/CD Pipeline**: Automated testing on every commit

### 🏃‍♂️ Running Tests

#### Quick Start
```bash
# Run all tests with coverage
./test.sh

# Run specific test suites  
./test.sh integration    # Integration tests only
./test.sh unit          # Unit tests only
./test.sh performance   # Performance tests only
./test.sh coverage      # Generate coverage report
```

#### Manual Test Commands
```bash
# All tests
cargo test

# Integration tests with mock MailPace API
cargo test --test integration_tests

# Unit tests for individual components  
cargo test --test mailpace_tests

# Performance and load tests
cargo test --test performance_tests --release
```

### 📊 Test Coverage
- **SMTP Protocol**: Command handling, authentication, data transfer
- **MailPace Integration**: API calls, error handling, payload formatting
- **Email Processing**: Attachments, HTML/text content, headers
- **Performance**: Concurrent connections, throughput, resource usage
- **Security**: Authentication, input validation, error handling

### 🔄 Continuous Integration
The project uses GitHub Actions for automated testing:
- ✅ **Code Quality**: Formatting, linting, and security audits
- ✅ **Cross-Platform**: Testing on Linux, macOS, and Windows
- ✅ **Performance**: Automated benchmarking and regression detection
- ✅ **Security**: Dependency vulnerability scanning
- ✅ **Docker**: Container build verification and security scanning

For detailed testing documentation, see [TESTING.md](TESTING.md).
