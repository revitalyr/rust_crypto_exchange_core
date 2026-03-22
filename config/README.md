# Configuration Files

This directory contains configuration files for crypto exchange clients.

## Configuration Files

### `default.toml`
Default development configuration with local server settings:
- WebSocket URL: `ws://127.0.0.1:8080/ws`
- Connection timeout: 30 seconds
- Auto-reconnect: enabled
- Theme: auto
- Log level: info

### `production.toml`
Production configuration for live trading:
- WebSocket URL: `wss://api.cryptoexchange.com/ws`
- Enhanced security settings
- Optimized performance settings
- Warning-level logging

### `test.toml`
Test configuration for development and testing:
- WebSocket URL: `ws://127.0.0.1:8081/ws`
- Fast reconnection (1 second)
- Debug logging enabled
- Light theme default

## Configuration Structure

```toml
[server]
url = "ws://127.0.0.1:8080/ws"
timeout = 30
reconnect_interval = 5
max_reconnect_attempts = 10

[client]
name = "Desktop Trading Client"
version = "1.0.0"
auto_reconnect = true

[trading]
default_pair = "BTC/USDT"
max_order_size = 1000000
order_timeout = 60

[ui]
theme = "auto"
chart_update_interval = 1000
max_chart_points = 100

[logging]
level = "info"
# file = "logs/client.log"  # Optional
```

## Usage

### Environment Variable
Set the `CRYPTO_EXCHANGE_CONFIG` environment variable to specify a custom configuration file:

```bash
export CRYPTO_EXCHANGE_CONFIG=/path/to/custom.toml
```

### Client Loading
Clients automatically load configuration in this order:
1. Environment variable `CRYPTO_EXCHANGE_CONFIG`
2. `config/production.toml`
3. `config/default.toml`
4. `config/test.toml`
5. Default values

## Security Notes

- **Never commit configuration files with secrets** to version control
- Use environment variables for sensitive data in production
- Production configuration should use `wss://` (secure WebSocket)
- Test configuration should use different ports to avoid conflicts

## Customization

Copy any configuration file and modify it for your needs:
- Change server URLs for different environments
- Adjust timeouts and reconnection settings
- Customize UI preferences
- Set appropriate logging levels

## Validation

Configuration files are automatically validated on load:
- Required fields must be present
- URLs must be valid WebSocket endpoints
- Numeric values must be positive
- Log levels must be valid (trace, debug, info, warn, error)
