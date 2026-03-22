# 🚀 Crypto Exchange Multi-Platform Clients

This directory contains real trading clients for different platforms that connect to the crypto exchange server via WebSocket.

## 📱 Available Clients

### 🐍 Python Client
**Location**: `clients/python/`

**Features**:
- Real-time WebSocket connection
- Advanced trading strategies (Mean Reversion, Momentum, Market Making)
- Risk management and position sizing
- Technical indicators (SMA, EMA, RSI, Bollinger Bands, MACD)
- Performance tracking and analytics
- Multiple concurrent trading bots
- Comprehensive logging and monitoring

**Requirements**:
```bash
pip install -r requirements.txt
```

**Usage**:
```bash
# Basic trading client
python trading_client.py

# Advanced trading bot with strategies
python trading_bot.py
```

**Key Components**:
- `trading_client.py` - Basic WebSocket client with manual trading
- `trading_bot.py` - Advanced bot with multiple strategies and risk management

---

### 🖥️ Desktop Client
**Location**: `clients/desktop/`

**Features**:
- Cross-platform GUI using egui
- Real-time price charts
- Order book visualization
- Balance management
- Order history and tracking
- Auto-trading capabilities
- Dark/Light theme support
- Responsive design

**Requirements**:
- Rust 1.70+
- Modern graphics drivers

**Build & Run**:
```bash
cd clients/desktop
cargo run --bin desktop-client
```

**Key Features**:
- **Real-time GUI** - Modern, responsive interface
- **Price Charts** - Live price visualization with historical data
- **Order Management** - Place, cancel, and track orders
- **Balance Panel** - Monitor available and reserved balances
- **Auto Trading** - Automated trading with configurable intervals
- **Theme Support** - Switch between dark and light modes

---

### 📱 Android Client
**Location**: `clients/android/`

**Features**:
- Native Android app using Jetpack Compose
- Real-time WebSocket connection
- Material Design 3 UI
- Order placement and management
- Balance tracking
- Order book visualization
- Recent trades display
- Auto-trading toggle

**Requirements**:
- Android Studio
- Android SDK API 24+
- Kotlin 1.8+

**Build & Run**:
```bash
cd clients/android
./gradlew assembleDebug
./gradlew installDebug
```

**Architecture**:
- **MVVM Pattern** - Clean separation of concerns
- **Jetpack Compose** - Modern declarative UI
- **Coroutines** - Asynchronous operations
- **WebSocket Client** - Real-time communication
- **State Management** - Reactive UI updates

---

### 🍎 iOS Client
**Location**: `clients/ios/`

**Features**:
- Native iOS app using SwiftUI
- Real-time WebSocket connection with Starscream
- Modern iOS design patterns
- Order placement and management
- Balance tracking
- Order book visualization
- Recent trades display
- Auto-trading capabilities

**Requirements**:
- Xcode 15+
- iOS 17+
- Swift 5.9+

**Build & Run**:
```bash
cd clients/ios
open CryptoExchange.xcodeproj
# Build and run in Xcode
```

**Architecture**:
- **MVVM Pattern** - ObservableObject for state management
- **SwiftUI** - Declarative UI framework
- **Starscream** - WebSocket library
- **Codable** - JSON serialization
- **Combine** - Reactive programming (optional)

---

## 🌐 Server Integration

All clients connect to the same WebSocket server:
- **URL**: `ws://127.0.0.1:8080/ws`
- **Protocol**: JSON-based WebSocket messages
- **Real-time**: Bidirectional communication
- **Multi-client**: Concurrent connections supported

### Message Protocol

```json
{
  "type": "PlaceOrder",
  "id": 12345,
  "side": "buy",
  "order_type": "market",
  "price": 5000000,
  "quantity": 1000,
  "pair": "BTC/USDT"
}
```

### Supported Message Types
- `Identify` - Client registration
- `PlaceOrder` - Order placement
- `CancelOrder` - Order cancellation
- `GetBalance` - Balance request
- `GetOrderBook` - Order book request
- `OrderBookUpdate` - Real-time order book updates
- `Trade` - Trade notifications
- `BalanceUpdate` - Balance updates
- `OrderUpdate` - Order status updates

---

## 🎯 Trading Features

### Order Types
- **Market Orders** - Instant execution at best price
- **Limit Orders** - Price-specific execution
- **Order Sides** - Buy/Sell operations

### Trading Pairs
- **BTC/USDT** - Bitcoin to Tether
- **ETH/USDT** - Ethereum to Tether
- **Extensible** - Easy to add new pairs

### Risk Management
- **Position Sizing** - Calculate optimal order sizes
- **Stop Loss** - Automatic loss protection
- **Take Profit** - Automatic profit taking
- **Portfolio Management** - Balance tracking

### Trading Strategies (Python)
- **Mean Reversion** - Trade price deviations
- **Momentum** - Follow price trends
- **Market Making** - Provide liquidity
- **Technical Analysis** - Indicators and signals

---

## 📊 Performance & Analytics

### Real-time Metrics
- **Portfolio Value** - Total account value
- **P&L Tracking** - Profit and loss monitoring
- **Trade Count** - Number of executed trades
- **Win Rate** - Success rate percentage
- **Drawdown** - Maximum loss tracking

### Historical Data
- **Price Charts** - Visual price history
- **Trade History** - Complete trade log
- **Balance History** - Account balance over time
- **Performance Reports** - Strategy effectiveness

---

## 🔧 Configuration System

### Overview
All clients now use a **flexible configuration system** that eliminates hardcoded data and provides:
- **Environment-specific settings** (development, test, production)
- **Centralized configuration management**
- **Secure credential handling**
- **Easy deployment and scaling**

