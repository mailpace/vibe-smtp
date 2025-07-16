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
