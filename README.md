# World Currency Converter API (v0.0.1)

## A REST API service that converts currency amounts between countries using the RestCountries API to get country information and the ExchangeRate API for currency conversion rates

## Features

Current Implementation:

- Currency conversion between any two countries with proper case handling
- Automatic country name to currency code resolution
- Real-time exchange rate fetching with error handling
- Concurrent request handling with connection pooling
- Comprehensive error handling with detailed messages
- Environment-based configuration system
- Robust logging system with debug capabilities
- In-memory caching for better performance
- Rate limiting to comply with API restrictions
- Health check endpoint for monitoring
- Both simple and detailed API response formats (v1)
- Unit and integration tests with full coverage
- Documentation tests with examples

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
git clone <repository-url>
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
  "from": "INVALID",
  "to": "INVALID",
  "amount": 0.0
}
```

Service Error:

```json
{
  "from": "ERROR",
  "to": "ERROR",
  "amount": 0.0
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
    "multiple_currencies_available": false
  }
}
```

**Error Response:**

```json
{
  "error": "Country not found: Narnia",
  "request_id": "0bef9088-f272-4b88-b9c6-69cabaf0f96a",
  "timestamp": "2024-11-26T22:51:42.178152195Z",
  "available_currencies": null
}
```

### API Features Comparison

| Feature | Simple API | V1 API |
|---------|------------|---------|
| Endpoint | `/currency` | `/v1/currency` |
| Content Types | Plain Text, JSON | JSON only |
| Response Format | Minimal | Detailed |
| Request Tracking | No | Yes (request_id) |
| Timestamps | No | Yes |
| Currency Details | Codes only | Full details |
| Exchange Rate Info | No | Yes |
| Performance Metrics | No | Yes |
| Error Details | Basic | Comprehensive |

### Common Features

- Case-insensitive country names
- Automatic currency code resolution
- Real-time exchange rates
- Input validation
- Rate limiting protection
- Error handling

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
│   ├── main.rs         # Application entry point
│   ├── lib.rs          # Library interface and utilities
│   ├── models.rs       # Data structures
│   ├── handlers.rs     # Simple API handlers
│   ├── handlers_v1.rs  # V1 API handlers
│   ├── cache.rs        # Caching implementation
│   ├── config.rs       # Configuration management
│   ├── currency_service.rs  # Currency conversion logic
│   ├── monitor.rs      # Monitoring implementation
│   └── rate_limit.rs   # Rate limiting logic
├── tests/
│   └── api.rs          # Integration tests
└── test_currency_api.sh  # API test script
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
```

### Test Coverage

The test suite includes:

- Unit tests for core functions
- Integration tests for API endpoints
- Documentation tests with examples
- Error handling tests
- Rate limiting tests
- Cache behavior tests

## Implemented Features

- [x] Basic currency conversion
- [x] Detailed v1 API response format
- [x] Request ID tracking
- [x] Timestamp in responses
- [x] Enhanced error messages
- [x] Rate information in responses
- [x] Source country and currency details
- [x] In-memory caching
- [x] Rate limiting
- [x] Health check endpoint
- [x] Logging system
- [x] Configuration management
- [x] Test infrastructure

## Future Enhancements

- [ ] Prometheus metrics integration
- [ ] Multi-currency support for countries
- [ ] Batch conversion endpoint
- [ ] Historical rate lookup
- [ ] Rate alerts
- [ ] API documentation using OpenAPI/Swagger
- [ ] WebSocket support for real-time rates
- [ ] Database integration for audit logs

## Performance Considerations

Current Optimizations:

- Connection pooling for HTTP clients
- In-memory caching for country information
- Rate limiter to prevent API exhaustion
- Concurrent request handling
- Efficient error handling
- Debug logging for troubleshooting

## Development Practices

1. Code Style:

```bash
# Format code
cargo fmt

# Check lints
cargo clippy
```

1. Git Workflow:
    - Main branch: Stable releases
    - Feature branches: New functionality
    - Test branches: Testing infrastructure

## Troubleshooting

Common Issues:

1. API Key Issues

```bash
# Check environment
echo $EXCHANGE_RATE_API_KEY

# Verify .env file
cat .env

# Restart server
cargo run
```

1. Rate Limiting
    - Check response headers for limit information
    - Use debug logging to monitor rate limit status
    - Implement caching if hitting limits frequently

1. Connection Issues
    - Verify network connectivity
    - Check API service status
    - Review debug logs for connection errors

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

1. Check the troubleshooting guide
1. Review debug logs
1. Open an issue in the repository
1. Contact the maintainers

## Acknowledgments

- REST Countries API: [https://restcountries.com/](https://restcountries.com/)
- Exchange Rate API: [https://www.exchangerate-api.com/](https://www.exchangerate-api.com/)
- Rust Community: [https://www.rust-lang.org/community](https://www.rust-lang.org/community)

---

**Note:** This documentation is actively maintained and updated with new features and improvements.
