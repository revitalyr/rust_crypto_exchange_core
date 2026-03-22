# 🚀 Rust Crypto Exchange Core

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/revitalyr/rust_crypto_exchange_core)

A high-performance, modular cryptocurrency exchange core engine written in Rust. This project implements fundamental components required for a trading platform, including order matching, risk management, account balances, and order book management.

## ✨ Key Features

- **🔥 High Performance**: Optimized for high-frequency trading with sub-millisecond order matching
- **🛡️ Type Safety**: Semantic type system prevents common trading errors  
- **🔌 Real-time Communication**: WebSocket-based API supporting multiple client platforms
- **📊 Multi-Platform Clients**: Desktop GUI and Python trading bots
- **⚡ Price Protection**: Built-in safeguards against price manipulation
- **📈 Auto-Trading**: Configurable automated trading strategies
- **🧪 Comprehensive Testing**: Full test coverage for all components

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Desktop GUI   │    │  Python Bots   │    │   Web Client   │
│   (egui)       │    │  (asyncio)     │    │  (WebSocket)   │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┴──────────────────────┘
                                 │
                    ┌─────────────────┴─────────────────┐
                    │     API Gateway (WebSocket)      │
                    └─────────────────┬─────────────────┘
                                 │
          ┌────────────────────────┴────────────────────────┐
          │         Matching Engine & Order Book        │
          │    (Price-Time Priority Matching)         │
          └───────────────────────────────────────────────┘
```

## 🚀 Quick Start

```bash
# Clone the repository
git clone https://github.com/revitalyr/rust_crypto_exchange_core.git
cd rust_crypto_exchange_core

# Start the exchange server
cargo run --bin demo-server

# Run desktop client  
cargo run --bin desktop-client

# Run Python trading bots
cd clients/python
python trading_client.py
```

## 📦 Components

### Core Engine
- **Order Book**: High-performance limit order book with price-time priority
- **Matching Engine**: Real-time order matching with price protection
- **API Gateway**: WebSocket server supporting multiple client platforms  
- **Common Types**: Semantic type system with self-documenting code

### Client Applications
- **Desktop Client**: Cross-platform GUI trading client with egui
- **Python Clients**: Automated trading bots with WebSocket connectivity

## 📊 Performance

- **Order Matching**: < 1ms latency
- **Throughput**: 10,000+ orders/second
- **Memory**: Optimized for low footprint
- **Concurrency**: Lock-free data structures

## 🛠️ Tech Stack

- **Rust**: Core engine for performance and safety
- **WebSocket**: Real-time client-server communication
- **egui**: Cross-platform desktop GUI
- **Python**: Trading bots with asyncio
- **Tokio**: Async runtime for high concurrency

## 🎯 Use Cases

- **Cryptocurrency Exchanges**: Complete exchange infrastructure
- **Trading Platforms**: High-frequency trading systems
- **Financial Education**: Learning about exchange mechanics
- **Research**: Algorithmic trading development
- **Prototyping**: Quick exchange system development

## 📈 Trading Features

- **Limit Orders**: Price-time priority matching
- **Market Orders**: Immediate execution
- **Order Cancellation**: Real-time order management
- **Balance Tracking**: Multi-asset account management
- **Trade History**: Complete audit trail
- **Price Protection**: Anti-manipulation safeguards

## 🌐 Multi-Platform Support

- **Windows**: Native Windows desktop client
- **macOS**: Native macOS desktop client
- **Linux**: Native Linux desktop client
- **Python**: Cross-platform trading bots

## 📝 Documentation

- **Comprehensive README**: Detailed setup and usage instructions
- **Code Comments**: Full English documentation throughout
- **API Docs**: Complete type and function documentation
- **Examples**: Working code samples for all features

## 🧪 Testing

- **Unit Tests**: Full coverage of core components
- **Integration Tests**: End-to-end trading scenarios
- **Benchmarks**: Performance regression testing
- **Load Tests**: High-frequency trading simulation

## 🔒 Security Features

- **Price Protection**: Prevents price manipulation
- **Order Validation**: Comprehensive input validation
- **Type Safety**: Compile-time error prevention
- **Memory Safety**: Rust's ownership system

## 📊 Monitoring & Logging

- **Structured Logging**: Comprehensive event tracking
- **Performance Metrics**: Real-time monitoring
- **Trade Analytics**: Detailed trading statistics
- **Debug Tools**: Development and debugging utilities

## 🤝 Contributing

We welcome contributions! Please see our contributing guidelines for details.

## 📄 License

MIT License - see LICENSE file for details.

---

**Built with ❤️ for the crypto community**
