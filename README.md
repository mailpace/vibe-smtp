# Vibe Gateway - SMTP to MailPace Bridge

A Rust SMTP server that accepts emails and forwards them to the MailPace API.

## Features

- Full SMTP server implementation with authentication support
- Converts SMTP emails to MailPace API format
- Handles attachments, HTML/text bodies, and custom headers
- Supports MailPace-specific features like tags and list-unsubscribe
- Detailed error reporting back to SMTP clients
- Configurable listening address and API endpoints

## Configuration

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

## Authentication

The server follows the MailPace SMTP authentication model as described in the SMTP-DOCS.md:

- **Primary method**: Users provide their MailPace API token as both username and password when connecting via SMTP
- **Fallback**: If no token is provided via SMTP AUTH, the server can use a default token from the environment or command line
- **Token format**: Both username and password should be set to the same MailPace API token

According to MailPace documentation: "API tokens can be found under the 'API Tokens' menu of each Domain, there is one unique API token for every domain"

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

## SMTP Client Configuration

Configure your email client or application with these settings:

- **SMTP Server**: `localhost` (or your server's IP)
- **SMTP Port**: `2525` (or your configured port)
- **Encryption**: None (STARTTLS supported but not enforced)
- **Authentication**: PLAIN or LOGIN
- **Username**: Your MailPace API token
- **Password**: Your MailPace API token (same as username)

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

### HTML Compression Testing

Test HTML compression with the included script:

```bash
# Start server with compression enabled
cargo run -- --enable-html-compression --debug

# In another terminal, run the test script
./test_html_compression.py
```

The test script sends HTML emails with:
- Comments and extra whitespace
- Inline CSS and JavaScript
- Complex HTML structures
- Compression statistics in server logs

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

## Development

Build and run:
```bash
cargo build
cargo run
```

Run with debug logging:
```bash
cargo run -- --debug
```

## Dependencies

- `tokio`: Async runtime
- `reqwest`: HTTP client for MailPace API
- `mail-parser`: Email parsing
- `serde`: JSON serialization
- `base64`: Attachment encoding
- `tracing`: Logging
- `clap`: Command line argument parsing

## License

This project is licensed under the MIT License.

## Testing

This project includes a comprehensive test suite to ensure reliability and performance:

### Test Suite Overview
- **Integration Tests**: End-to-end testing of SMTP functionality with mock MailPace API
- **Unit Tests**: Testing individual components in isolation
- **Performance Tests**: Load testing and throughput measurement
- **CI/CD Pipeline**: Automated testing on every commit

### Running Tests

#### Quick Start
```bash
# Run all tests
./test.sh

# Run specific test suites
./test.sh integration    # Integration tests only
./test.sh unit          # Unit tests only
./test.sh performance   # Performance tests only
./test.sh coverage      # Tests with coverage report
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

### Test Coverage
- **SMTP Protocol**: Command handling, authentication, data transfer
- **MailPace Integration**: API calls, error handling, payload formatting
- **Email Processing**: Attachments, HTML/text content, headers
- **HTML Compression**: Compression functionality, performance impact, edge cases
- **Performance**: Concurrent connections, throughput, resource usage
- **Security**: Authentication, input validation, error handling

### Continuous Integration
The project uses GitHub Actions for automated testing:
- ✅ Code formatting and linting
- ✅ Unit and integration tests
- ✅ Performance benchmarks
- ✅ Security audits
- ✅ Docker build verification

For detailed testing documentation, see [TESTING.md](TESTING.md).
