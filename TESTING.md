# Test Suite Documentation

This document describes the comprehensive testing suite for the Vibe Gateway SMTP server using cargo-nextest.

## Test Infrastructure

### cargo-nextest
This project uses [cargo-nextest](https://nexte.st/) for test execution, which provides:
- **Faster execution**: Parallel test execution with intelligent scheduling
- **Better isolation**: Tests run in separate processes with proper cleanup
- **Rich output**: Detailed timing, retry information, and multiple output formats
- **Configurable profiles**: Different test configurations for various scenarios
- **Reliable results**: Built-in retry mechanisms and flaky test detection

### Test Profiles
The project includes several nextest profiles configured in `.config/nextest.toml`:
- **default**: Standard test execution with retries and reasonable timeouts
- **ci**: Optimized for continuous integration with increased timeouts
- **integration**: Specific settings for integration tests with reduced parallelism
- **performance**: Sequential execution for performance tests
- **unit**: Fast execution for unit tests only

## Test Structure

### Integration Tests (`tests/integration_tests.rs`)
- **Purpose**: End-to-end testing of the entire SMTP server functionality
- **Coverage**: Tests the complete workflow from SMTP client to MailPace API
- **Test Cases**:
  - Basic email sending
  - HTML email content
  - Email attachments
  - MailPace-specific headers
  - Authentication scenarios
  - Multiple recipients
  - Large email content
  - SMTP command handling
  - Default token usage

### Unit Tests (`tests/mailpace_tests.rs`)
- **Purpose**: Testing individual components in isolation
- **Coverage**: MailPace client functionality, payload serialization
- **Test Cases**:
  - MailPace API success responses
  - MailPace API error handling
  - Attachment handling
  - Payload serialization
  - Network error scenarios

### Performance Tests (`tests/performance_tests.rs`)
- **Purpose**: Performance and load testing
- **Coverage**: Throughput, concurrency, resource usage
- **Test Cases**:
  - Concurrent email sending
  - Throughput measurement
  - Large email performance
  - Connection handling under load
  - Memory usage with attachments
  - Stress testing

## Running Tests

### Prerequisites
```bash
# Install Rust and Cargo
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cargo-nextest
cargo install cargo-nextest --locked

# Build the project
cargo build
```

### Using the Test Runner Script
```bash
# Run all tests with the test runner
./test.sh

# Run specific test suites
./test.sh unit           # Unit tests only
./test.sh integration    # Integration tests only
./test.sh performance    # Performance tests only

# Run tests with pattern matching
./test.sh pattern "smtp"  # Run tests matching "smtp"

# Run tests in watch mode
./test.sh watch

# Generate coverage report
./test.sh coverage

# Show detailed timing information
./test.sh timing
```

### Running Tests with cargo-nextest Directly
```bash
# Run all tests with nextest
cargo nextest run

# Run tests with specific profile
cargo nextest run --profile ci
cargo nextest run --profile integration
cargo nextest run --profile performance

# Run tests with output
cargo nextest run --success-output immediate

# Run tests with debug logging
RUST_LOG=debug cargo nextest run
```

### Running Specific Test Suites
```bash
# Integration tests only
cargo nextest run --profile integration

# Unit tests only  
cargo nextest run --profile unit

# Performance tests only
cargo nextest run --profile performance --release

# Tests matching a pattern
cargo nextest run -E 'test(smtp)'
cargo nextest run -E 'test(mailpace)'
```

### Advanced nextest Features
```bash
# Run tests with retries
cargo nextest run --retries 3

# Run with timing information
cargo nextest run --final-status-level slow

# Generate JUnit XML output
cargo nextest run --profile ci --output-format junit > test-results.xml

# Run tests with specific timeout
cargo nextest run --profile default --slow-timeout 120s
```

## Test Environment Setup

### Mock Server
- Uses `wiremock` to simulate the MailPace API
- Automatically starts and stops with each test
- Configurable responses for different scenarios

### Test Server
- Starts a real instance of the vibe-gateway server
- Uses random ports to avoid conflicts
- Automatically cleaned up after tests

### Test Data
- Uses realistic email content and headers
- Tests various payload sizes and formats
- Includes edge cases and error conditions

## Continuous Integration

### GitHub Actions Workflow
Located at `.github/workflows/ci.yml`, the CI pipeline includes:

1. **Code Quality Checks**:
   - Rust formatting (`cargo fmt`)
   - Clippy linting (`cargo clippy`)

2. **Testing**:
   - Unit tests (`cargo test --lib`)
   - Integration tests (`cargo test --test integration_tests`)
   - Performance tests (`cargo test --test performance_tests`)

3. **Security**:
   - Dependency audit (`cargo audit`)

4. **Build Verification**:
   - Release build (`cargo build --release`)
   - Docker image build

### Test Coverage
- Coverage reporting with `cargo tarpaulin`
- Uploaded to Codecov for tracking
- Fails CI if coverage drops significantly

## Test Configuration

### Environment Variables
- `RUST_LOG`: Set logging level for tests
- `MAILPACE_API_TOKEN`: Default API token for testing

### Test Timeouts
- Individual test timeout: 30 seconds
- Server startup timeout: 3 seconds
- Network operation timeout: 10 seconds

## Writing New Tests

### Integration Test Template
```rust
#[tokio::test]
async fn test_new_feature() -> Result<()> {
    let server = TestServer::new().await?;
    server.mock_server.setup_success_response().await;
    
    let transport = create_smtp_transport(
        server.smtp_address(),
        Some(Credentials::new("test-token".to_string(), "test-token".to_string()))
    );
    
    // Test implementation
    
    Ok(())
}
```

### Unit Test Template
```rust
#[tokio::test]
async fn test_component_function() {
    // Test setup
    
    // Test execution
    
    // Assertions
    assert!(result.is_ok());
}
```

## Test Data Management

### Mock Responses
- Realistic MailPace API responses
- Error scenarios with proper status codes
- Edge cases and malformed data

### Test Fixtures
- Sample email content
- Various attachment types
- Different header configurations

## Debugging Tests

### Logging
```bash
# Enable debug logging
RUST_LOG=debug cargo test test_name -- --nocapture

# Enable trace logging for detailed output
RUST_LOG=trace cargo test test_name -- --nocapture
```

### Test Isolation
- Each test runs in isolation
- Clean state between tests
- No shared global state

### Common Issues
1. **Port conflicts**: Tests use random ports
2. **Timing issues**: Tests include proper wait mechanisms
3. **Resource cleanup**: Automatic cleanup via Drop trait

## Performance Benchmarks

### Throughput Targets
- Minimum 5 emails/second sequential
- Support for 20+ concurrent connections
- Handle 1MB emails within 10 seconds

### Memory Usage
- Efficient handling of large attachments
- Proper cleanup of resources
- No memory leaks under load

## Contributing

### Adding Tests
1. Identify the feature or bug to test
2. Choose appropriate test type (integration/unit/performance)
3. Follow existing patterns and conventions
4. Include both positive and negative test cases
5. Add appropriate documentation

### Test Review Checklist
- [ ] Tests are deterministic and repeatable
- [ ] Proper error handling and cleanup
- [ ] Realistic test data and scenarios
- [ ] Performance implications considered
- [ ] Documentation updated
