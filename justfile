# Justfile for fast build commands
# Install with: `cargo install just`

# Default command
default: help

# Help
help:
    @echo "Available commands:"
    @echo "  build              - Build the entire project"
    @echo "  build-dev          - Build in dev mode (faster)"
    @echo "  build-release      - Build in release mode"
    @echo "  run-server         - Run the demo server"
    @echo "  run-desktop        - Run desktop client"
    @echo "  clean              - Clean build artifacts"
    @echo "  test               - Run tests"
    @echo "  check              - Check code without building"
    @echo "  bench              - Run benchmarks"
    @echo ""
    @echo "Presentation commands:"
    @echo "  presentation       - Full presentation (build + server + client)"
    @echo "  presentation-full  - Full demo with multiple clients"
    @echo "  presentation-demo  - Quick demo (server only)"
    @echo "  presentation-setup - Setup environment (build only)"

# Fast development build
build-dev:
    @echo "Building in development mode..."
    cargo build --profile fastdev

# Full build
build:
    @echo "Building project..."
    cargo build

# Release build
build-release:
    @echo "Building release version..."
    cargo build --release

# Run demo server
run-server:
    @echo "Starting demo server..."
    cargo run --bin demo-server

# Run desktop client
run-desktop:
    @echo "Starting desktop client..."
    cd clients/desktop && cargo run --bin desktop-client --profile fastdev

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean
    cd clients/desktop && cargo clean

# Run tests
test:
    @echo "Running tests..."
    cargo test

# Check code without building
check:
    @echo "Checking code..."
    cargo check

# Run benchmarks
bench:
    @echo "Running benchmarks..."
    cargo bench

# Build only specific crate
build-crate CRATE:
    @echo "Building crate {{CRATE}}..."
    cargo build -p {{CRATE}}

# Run with specific profile
run-with-profile PROFILE:
    @echo "Running with profile {{PROFILE}}..."
    cargo run --profile {{PROFILE}}

# Parallel build with all cores
build-parallel:
    @echo "Building with all cores..."
    CARGO_BUILD_JOBS=8 cargo build

# Update dependencies
update:
    @echo "Updating dependencies..."
    cargo update

# Check for unused dependencies
check-unused:
    @echo "Checking for unused dependencies..."
    cargo machete

# Install development tools
install-tools:
    @echo "Installing development tools..."
    cargo install cargo-watch cargo-machete cargo-audit cargo-deny

# Watch for changes and rebuild
watch:
    @echo "Watching for changes..."
    cargo watch -x 'build --profile fastdev'

# Format code
fmt:
    @echo "Formatting code..."
    cargo fmt

# Lint code
lint:
    @echo "Linting code..."
    cargo clippy -- -D warnings

# Security audit
audit:
    @echo "Running security audit..."
    cargo audit

# Generate documentation
docs:
    @echo "Generating documentation..."
    cargo doc --open

# Build all clients
build-clients:
    @echo "Building all clients..."
    cd clients/desktop && cargo build --profile fastdev
    @echo "Desktop client built successfully"

# Development workflow
dev: build-dev run-server

# Production workflow
prod: build-release run-server

# Quick test workflow
quick-test: build-dev test

# Full CI workflow
ci: fmt lint test audit build-release

# Presentation workflow
presentation:
    @echo "🚀 Starting Crypto Exchange Presentation..."
    @echo "=========================================="
    @echo ""
    @echo "Step 1: Building project..."
    just build-dev
    @echo ""
    @echo "Step 2: Building clients..."
    just build-clients
    @echo ""
    @echo "Step 3: Starting demo server..."
    @echo "📡 Server will start on ws://127.0.0.1:8080/ws"
    @echo "🌐 Web interface will be available at http://127.0.0.1:8080"
    @echo ""
    @echo "Step 4: Starting desktop client in new window..."
    @echo "💻 Desktop client will connect automatically"
    @echo ""
    @echo "🎯 Presentation ready! Press Ctrl+C to stop server"
    @echo ""
    # Start server and keep it running
    cargo run --bin demo-server

# Full presentation with multiple clients
presentation-full:
    @echo "🚀 Starting Full Crypto Exchange Presentation..."
    @echo "=================================================="
    @echo ""
    @echo "Step 1: Building project..."
    just build-dev
    @echo ""
    @echo "Step 2: Building all clients..."
    just build-clients
    @echo ""
    @echo "Step 3: Starting demo server..."
    @echo "📡 Server: ws://127.0.0.1:8080/ws"
    @echo "🌐 Web UI: http://127.0.0.1:8080"
    @echo ""
    @echo "Step 4: Starting desktop client..."
    start cmd /k "cd clients/desktop && cargo run --bin desktop-client --profile fastdev"
    @echo ""
    @echo "Step 5: Starting Python trading bots..."
    start cmd /k "cd clients/python && python trading_client.py"
    start cmd /k "cd clients/python && python trading_bot.py"
    @echo ""
    @echo "🎯 Full presentation started!"
    @echo "📊 Multiple clients are running in separate windows"
    @echo "🔄 Server is handling all connections"
    @echo ""
    @echo "Press Ctrl+C to stop server (clients will continue running)"
    @echo ""
    # Start server
    cargo run --bin demo-server

# Quick presentation demo (server only)
presentation-demo:
    @echo "⚡ Quick Demo - Server Only"
    @echo "============================="
    @echo ""
    @echo "📡 Starting demo server..."
    @echo "🌐 Connect clients to: ws://127.0.0.1:8080/ws"
    @echo "🌐 Web interface: http://127.0.0.1:8080"
    @echo ""
    @echo "💡 Run 'just run-desktop' in another terminal to connect desktop client"
    @echo "💡 Run 'cd clients/python && python trading_client.py' for Python client"
    @echo ""
    cargo run --bin demo-server

# Presentation setup (build only)
presentation-setup:
    @echo "🔧 Setting up presentation environment..."
    @echo "======================================="
    @echo ""
    @echo "Step 1: Cleaning previous builds..."
    just clean
    @echo ""
    @echo "Step 2: Building project..."
    just build-dev
    @echo ""
    @echo "Step 3: Building clients..."
    just build-clients
    @echo ""
    @echo "Step 4: Running tests..."
    just test
    @echo ""
    @echo "✅ Presentation setup complete!"
    @echo "🚀 Run 'just presentation' to start the demo"
    @echo ""
