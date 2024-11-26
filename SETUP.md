# Currency Converter Setup Guide

This guide will help you get the World Currency Converter API running on your system quickly and easily.

## Quick Start

### Prerequisites

1. Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

1. Get your API key:

- Sign up at [ExchangeRate API](https://www.exchangerate-api.com/)
- Copy your API key from the dashboard

### Basic Setup

1. Clone the repository:

```bash
git clone https://github.com/akarales/world-currency-converter.git
cd world-currency-converter
```

1. Create your environment file:

```bash
echo "EXCHANGE_RATE_API_KEY=your_api_key_here" > .env
echo "RUST_LOG=info" >> .env
```

1. Build and run the server:

```bash
# First, build the project
cargo build

# Then run it
cargo run
```

The server will start at [http://localhost:8080](http://localhost:8080)

## Running Tests

### Important Note

Before running the test script, make sure you have:

1. Built the project with `cargo build`
1. Started the server with `cargo run`
1. Left the server running in a separate terminal

### Test Script

Open a new terminal and run:

```bash
# Make the script executable
chmod +x test_currency_api.sh

# Run the tests
./test_currency_api.sh
```

### Other Tests

```bash
# Run all tests
cargo test

# Run only API tests
cargo test --test api

# Run only library tests
cargo test --lib

# Run with debug logging
RUST_LOG=debug cargo test
```

## Quick Test

Once the server is running, test if everything is working:

```bash
# Health check
curl http://localhost:8080/health

# Simple conversion
curl -X POST localhost:8080/currency \
  -d '{ "to": "France", "from": "USA", "amount": 100 }'

# Detailed conversion
curl -X POST localhost:8080/v1/currency \
  -H "Content-Type: application/json" \
  -d '{
    "from": "United States",
    "to": "France",
    "amount": 100,
    "preferred_currency": null
  }'
```

## Environment Configuration

### Development Environment

```bash
# .env
EXCHANGE_RATE_API_KEY=your_api_key_here
RUST_LOG=debug
```

### Test Environment

```bash
# .env.test
EXCHANGE_RATE_API_KEY=your_test_api_key
RUST_LOG=debug
```

## Common Tasks

### Update Dependencies

```bash
cargo update
```

### Check Code Style

```bash
cargo fmt
cargo clippy
```

### Clean Build

```bash
cargo clean
cargo build
```

## Troubleshooting

### API Key Issues

1. Verify your API key:

```bash
cat .env | grep EXCHANGE_RATE_API_KEY
```

1. Check environment variable:

```bash
echo $EXCHANGE_RATE_API_KEY
```

### Connection Issues

1. Check if the server is running:

```bash
curl http://localhost:8080/health
```

1. Check external API connectivity:

```bash
curl https://v6.exchangerate-api.com/v6/YOUR_API_KEY/latest/USD
curl https://restcountries.com/v3.1/name/United%20States
```

### Log Analysis

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Save logs to file
RUST_LOG=debug cargo run 2> debug.log
```

## Development Tips

### Watch for Changes

Use `cargo-watch` for automatic rebuilds:

```bash
cargo install cargo-watch
cargo watch -x run
```

### Debug Mode

Run with debug logging:

```bash
RUST_LOG=debug cargo run
```

### Performance Testing

1. Install `hey` for load testing:

```bash
go install github.com/rakyll/hey@latest
```

1. Run a load test:

```bash
hey -n 100 -c 10 -m POST -D '{"from":"USA","to":"France","amount":100}' \
  http://localhost:8080/currency
```

## Next Steps

1. Read the full [README.md](README.md) for detailed documentation
1. Check the API Features Comparison table for endpoint differences
1. Review the Troubleshooting section for common issues
1. Join our community for support

## Support

If you encounter any issues:

1. Check the Troubleshooting section above
1. Review logs for error messages
1. Open an issue in the [repository](https://github.com/akarales/world-currency-converter/issues)
1. Contact the maintainers

---

For more detailed information, please refer to the [README.md](README.md) file.
