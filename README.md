# World Currency Converter API

## A REST API service that converts currency amounts between countries using the RestCountries API to get country information and the ExchangeRate API for currency conversion rates

## Features

Current Implementation:

- Currency conversion between any two countries
- Automatic country name to currency code resolution
- Real-time exchange rate fetching
- Concurrent request handling
- Error handling with detailed messages
- Environment-based configuration
- Logging system

## Prerequisites

### System Requirements

- Rust (1.78.0 or newer)
- Ubuntu 24.04 LTS or compatible Linux distribution
- Git

### Required API Access

1. Exchange Rate API
   - Sign up at: [https://www.exchangerate-api.com/](https://www.exchangerate-api.com/)
   - Get your free API key
   - Free tier includes 1,500 requests per month
   - Documentation: [https://www.exchangerate-api.com/docs/overview](https://www.exchangerate-api.com/docs/overview)

2. REST Countries API
   - Base URL: [https://restcountries.com/v3.1](https://restcountries.com/v3.1)
   - No API key required
   - Documentation: [https://restcountries.com/](https://restcountries.com/)
   - Endpoints used:
     - Country search: `https://restcountries.com/v3.1/name/{name}`
     - Fields filtering: `?fields=name,currencies`

### Development Tools

- Cargo (comes with Rust installation)
- curl or any HTTP client for testing
- A text editor or IDE (recommended: VS Code with rust-analyzer)

## Installation Steps

1.Install Rust (if not already installed):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

2.Verify Rust installation:

```bash
rustc --version  # Should show 1.78.0 or newer
cargo --version
```

3.Clone the repository:

```bash
git clone <repository-url>
cd currency-converter
```

4.Set up your environment variables:

```bash
# Create .env file
echo "EXCHANGE_RATE_API_KEY=your_api_key_here" > .env
echo "RUST_LOG=info" >> .env
```

5.Build and run:

```bash
cargo build
cargo run
```

The server will start at [http://localhost:8080](http://localhost:8080)

## API Usage

### Convert Currency

**Endpoint:** `POST /currency`

**Request Format:**

```bash
curl -X POST http://localhost:8080/currency \
  -H "Content-Type: application/json" \
  -d '{"from": "USA", "to": "France", "amount": 100}'
```

**Success Response:**

```json
{
  "from": "USD",
  "to": "EUR",
  "amount": 95.96
}
```

**Error Response:**

```json
{
  "error": "Invalid source country 'INVALID': Country not found"
}
```

## Project Structure

```bash
currency-converter/
├── .env                 # Environment variables (API keys)
├── .gitignore          # Git ignore rules
├── Cargo.toml          # Project dependencies
└── src/
    ├── main.rs         # Application entry point
    ├── models.rs       # Data structures
    └── handlers.rs     # Request handlers
```

## Implementation Plan

### Phase 1: Enhanced Response Format (Current Branch: feature/enhanced-response)

- [ ] Add request ID tracking
- [ ] Include timestamp in responses
- [ ] Enhance error messages
- [ ] Add rate information to responses
- [ ] Include source country and currency details

### Phase 2: Caching (Future Branch: feature/caching)

- [ ] Implement in-memory cache for country information
  - Cache duration: 24 hours
  - Automatic invalidation
- [ ] Add exchange rate caching
  - Cache duration: 1 hour
  - Rate expiration handling
- [ ] Add cache headers to responses

### Phase 3: Rate Limiting (Future Branch: feature/rate-limiting)

- [ ] Implement token bucket algorithm
- [ ] Add per-IP rate limiting
- [ ] Configure rate limit windows
- [ ] Add rate limit headers
- [ ] Implement graceful rate limit handling

### Phase 4: Monitoring & Metrics (Future Branch: feature/monitoring)

- [ ] Add health check endpoint
- [ ] Implement Prometheus metrics
  - Request counts
  - Response times
  - Error rates
- [ ] Add logging middleware
- [ ] Create operational dashboards

### Phase 5: Multi-Currency Support (Future Branch: feature/multi-currency)

- [ ] Handle countries with multiple currencies
- [ ] Allow specific currency selection
- [ ] Implement currency fallback logic
- [ ] Add currency validation

## API Rate Limits and Considerations

### REST Countries API

- No API key required
- Rate limiting: Unspecified
- Endpoint used: `/v3.1/name/{name}?fields=name,currencies`
- Response format includes:
  - Country name (common and official)
  - Currency information (code, name, symbol)
- Error handling:
  - 404: Country not found
  - 500: Server error

### Exchange Rate API

- Requires API key
- Free tier limitations:
  - 1,500 requests per month
  - Rate limiting applies
- HTTP Response codes:
  - 200: Success
  - 401: Invalid API key
  - 429: Rate limit exceeded
- After hitting rate limit:
  - 20-minute cooldown period
  - Returns HTTP 429 during cooldown

## Performance Considerations

Current Bottlenecks:

1. External API Dependencies
   - REST Countries API calls
   - Exchange Rate API calls
2. No Caching
   - Repeated country lookups
   - Frequent exchange rate fetches
3. Sequential Processing
   - Country lookups happen one after another

Planned Optimizations:

1. Implement caching to reduce API calls
2. Parallel processing of country lookups
3. Connection pooling for API requests
4. Response compression
5. Request batching capabilities

## Development Practices

1. Git Workflow:
   - Main branch: Stable releases
   - Develop branch: Integration
   - Feature branches: New functionality
   - Hotfix branches: Production fixes

2. Testing:

   ```bash
   # Run tests
   cargo test
   
   # Run with logging
   RUST_LOG=debug cargo test
   ```

3. Code Style:

   ```bash
   # Format code
   cargo fmt
   
   # Check lints
   cargo clippy
   ```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Troubleshooting

Common Issues:

1. Exchange Rate API key not found
   - Check .env file exists
   - Verify API key is correct
   - Restart server after changing .env

2. Country not found
   - Check country name spelling
   - Try using official country names
   - Check REST Countries API directly

3. Rate limiting
   - Wait 20 minutes after hitting rate limit
   - Implement caching to reduce API calls
   - Consider upgrading API plan for production use

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- REST Countries API: [https://restcountries.com/](https://restcountries.com/)
- Exchange Rate API: [https://www.exchangerate-api.com/](https://www.exchangerate-api.com/)

## Support

For support, please open an issue in the repository or contact the maintainers.

---

**Note:** This is a living document and will be updated as the project evolves.
