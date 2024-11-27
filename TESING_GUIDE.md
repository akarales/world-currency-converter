# Testing Guide - World Currency Converter API

## Overview

This guide covers all aspects of testing the World Currency Converter API, including unit tests, integration tests, and end-to-end testing.

## Quick Start

```bash
# Run all tests with default logging
cargo test

# Run all tests with debug logging
RUST_LOG=debug cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_simple_conversion_validation
```

## Test Categories

### 1. Unit Tests

Run unit tests for specific modules:

```bash
# Test handlers
cargo test --lib -- handlers::tests

# Test currency service
cargo test --lib -- currency_service::tests

# Test cache
cargo test --lib -- cache::tests
```

### 2. Integration Tests

#### API Tests
```bash
# Run all integration tests
cargo test --test api

# Run with detailed output
cargo test --test api -- --nocapture

# Run specific integration test
cargo test --test api test_basic_conversion_flow
```

#### Shell Script Tests
```bash
# Make script executable
chmod +x test_currency_api.sh

# Run the test script
./test_currency_api.sh
```

Example test script output:
```bash
=== Testing Simple Endpoint ===
Testing: Valid conversion (US to France)
POST /currency
Request: {"from": "United States", "to": "France", "amount": 100}
Response: {"from": "USD", "to": "EUR", "amount": 95.36}
✓ Test passed
```

### 3. Manual API Testing

#### Simple Endpoint Tests

1. Valid Conversion:
```bash
curl -X POST localhost:8080/currency \
  -d '{ "to": "France", "from": "USA", "amount": 33 }'
```

2. Invalid Amount:
```bash
curl -X POST localhost:8080/currency \
  -d '{ "to": "France", "from": "USA", "amount": 0 }'
```

3. Invalid Country:
```bash
curl -X POST localhost:8080/currency \
  -d '{ "to": "Narnia", "from": "USA", "amount": 100 }'
```

4. Case Sensitivity Test:
```bash
curl -X POST localhost:8080/currency \
  -d '{ "to": "FRANCE", "from": "usa", "amount": 50 }'
```

#### V1 Endpoint Tests

1. Valid Conversion:
```bash
curl -X POST localhost:8080/v1/currency \
  -H "Content-Type: application/json" \
  -d '{
    "from": "United States",
    "to": "France",
    "amount": 100,
    "preferred_currency": null
  }'
```

2. Invalid Request:
```bash
curl -X POST localhost:8080/v1/currency \
  -H "Content-Type: application/json" \
  -d '{
    "from": "",
    "to": "France",
    "amount": 100
  }'
```

#### Health Check:
```bash
curl http://localhost:8080/health
```

## Test Environment Setup

### 1. Environment Variables

Create a test environment file:
```bash
# .env.test
EXCHANGE_RATE_API_KEY=your_test_api_key
RUST_LOG=debug
```

Run tests with test environment:
```bash
ENV_FILE=.env.test cargo test
```

### 2. Test Coverage

Using cargo-tarpaulin for coverage:

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage analysis
cargo tarpaulin --ignore-tests
```

## Component Testing

### 1. Cache Tests
```bash
# Run cache-specific tests
cargo test --lib -- cache::tests

# Test cache expiration
RUST_LOG=debug cargo test test_cache_expiration
```

### 2. Rate Limiter Tests
```bash
# Run rate limiter tests
cargo test --lib -- rate_limit::tests

# Test rate limit exceeded
RUST_LOG=debug cargo test test_rate_limit_exceeded
```

### 3. Validation Tests
```bash
# Run model validation tests
cargo test --lib -- models::tests::test_conversion_request_validation
```

## Performance Testing

Using `hey` for load testing:

```bash
# Install hey
go install github.com/rakyll/hey@latest

# Run load test (100 requests, 10 concurrent)
hey -n 100 -c 10 -m POST \
  -H "Content-Type: application/json" \
  -d '{"from":"USA","to":"France","amount":100}' \
  http://localhost:8080/currency
```

## Test Debugging

### 1. Enable Debug Logs
```bash
# Set log level
export RUST_LOG=debug

# Run specific test with logs
RUST_LOG=debug cargo test test_simple_conversion_validation -- --nocapture
```

### 2. Test Timeouts
```bash
# Run tests with increased timeout
RUST_TEST_THREADS=1 cargo test --test api -- --test-threads=1
```

## Common Test Scenarios

### 1. Error Cases
- Missing API key
- Invalid country names
- Zero or negative amounts
- Rate limit exceeded
- API service unavailable

### 2. Edge Cases
- Same country conversion
- Case sensitivity
- Special characters in country names
- Very large amounts
- Multiple currencies per country

## Test Results Analysis

Example test output:
```bash
running 12 tests
test cache::tests::test_cache_expiration ... ok
test models::tests::test_conversion_request_validation ... ok
test rate_limit::tests::test_rate_limit_exceeded ... ok
test handlers::tests::test_simple_conversion_validation ... ok
...
test result: ok. 12 passed; 0 failed; 0 ignored
```

## Troubleshooting

### Common Issues

1. Test Timeouts
```bash
# Increase test timeout
export RUST_TEST_TIMEOUT=120
```

2. API Key Issues
```bash
# Verify test environment
cat .env.test
echo $EXCHANGE_RATE_API_KEY
```

3. Port Conflicts
```bash
# Check if port 8080 is in use
lsof -i :8080

# Kill process if needed
kill -9 <PID>
```

### Debug Commands

```bash
# Print test binary path
cargo test --test api --no-run -v

# Run single test with backtrace
RUST_BACKTRACE=1 cargo test test_name -- --exact

# Debug test compilation
cargo test -v
```

## Continuous Integration

Example GitHub Actions workflow:

```yaml
name: Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          profile: minimal
          toolchain: stable
      - name: Run tests
        run: cargo test
        env:
          EXCHANGE_RATE_API_KEY: ${{ secrets.EXCHANGE_RATE_API_KEY }}
          RUST_LOG: debug
```

## Best Practices

1. Always run tests before committing
2. Keep test files organized and well-documented
3. Use meaningful test names
4. Test both success and failure cases
5. Clean up test resources
6. Avoid test interdependencies
7. Use appropriate logging levels
8. Mock external services when appropriate

## Updating Tests

When adding new features:

1. Add unit tests for new functions
2. Update integration tests if API changes
3. Add new test cases to test_currency_api.sh
4. Update documentation tests
5. Add performance tests if relevant

---

For more information, see:
- [README.md](README.md) for general information
- [SETUP.md](SETUP.md) for environment setup
- [UPGRADE_PLAN.md](UPGRADE_PLAN.md) for future improvements