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
- `--mailpace-token`: MailPace API token (can also be set via `MAILPACE_API_TOKEN` environment variable)
- `--debug` or `-d`: Enable debug logging

## Quick Start

1. Set your MailPace API token:
   ```bash
   export MAILPACE_API_TOKEN=your_api_token_here
   ```

2. Run the server:
   ```bash
   cargo run
   ```

3. Test with the included Python script:
   ```bash
   python3 test_smtp.py
   ```

## Usage

1. Set your MailPace API token:
   ```bash
   export MAILPACE_API_TOKEN=your_api_token_here
   ```

2. Run the server:
   ```bash
   cargo run
   ```

3. Or with custom settings:
   ```bash
   cargo run -- --listen 0.0.0.0:587 --debug
   ```

## SMTP Client Configuration

Configure your email client or application with these settings:

- **SMTP Server**: `localhost` (or your server's IP)
- **SMTP Port**: `2525` (or your configured port)
- **Encryption**: None (STARTTLS supported but not enforced)
- **Authentication**: PLAIN or LOGIN (any credentials accepted)
- **Username**: Any value (authentication is pass-through)
- **Password**: Any value (authentication is pass-through)

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
