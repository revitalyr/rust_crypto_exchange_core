# Rust Crypto Exchange Core

A high-performance, modular cryptocurrency exchange core engine written in Rust. This project implements fundamental components required for a trading platform, including order matching, risk management, account balances, and order book management.

## 🔐 What Makes It "Crypto"

### Technical Level: Cryptographic Protection & Mathematics

The term "cryptocurrency" originates from **cryptography**. Unlike traditional money (dollars or euros) that physically exist in banks and are protected by state laws, cryptocurrency exists only as a record in a blockchain. This record is protected by **mathematical algorithms**.

**What "crypto" specifically protects:**

-   **Digital Signatures**: When you transfer Bitcoin, you don't just check a box in a banking app. You sign the transaction with your private key (a very long set of characters). This is a digital signature that cannot be forged without knowing the key.

-   **Network Security**: Blockchain (like Bitcoin or Ethereum) uses hashing (SHA-256 and other algorithms) to make it impossible to break or rewrite the chain's history.

**Conclusion**: "Crypto" in the exchange name indicates that it works with assets whose value is based not on a country's gold reserves, but on **mathematical protection and decentralized algorithms**.

### Asset Level: Digital Nature

When we say "crypto exchange," we emphasize that the objects of trading are **digital assets**:

-   **Stock Exchange**: You buy shares of companies — this is a right to a portion of a real business.

-   **Forex Exchange**: You buy fiat money (dollars, euros) issued by central banks.