### Configuration Files
Located in `config/` directory:
- `default.toml` - Development settings
- `production.toml` - Production environment  
- `test.toml` - Testing environment

### Loading Priority
1. **Environment Variable** `CRYPTO_EXCHANGE_CONFIG`
2. **Production** config/production.toml
3. **Default** config/default.toml
4. **Test** config/test.toml
5. **Built-in defaults**

### Configuration Structure
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

### Client Implementation

#### Rust Desktop Client
```rust
// Load configuration automatically
let config = AppConfig::load()?;
let mut app = TradingApp::with_config(config);
```

#### Python Client
```python
# Load configuration automatically
config = AppConfig.load()
client = ExchangeClient("python_trader", Platform.PYTHON, config)
```

### Environment Usage

#### Development
```bash
# Uses default.toml automatically
cargo run --bin desktop-client
```

#### Production
```bash
# Use production configuration
export CRYPTO_EXCHANGE_CONFIG=config/production.toml
cargo run --bin desktop-client
```

#### Testing
```bash
# Use test configuration
export CRYPTO_EXCHANGE_CONFIG=config/test.toml
python trading_client.py
```

### Benefits

✅ **No Hardcoded Values** - All settings externalized  
✅ **Environment Isolation** - Different settings per environment  
✅ **Easy Deployment** - Change config without recompilation  
✅ **Security** - No credentials in source code  
✅ **Maintainability** - Centralized configuration management  
✅ **Flexibility** - Runtime configuration changes  

### Migration from Hardcoded Values

**Before (Hardcoded)**:
```rust
let url = "ws://127.0.0.1:8080/ws";
let timeout = 30;
let theme = "dark";
```

**After (Configuration)**:
```rust
let config = AppConfig::load()?;
let url = &config.server.url;
let timeout = config.server.timeout;
let theme = &config.ui.theme;
```

---

## 🚀 Quick Start

### 1. Start the Server
```bash
cd crates/api_gateway
cargo run --bin demo-server
```

### 2. Run Python Client
```bash
cd clients/python
python trading_client.py
```

### 3. Run Desktop Client
```bash
cd clients/desktop
cargo run --bin desktop-client
```

### 4. Build Mobile Clients
```bash
# Android
cd clients/android
./gradlew assembleDebug

# iOS
cd clients/ios
open CryptoExchange.xcodeproj
```

---

## 📱 Platform-Specific Notes

### Android
- Uses Jetpack Compose for modern UI
- WebSocket client with background service
- Material Design 3 components
- Permission for network access

### iOS
- SwiftUI for declarative UI
- Starscream for WebSocket connection
- iOS 17+ features
- Background app refresh support

### Desktop
- egui for cross-platform GUI
- Hardware acceleration
- Custom rendering pipeline
- Window management

### Python
- Asyncio for concurrency
- WebSockets for real-time communication
- NumPy/Pandas for analysis
- Matplotlib for visualization

---

## 🔄 Multi-Client Demo

### Scenario Setup
1. **Start Server** - Launch the exchange server
2. **Connect Clients** - Connect all platform clients
3. **Initialize Data** - Load demo balances and order books
4. **Start Trading** - Begin manual or auto trading

### Expected Behavior
- **Real-time Updates** - All clients see trades instantly
- **Cross-platform** - Orders from any platform affect all others
- **Consistent State** - Synchronized balances and order books
- **Performance** - Handle multiple concurrent connections

### Testing Commands
```bash
# Start multiple Python clients
python -c "
import asyncio
from trading_client import ExchangeClient, Platform

async def run_multiple():
    clients = [
        ExchangeClient(f'bot_{i}', Platform.PYTHON)
        for i in range(5)
    ]
    
    tasks = [client.connect() for client in clients]
    await asyncio.gather(*tasks)

asyncio.run(run_multiple())
"
```

---

## 🐛 Troubleshooting

### Connection Issues
- **Server Not Running** - Start the demo server first
- **Firewall** - Check port 8080 accessibility
- **Network** - Verify localhost connectivity
- **WebSocket** - Ensure WebSocket support

### Performance Issues
- **Memory Usage** - Monitor client memory consumption
- **CPU Usage** - Check for high CPU utilization
- **Network Latency** - Measure WebSocket message latency
- **UI Responsiveness** - Ensure smooth animations

### Platform-Specific Issues
- **Android** - Check network permissions and API level
- **iOS** - Verify Info.plist network settings
- **Desktop** - Update graphics drivers
- **Python** - Install required dependencies

---

## 📈 Future Enhancements

### Planned Features
- **Authentication** - JWT-based client authentication
- **Persistence** - Local data storage
- **Advanced Charts** - Technical analysis tools
- **Portfolio Analytics** - Enhanced reporting
- **Mobile Notifications** - Trade alerts
- **API Integration** - REST API support

### Technical Improvements
- **Load Balancing** - Multiple server instances
- **Message Queuing** - Reliable message delivery
- **Compression** - WebSocket message compression
- **Security** - TLS encryption and authentication
- **Testing** - Automated test suites
- **Documentation** - API documentation

---

## 📞 Support

### Getting Help
- **Issues** - Report bugs on GitHub
- **Documentation** - Check inline comments
- **Examples** - Review sample code
- **Community** - Join discussions

### Contributing
- **Pull Requests** - Submit improvements
- **Bug Reports** - Detailed issue reports
- **Feature Requests** - Enhancement suggestions
- **Documentation** - Help improve docs

---

## 📄 License

This project is licensed under the MIT License. See individual client directories for specific licensing information.
