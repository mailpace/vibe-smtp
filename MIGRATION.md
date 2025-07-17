# Migration to cargo-nextest

This document describes the migration from the homegrown test runner to cargo-nextest.

## What Changed

### 1. Test Runner Replacement
- **Before**: Custom bash script with `cargo test`
- **After**: cargo-nextest with advanced configuration and profiles

### 2. Key Improvements
- **Faster execution**: Parallel test execution with intelligent scheduling
- **Better isolation**: Tests run in separate processes
- **Rich output**: Detailed timing, retry information, and multiple formats
- **Configurable profiles**: Different test configurations for various scenarios
- **Reliable results**: Built-in retry mechanisms and flaky test detection

### 3. Configuration Files Added
- `.config/nextest.toml` - Main nextest configuration with profiles
- `justfile` - Alternative command runner (optional)
- Updated `.github/workflows/ci.yml` - CI pipeline using nextest

### 4. New Test Profiles
- `default`: Standard test execution
- `ci`: Optimized for continuous integration
- `integration`: Specific settings for integration tests
- `performance`: Sequential execution for performance tests
- `unit`: Fast execution for unit tests only

## Usage

### Installation
```bash
# Install cargo-nextest
cargo install cargo-nextest --locked

# Or use the test script
./test.sh install
```

### Running Tests
```bash
# Using the updated test script
./test.sh                    # Run all tests
./test.sh unit               # Run unit tests
./test.sh integration        # Run integration tests
./test.sh performance        # Run performance tests
./test.sh pattern "smtp"     # Run tests matching pattern
./test.sh watch              # Run in watch mode
./test.sh timing             # Show timing information

# Using cargo-nextest directly
cargo nextest run                        # Run all tests
cargo nextest run --profile unit         # Run unit tests
cargo nextest run --profile integration  # Run integration tests
cargo nextest run -E 'test(smtp)'       # Pattern matching
```

### New Features
- **Watch mode**: Automatically re-run tests on file changes
- **Pattern matching**: Run specific test patterns
- **Timing information**: Detailed execution timing
- **Multiple output formats**: JSON, JUnit XML, etc.
- **Test result archiving**: Persistent test results

## Benefits

1. **Performance**: Tests run faster due to intelligent parallelization
2. **Reliability**: Built-in retry mechanisms for flaky tests
3. **Observability**: Rich output with timing and status information
4. **Flexibility**: Multiple profiles for different testing scenarios
5. **CI/CD Ready**: Better integration with continuous integration systems

## Backward Compatibility

The test script maintains the same interface, so existing workflows continue to work:
- `./test.sh` still runs all tests
- `./test.sh integration` still runs integration tests
- Environment variables like `RUST_LOG` still work

## Migration Checklist

- [x] Install cargo-nextest
- [x] Create nextest configuration file
- [x] Update test runner script
- [x] Update CI/CD pipeline
- [x] Update documentation
- [x] Test all workflows

## Next Steps

1. **Run the installation**: `./test.sh install`
2. **Test the migration**: `./test.sh`
3. **Verify CI pipeline**: Check that CI tests pass
4. **Team onboarding**: Share this documentation with the team