-   **Crypto Exchange**: You buy cryptocurrencies. These are assets that:
    -   Have no physical embodiment (you can't touch them like a gold coin).
    -   Have no single issuer (often not issued by a state or bank, they are created by code).

### Ideological Level: Decentralization vs Centralization

It's important to note the nuance that often confuses newcomers.

When we say "crypto" in the context of Bitcoin philosophy, we mean **decentralization** — absence of intermediaries. However, a crypto exchange (like Binance, Bybit, etc.) is a **centralized intermediary**.

This creates a paradox: "crypto" (decentralized money) + "exchange" (centralized service).

Therefore, the industry distinguishes:

-   **CEX (Centralized Exchange)**: "Crypto" only by asset type. In terms of management, they work like traditional banks (hold your keys themselves).

-   **DEX (Decentralized Exchange)**: "Crypto" in the full sense. Here the exchange is just a program (smart contract) running on a blockchain, where even the creators cannot take your money or stop a transaction.

**Summary**: In the phrase "cryptocurrency exchange," the word "crypto" means that:

-   The medium of exchange is protected by cryptographic algorithms (not paper laws).
-   The asset exists exclusively in a digital environment based on blockchain.
-   The goal (ideally) is to allow users to manage their finances without control from the state or banks, although exchanges themselves often work according to traditional financial rules.

**Simply put**: "crypto" here = digital assets protected by mathematics.

## 🚀 Key Features

*   **High-Performance Order Book**: Efficient price-level organization with memory pooling for low-latency order management.
*   **Matching Engine**: Supports Limit and Market orders with Time-in-Force policies (GTC, IOC, FOK).
*   **Risk Engine**: Comprehensive pre-trade risk validation including:
    *   Balance checks
    *   Position limits
    *   Daily trading limits
    *   Price deviation and volatility checks
    *   Order frequency rate limiting
*   **Account Management**: Multi-asset wallet system with balance tracking and freezing mechanisms.
*   **Real-time Trade Execution**: Instant order matching with balance updates.
*   **Cryptographic Security**: Digital signatures and transaction verification.
*   **Blockchain Integration**: Support for both centralized and decentralized operation modes.

## 🏗️ Architecture

The project is organized as a modular workspace with the following crates:

-   `crates/common` - Shared types and utilities
-   `crates/orderbook` - High-performance order book implementation
-   `crates/matching_engine` - Order matching and execution engine
-   `crates/risk_engine` - Risk management and validation
-   `crates/accounts` - Account and wallet management
-   `crates/blockchain` - Blockchain integration utilities
-   `crates/transport` - Network transport layer
-   `crates/api_gateway` - REST and WebSocket API gateway
-   `crates/persistence` - Database persistence layer
-   `crates/benchmarks` - Performance benchmarking suite

## 🎯 Demo Application

The project includes a comprehensive demonstration that showcases all core components, including both **Centralized (CEX)** and **Decentralized (DEX)** exchange modes.

### Running Demo

```bash
cargo run --bin exchange-demo
```

### What Demo Shows

The demonstration creates a fully functional crypto exchange with:

1. **🔐 Cryptographic Security**: Digital signature creation and verification
2. **🏦 Centralized Exchange (CEX)**: Traditional exchange with internal processing
3. **🔗 Decentralized Exchange (DEX)**: Blockchain-based exchange with mining
4. **Account Creation**: Multiple user accounts with initial balances
5. **Order Placement**: Limit orders for BTC-USDT trading pair
6. **Order Matching**: Automatic trade execution when orders cross
7. **Balance Management**: Real-time balance updates and freezing
8. **Risk Validation**: Pre-trade checks for sufficient balances
9. **Order Book Management**: Price-time priority ordering
10. **Trade Reporting**: Real-time trade execution logs
11. **Blockchain Mining**: Block creation and transaction confirmation

### Sample Output

```
🚀 Starting Advanced Crypto Exchange Demo
=========================================

🔐 Demonstrating Cryptographic Security:
📝 Original Message: Transfer 1.5 BTC to user2
🔑 Private Key: priv_key_12345
✍️  Digital Signature: SIG_ccf9b7a695c1bddd4fc7b9028a54b4a926b72969d0012454267dddcffd9f34b4:priv_key_12345
✅ Signature Verification: true
❌ Invalid Verification: true

🏦 CENTRALIZED EXCHANGE MODE:
================================
✅ Created account 'user1' with ID: 727d23ea-0b4d-4c5e-92e9-99722b08fe86
✅ Created account 'user2' with ID: 78b0637c-74e2-442c-94a1-d043637760f2
🏦 Processing transaction internally: 4133124c-fe45-4670-b523-7e10969af9bc
💰 Trade executed: 1 BTC @ 45000 USDT
💰 Trade executed: 0.5 BTC @ 45100 USDT

📊 🏦 CENTRALIZED EXCHANGE STATUS - BTC-USDT
=====================================
💰 ACCOUNTS:
  Account: user1 - BTC: 10.00000000, USDT: 9950.00000000
  Account: user2 - BTC: 1.50000000, USDT: 50000.00000000

🔗 DECENTRALIZED EXCHANGE MODE:
==================================
🔗 Added transaction to blockchain: 6852044d-6c5b-4f81-8acc-6fd4dcd56acb
⛏️  Mined block #1 with 2 transactions
   Block hash: 939c0d283d85b39251ea0868f308da593d1ae875c19427466b82d8c2a9b9efb1

📊 🔗 DECENTRALIZED EXCHANGE STATUS - BTC-USDT
=====================================
🔗 BLOCKCHAIN STATUS:
  Current Height: 1
  Total Blocks: 2
  Pending Transactions: 0
  Latest Block Hash: 939c0d283d85b39251ea0868f308da593d1ae875c19427466b82d8c2a9b9efb1

💡 Key Differences:
   CEX: Fast, user-friendly, but custodial (holds your keys)
   DEX: Trustless, you control keys, but slower and more complex
   Both: Handle the same crypto assets with different security models

## 🎬 Presentation Scripts

Easy-to-use scripts for launching the complete crypto exchange demo:

### Quick Start
```bash
# Using Just (recommended)
just presentation              # Full demo with all clients
just presentation-full         # Multiple clients in separate windows
just presentation-demo         # Server only
just presentation-setup        # Build and setup

# Using scripts directly
./scripts/presentation.sh       # Linux/macOS
./scripts/presentation.ps1      # Windows PowerShell
scripts/presentation.bat       # Windows Batch
```

### Presentation Modes
- **Full Presentation**: Server + Desktop + Python clients
- **Server Only**: Demo server with web interface
- **Desktop Only**: GUI client only
- **Setup**: Build and prepare environment

### Features Demonstrated
- ✅ Real-time WebSocket communication
- ✅ Multi-platform client connections
- ✅ Live order book and trading
- ✅ Cross-platform GUI (egui)
- ✅ Automated trading bots
- ✅ Configuration system
- ✅ Cryptographic operations

See [scripts/README.md](scripts/README.md) for detailed usage instructions.

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+
- PostgreSQL 14+ (for database features)
- Redis 6+ (for caching)

### Configuration

The project uses a flexible configuration system:

1. **Environment Variable** (Recommended for production):
   ```bash
   export CRYPTO_EXCHANGE_CONFIG=config/production.toml
   ```

2. **Configuration Files** (in order of priority):
   - `config/production.toml` - Production settings
   - `config/default.toml` - Development defaults  
   - `config/test.toml` - Test environment

3. **Default Values**: If no configuration found, sensible defaults are used

### Running the System

```bash
# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Start matching engine
cargo run --bin matching-engine

# Start API gateway
cargo run --bin api-gateway

# Run desktop client
cd clients/desktop && cargo run
```

## 📁 Project Structure

```
├── config/                 # Configuration files
│   ├── default.toml      # Development settings
│   ├── production.toml   # Production settings
│   └── test.toml         # Test settings
├── crates/                 # Core Rust crates
│   ├── common/           # Shared types and utilities
│   ├── matching_engine/   # Order matching logic
│   ├── risk_engine/      # Risk management
│   ├── accounts/          # Account management
│   └── crypto_operations/ # Cryptographic operations
├── clients/                # Client implementations
│   ├── desktop/          # Rust desktop client
│   ├── python/            # Python client
│   ├── android/           # Android client
│   └── ios/               # iOS client
└── examples/               # Usage examples
    └── config_example.rs   # Configuration demo
```

## ⚙️ Configuration System

The configuration system supports multiple environments and flexible settings:

### Server Settings
- WebSocket URLs for different environments
- Connection timeouts and reconnection logic
- Security configurations

### Client Settings  
- Client identification and versioning
- Auto-reconnection behavior
- Platform-specific settings

### Trading Settings
- Default trading pairs
- Order size limits
- Timeout configurations

### UI Settings
- Theme preferences (light/dark/auto)
- Chart update intervals
- Display customization

### Logging Settings
- Configurable log levels
- File or console output
- Environment-specific defaults

See [config/README.md](config/README.md) for detailed configuration options.

## 🛠️ Development

### Prerequisites

-   Rust 1.70+ with `cargo`
-   Git

### Building

```bash
# Build entire workspace
cargo build --workspace

# Build with optimizations
cargo build --release

# Run demo
cargo run --bin exchange-demo
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run benchmarks
cargo bench --package benchmarks
```

## 📊 Performance

The exchange core is designed for high-performance trading:

-   **Order Book Operations**: O(log n) insertion and removal
-   **Order Matching**: Linear time matching based on price levels
-   **Memory Efficiency**: Object pooling for reduced allocations
-   **Concurrency**: Lock-free data structures where applicable

## 🔧 Configuration

The exchange supports various configuration options:

-   **Trading Pairs**: Multiple asset pairs support
-   **Price Precision**: Configurable decimal precision
-   **Order Size Limits**: Minimum and maximum order sizes
-   **Risk Parameters**: Position limits and frequency checks
-   **Operating Mode**: Centralized (CEX) or Decentralized (DEX)

## 🤝 Contributing

1.  Fork repository
2.  Create a feature branch
3.  Make your changes
4.  Add tests for new functionality
5.  Submit a pull request

## 📝 License

This project is licensed under MIT License - see [LICENSE](LICENSE) file for details.

## 🔮 Roadmap

-   [ ] WebSocket API for real-time updates
-   [ ] Market data streaming
-   [ ] Advanced order types (Stop, Stop-Limit)
-   [ ] Multi-asset collateral support
-   [ ] Liquidity provider programs
-   [ ] Advanced risk management features

---

**Built with ❤️ in Rust for performance and cryptographic security**
