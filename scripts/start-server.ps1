# Simple Server Start Script
# Starts demo server without building

param(
    [Parameter(Mandatory=$false)]
    [string]$Config = "default"
)

Write-Host "🚀 Starting Crypto Exchange Server" -ForegroundColor Cyan
Write-Host "=================================" -ForegroundColor Cyan
Write-Host ""

# Check if cargo is available
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Error: cargo not found" -ForegroundColor Red
    Write-Host "Please install Rust toolchain from https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ Cargo found!" -ForegroundColor Green
Write-Host ""

# Set configuration
switch ($Config) {
    "production" {
        $env:CRYPTO_EXCHANGE_CONFIG = "config/production.toml"
        Write-Host "🔧 Using production configuration" -ForegroundColor Yellow
    }
    "test" {
        $env:CRYPTO_EXCHANGE_CONFIG = "config/test.toml"
        Write-Host "🧪 Using test configuration" -ForegroundColor Yellow
    }
    default {
        $env:CRYPTO_EXCHANGE_CONFIG = "config/default.toml"
        Write-Host "🔧 Using default configuration" -ForegroundColor Yellow
    }
}

Write-Host "📡 Starting demo server..." -ForegroundColor Yellow
Write-Host "🌐 WebSocket: ws://127.0.0.1:8080/ws" -ForegroundColor Cyan
Write-Host "🌐 Web UI: http://127.0.0.1:8080" -ForegroundColor Cyan
Write-Host ""
Write-Host "💡 Connect clients to:" -ForegroundColor Gray
Write-Host "   Desktop: cd clients/desktop && cargo run --bin desktop-client" -ForegroundColor Gray
Write-Host "   Python: cd clients/python && python trading_client.py" -ForegroundColor Gray
Write-Host ""
Write-Host "📝 Configuration: $env:CRYPTO_EXCHANGE_CONFIG" -ForegroundColor Blue
Write-Host ""
Write-Host "Press Ctrl+C to stop server" -ForegroundColor Yellow
Write-Host ""

# Change to api_gateway directory and run server
Set-Location crates\api_gateway
try {
    cargo run --bin demo-server
} catch {
    Write-Host "❌ Server failed to start: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "👋 Server stopped!" -ForegroundColor Cyan
