# World Currency Converter API (v0.0.2)

## A REST API service that converts currency amounts between countries using the RestCountries API to get country information and the ExchangeRate API for currency conversion rates

## Features

Current Implementation:

- Currency conversion between any two countries with proper case handling
- Automatic country name to currency code resolution with validation
- Real-time exchange rate fetching with robust error handling
- Concurrent request handling with optimized connection pooling
- Comprehensive error handling with detailed messages and context
- Environment-based configuration system with validation
- Robust logging system with debug capabilities
- In-memory caching with TTL and size limits
- Configurable rate limiting with monitoring
- Health check endpoint with service status
- Both simple and detailed API response formats (v1)
- Unit and integration tests with full coverage
- Documentation tests with examples
- Request validation with detailed feedback
- Dependency injection for better testing
- Service registry for centralized management
- Cache hit tracking and metrics
- Rate limit monitoring and feedback
- Trait-based client implementations

## Prerequisites

### System Requirements

- Rust (1.78.0 or newer)
- Ubuntu 24.04 LTS or compatible Linux distribution
- Git
- Cargo (comes with Rust)

### Required API Access

1. Exchange Rate API
    - Sign up at: [https://www.exchangerate-api.com/](https://www.exchangerate-api.com/)
    - Get your free API key
    - Free tier includes 1,500 requests per month
    - Documentation: [https://www.exchangerate-api.com/docs/overview](https://www.exchangerate-api.com/docs/overview)

1. REST Countries API
    - Base URL: [https://restcountries.com/v3.1](https://restcountries.com/v3.1)
    - No API key required
    - Documentation: [https://restcountries.com/](https://restcountries.com/)
    - Endpoints used:
      - Country search: `https://restcountries.com/v3.1/name/{name}`
      - Fields filtering: `?fields=name,currencies`

### Development Tools

- curl or any HTTP client for testing
- A text editor or IDE (recommended: VS Code with rust-analyzer)
- Environment for testing (recommended: separate .env.test file)
- Debug tools (optional: cargo-tarpaulin for coverage)

## Installation Steps

1. Install Rust (if not already installed):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

1. Verify Rust installation:

```bash
rustc --version  # Should show 1.78.0 or newer
cargo --version
```

1. Clone the repository:

```bash
git clone https://github.com/akarales/world-currency-converter.git
cd currency-converter
```

1. Set up your environment variables:

```bash
# Create .env file
echo "EXCHANGE_RATE_API_KEY=your_api_key_here" > .env
echo "RUST_LOG=info" >> .env

# Optional: Create test environment
echo "EXCHANGE_RATE_API_KEY=your_test_api_key" > .env.test
```

1. Build and run:

```bash
cargo build
cargo run
```

The server will start at [http://localhost:8080](http://localhost:8080)

## API Usage

The API provides two endpoints for currency conversion: a simple endpoint for basic conversions and a detailed v1 endpoint for comprehensive information.

### Simple Currency Conversion

**Endpoint:** `POST /currency`

Accepts both plain text and JSON requests. Provides a straightforward conversion response.

**Request Formats:**

Plain Text:

```bash
curl -X POST localhost:8080/currency \
  -d '{ "to": "France", "from": "USA", "amount": 33 }'
```

JSON:

```bash
curl -X POST localhost:8080/currency \
  -H "Content-Type: application/json" \
  -d '{
    "from": "USA",
    "to": "France",
    "amount": 100,
    "preferred_currency": null
  }'
```

**Success Response:**

```json
{
  "from": "USD",
  "to": "EUR",
  "amount": 95.36
}
```

**Error Responses:**

Invalid Country:

```json
{
  "error": "Country not found: Narnia",
  "code": "COUNTRY_NOT_FOUND",
  "request_id": "0bef9088-f272-4b88-b9c6-69cabaf0f96a",
  "timestamp": "2024-11-26T22:51:42.178152195Z"
}
```

Service Error:

```json
{
  "error": "Service temporarily unavailable",
  "code": "SERVICE_ERROR",
  "request_id": "7ef9088-f272-4b88-b9c6-69cabaf0f96a",
  "timestamp": "2024-11-26T22:51:42.178152195Z",
  "details": "External API error: Rate limit exceeded"
}
```

### Detailed Currency Conversion (v1)

**Endpoint:** `POST /v1/currency`

Requires JSON content type. Provides detailed conversion information including rates, timestamps, and currency details.

**Request Format:**

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

**Success Response:**

```json
{
  "request_id": "365a1026-c502-4aab-a0c6-085af426ce55",
  "timestamp": "2024-11-26T22:51:42.002281834Z",
  "data": {
    "from": {
      "country": "United States",
      "currency_code": "USD",
      "currency_name": "United States dollar",
      "currency_symbol": "$",
      "amount": 100.0,
      "is_primary": true
    },
    "to": {
      "country": "France",
      "currency_code": "EUR",
      "currency_name": "Euro",
      "currency_symbol": "€",
      "amount": 95.36,
      "is_primary": true
    },
    "exchange_rate": 0.95361081,
    "last_updated": "2024-11-26T22:51:42.002214129Z",
    "available_currencies": null
  },
  "meta": {
    "source": "exchangerate-api.com",
    "response_time_ms": 143,
    "multiple_currencies_available": false,
    "cache_hit": false,
    "rate_limit_remaining": 1499
  }
}
```

**Error Response:**

```json
{
  "error": "Country not found: Narnia",
  "code": "COUNTRY_NOT_FOUND",
  "request_id": "0bef9088-f272-4b88-b9c6-69cabaf0f96a",
  "timestamp": "2024-11-26T22:51:42.178152195Z",
  "available_currencies": null,
  "details": "Country could not be found in the database"
}
```

### API Features Comparison

| Feature | Simple API | V1 API |
|---------|------------|---------|
| Endpoint | `/currency` | `/v1/currency` |
| Content Types | Plain Text, JSON | JSON only |
| Response Format | Minimal | Detailed |
| Request Tracking | Yes | Yes (with request_id) |
| Timestamps | Yes | Yes |
| Currency Details | Codes only | Full details |
| Exchange Rate Info | No | Yes |
| Performance Metrics | No | Yes |
| Error Details | Basic | Comprehensive |
| Cache Info | No | Yes |
| Rate Limit Info | No | Yes |

### Common Features

- Case-insensitive country names
- Automatic currency code resolution
- Real-time exchange rates
- Input validation
- Rate limiting protection
- Error handling with context
- Request tracking
- Performance monitoring

### Health Check

**Endpoint:** `GET /health`

**Response:**

```text
OK
```

## Project Structure

```text
currency-converter/
├── Cargo.toml          # Project dependencies and metadata
├── .env                # Environment variables (API keys)
├── .env.test           # Test environment configuration
├── src/
│   ├── cache.rs        # Caching implementation
│   ├── clients/        # API client implementations
│   │   └── mod.rs      # Client traits and HTTP client
│   ├── config.rs       # Configuration management
│   ├── currency_service.rs  # Core service logic
│   ├── errors.rs       # Error handling
│   ├── handlers.rs     # Simple API handlers
│   ├── handlers_v1.rs  # V1 API handlers
│   ├── lib.rs          # Library interface
│   ├── main.rs         # Application entry point
│   ├── models.rs       # Data structures
│   ├── monitor.rs      # Monitoring implementation
│   ├── rate_limit.rs   # Rate limiting
│   └── registry.rs     # Service registry
├── test_currency_api.sh # Integration test script
├── tests/
│   └── api.rs          # Integration tests
├── DEVELOPER_UPDATE.md # Developer documentation
├── TESTING_GUIDE.md   # Testing documentation
└── UPGRADE_PLAN.md    # Future plans
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --test api      # Run only API tests
cargo test --lib           # Run only library tests

# Run tests with logging
RUST_LOG=debug cargo test

# Run API integration tests
./test_currency_api.sh

# Run specific test
cargo test test_convert_currency_missing_api_key

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --ignore-tests
```

### Test Coverage

The test suite includes:

- Unit tests for core functions
- Integration tests for API endpoints
- Documentation tests with examples
- Error handling tests
- Rate limiting tests
- Cache behavior tests
- Validation tests
- Mock client tests
- Service registry tests
- Configuration validation tests
- Performance tests
- Load tests (using hey)

See `TESTING_GUIDE.md` for comprehensive testing documentation.

## Implemented Features

- [x] Basic currency conversion
- [x] Detailed v1 API response format
- [x] Request ID tracking
- [x] Timestamp in responses
- [x] Enhanced error messages with context
- [x] Rate information in responses
- [x] Source country and currency details
- [x] In-memory caching with TTL
- [x] Configurable rate limiting
- [x] Health check endpoint
- [x] Logging system with levels
- [x] Configuration management with validation
- [x] Test infrastructure with mocks
- [x] Dependency injection support
- [x] Service registry pattern
- [x] Cache metrics tracking
- [x] Rate limit monitoring
- [x] Request validation
- [x] Performance monitoring

## Future Enhancements

- [ ] Prometheus metrics integration
- [ ] Multi-currency support for countries
- [ ] Batch conversion endpoint
- [ ] Historical rate lookup
- [ ] Rate alerts
- [ ] API documentation using OpenAPI/Swagger
- [ ] WebSocket support for real-time rates
- [ ] Database integration for audit logs
- [ ] Rate limiting with Redis
- [ ] Distributed caching support
- [ ] GraphQL API support
- [ ] Docker containerization
- [ ] Kubernetes deployment manifests
- [ ] CI/CD pipeline
- [ ] API versioning strategy

See `UPGRADE_PLAN.md` for detailed enhancement roadmap.

## Performance Considerations

Current Optimizations:

- Connection pooling for HTTP clients with configurable settings
- In-memory caching with TTL and size limits
- Rate limiter with configurable windows
- Concurrent request handling with thread pool
- Efficient error handling with proper context
- Debug logging with controllable levels
- Request tracking with minimal overhead
- Async/await for non-blocking operations
- Service registry for resource management
- Optimized JSON serialization
- Memory-efficient data structures
- Connection reuse

## Development Practices

1. Code Style:

    ```bash
    # Format code
    cargo fmt

    # Check lints
    cargo clippy

    # Check documentation
    cargo doc

    # Run security audit
    cargo audit
    ```

2. Git Workflow:
    - Main branch: Stable releases
    - Feature branches: New functionality
    - Test branches: Testing infrastructure
    - Version tags: Release points

3. Development Process:
    - Write tests first
    - Document changes
    - Update UPGRADE_PLAN.md
    - Create pull requests
    - Code review
    - Update documentation

## Troubleshooting

Common Issues:

1. API Key Issues

    ```bash
    # Check environment
    echo $EXCHANGE_RATE_API_KEY

    # Verify .env file
    cat .env

    # Check logs
    RUST_LOG=debug cargo run

    # Restart server
    cargo run
    ```

2. Rate Limiting
    - Check response headers for limit information
    - Review rate_limit.rs logs
    - Monitor rate_limit metrics
    - Adjust rate limit configuration
    - Implement caching if hitting limits

3. Connection Issues
    - Verify network connectivity
    - Check API service status
    - Review debug logs for connection errors
    - Verify timeouts configuration
    - Check DNS resolution

4. Performance Issues
    - Monitor response times
    - Check cache hit rates
    - Review connection pool settings
    - Analyze request patterns
    - Check resource usage

See `DEVELOPER_UPDATE.md` for more troubleshooting details.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

1. Check the troubleshooting guide
2. Review debug logs
3. Open an issue in the repository
4. Contact the maintainers
5. Check `DEVELOPER_UPDATE.md`
6. Review `TESTING_GUIDE.md`

## Acknowledgments

- REST Countries API: [https://restcountries.com/](https://restcountries.com/)
- Exchange Rate API: [https://www.exchangerate-api.com/](https://www.exchangerate-api.com/)
- Rust Community: [https://www.rust-lang.org/community](https://www.rust-lang.org/community)
- Contributors and maintainers

---

**Note:** This documentation is actively maintained and updated with new features and improvements. See version history for changelog.
