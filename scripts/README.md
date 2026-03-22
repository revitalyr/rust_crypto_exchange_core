# 🚀 Presentation Scripts

This directory contains scripts for launching the crypto exchange presentation demo.

## 📁 Available Scripts

### `presentation.ps1` (Windows PowerShell)
Cross-platform PowerShell script for Windows systems.

### `presentation.sh` (Linux/macOS Bash)
Bash script for Unix-like systems.

## 🎯 Usage

### Quick Start
```bash
# Windows
.\scripts\presentation.ps1

# Linux/macOS
./scripts/presentation.sh
```

### Available Modes

#### Full Presentation (Default)
Starts the complete demo with server and multiple clients:
- Demo server with web interface
- Desktop GUI client
- Python trading clients
- Real-time interactions

```bash
# Windows
.\scripts\presentation.ps1 -Mode full

# Linux/macOS
./scripts/presentation.sh full
```

#### Server Only
Starts only the demo server:
- WebSocket server on `ws://127.0.0.1:8080/ws`
- Web interface at `http://127.0.0.1:8080`
- Manual client connection required

```bash
# Windows
.\scripts\presentation.ps1 -Mode server

# Linux/macOS
./scripts/presentation.sh server
```

#### Desktop Client Only
Starts only the desktop client:
- Requires server to be running first
- GUI application with trading interface

```bash
# Windows
.\scripts\presentation.ps1 -Mode desktop

# Linux/macOS
./scripts/presentation.sh desktop
```

#### Setup Mode
Builds and prepares the environment:
- Cleans previous builds
- Builds all components
- Runs tests
- No server started

```bash
# Windows
.\scripts\presentation.ps1 -Mode setup

# Linux/macOS
./scripts/presentation.sh setup
```

## 🔧 Requirements

### System Dependencies
- **Rust** 1.70+ with Cargo
- **Python** 3.8+ (for Python clients)
- **PowerShell** 5.0+ (Windows)
- **Bash** 4.0+ (Linux/macOS)

### Project Dependencies
The scripts automatically check for:
- `cargo` command availability
- `python` command availability
- Required project files

## 🌐 Demo Features

### Server Components
- **WebSocket API**: Real-time bidirectional communication
- **Web Interface**: HTTP server with demo pages
- **Order Matching**: Live order book and trade execution
- **Multi-client Support**: Concurrent connections

### Client Components
- **Desktop Client**: Cross-platform GUI with egui
- **Python Clients**: Trading bots and manual trading
- **Real-time Updates**: Live price charts and order books
- **Trading Features**: Market/limit orders, balance tracking

### Presentation Flow
1. **Setup**: Build all components
2. **Server**: Start demo server
3. **Clients**: Launch trading interfaces
4. **Demo**: Show real-time trading interactions
5. **Cleanup**: Stop server (clients continue if desired)

## 📱 Access Points

Once running, access the demo at:

- **WebSocket**: `ws://127.0.0.1:8080/ws`
- **Web Interface**: `http://127.0.0.1:8080`
- **Desktop Client**: Separate GUI window
- **Python Clients**: Terminal interfaces

## 🛠️ Troubleshooting

### Common Issues

#### Port Already in Use
```bash
# Check what's using port 8080
netstat -tulpn | grep :8080  # Linux
netstat -ano | findstr :8080  # Windows

# Kill the process
kill -PID <process_id>  # Linux
taskkill /PID <process_id> /F  # Windows
```

#### Build Failures
```bash
# Clean and rebuild
cargo clean
cargo build --profile fastdev
```

#### Python Dependencies
```bash
# Install required packages
pip install -r clients/python/requirements.txt
```

#### Permission Issues (Linux/macOS)
```bash
# Make script executable
chmod +x scripts/presentation.sh
```

### Debug Mode
For troubleshooting, run individual components:

```bash
# Build only
cargo build --profile fastdev

# Server only
cargo run --bin demo-server

# Desktop client only
cd clients/desktop && cargo run --bin desktop-client

# Python client only
cd clients/python && python trading_client.py
```

## 🎮 Demo Script

Here's a typical presentation flow:

```bash
# 1. Setup environment
./scripts/presentation.sh setup

# 2. Start full presentation
./scripts/presentation.sh full

# 3. In separate terminals, connect more clients:
./scripts/presentation.sh desktop
cd clients/python && python trading_client.py

# 4. Demo features:
# - Place orders from desktop client
# - Watch real-time updates in Python clients
# - Monitor web interface for order book
# - Show auto-trading bots in action

# 5. Stop server (Ctrl+C)
# Clients continue running for manual exploration
```

## 🔧 Customization

### Modify Server Settings
Edit `config/default.toml`:
```toml
[server]
url = "ws://127.0.0.1:8080/ws"
timeout = 30
```

### Change Presentation Mode
Modify script variables:
- `DEFAULT_MODE` in `presentation.sh`
- `$Mode` default in `presentation.ps1`

### Add New Clients
Extend the script to launch additional clients:
```bash
# Add new client launch
echo "🚀 Starting new client..."
start cmd /k "cd clients/new && cargo run"
```

## 📞 Support

If you encounter issues:

1. Check system requirements above
2. Run `presentation.sh setup` first
3. Check individual component logs
4. Verify port 8080 is available
5. Ensure all dependencies are installed

For additional help, check the main project README or open an issue.
