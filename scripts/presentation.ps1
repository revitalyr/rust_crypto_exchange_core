# Crypto Exchange Presentation Script
# PowerShell script for launching the complete demo

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("full", "server", "desktop", "setup")]
    [string]$Mode = "full"
)

Write-Host "🚀 Crypto Exchange Presentation Script" -ForegroundColor Cyan
Write-Host "=====================================" -ForegroundColor Cyan
Write-Host ""

# Function to check if command exists
function Test-Command($cmdname) {
    return [bool](Get-Command -Name $cmdname -ErrorAction SilentlyContinue)
}

# Function to run with colored output
function Run-Step($step, $command, $description) {
    Write-Host "Step $step`: $description" -ForegroundColor Yellow
    Write-Host "----------------------------------------" -ForegroundColor Gray
    
    try {
        Invoke-Expression $command
        if ($LASTEXITCODE -eq 0) {
            Write-Host "✅ Step $step completed successfully!" -ForegroundColor Green
        } else {
            Write-Host "❌ Step $step failed with exit code $LASTEXITCODE" -ForegroundColor Red
            if ($Mode -ne "setup") {
                Write-Host "Continue anyway? (y/n): " -ForegroundColor Yellow -NoNewline
                $continue = Read-Host
                if ($continue -ne "y") {
                    exit 1
                }
            }
        }
    } catch {
        Write-Host "❌ Step $step failed: $_" -ForegroundColor Red
        if ($Mode -ne "setup") {
            Write-Host "Continue anyway? (y/n): " -ForegroundColor Yellow -NoNewline
            $continue = Read-Host
            if ($continue -ne "y") {
                exit 1
            }
        }
    }
    Write-Host ""
}

# Check dependencies
Write-Host "🔍 Checking dependencies..." -ForegroundColor Blue
$missingDeps = @()

if (-not (Test-Command "cargo")) {
    $missingDeps += "cargo (Rust toolchain)"
}
if (-not (Test-Command "python")) {
    $missingDeps += "python"
}

if ($missingDeps.Count -gt 0) {
    Write-Host "❌ Missing dependencies:" -ForegroundColor Red
    foreach ($dep in $missingDeps) {
        Write-Host "  - $dep" -ForegroundColor Red
    }
    Write-Host ""
    Write-Host "Please install missing dependencies and try again." -ForegroundColor Yellow
    exit 1
}

Write-Host "✅ All dependencies found!" -ForegroundColor Green
Write-Host ""

# Main presentation logic
switch ($Mode) {
    "setup" {
        Write-Host "🔧 Presentation Setup Mode" -ForegroundColor Magenta
        Write-Host "=========================" -ForegroundColor Magenta
        Write-Host ""
        
        Run-Step "1" "cargo clean" "Cleaning previous builds"
        Run-Step "2" "cargo build --profile fastdev" "Building project"
        Run-Step "3" "cd clients/desktop; cargo build --profile fastdev" "Building desktop client"
        Run-Step "4" "cargo test" "Running tests"
        
        Write-Host "🎯 Setup complete!" -ForegroundColor Green
        Write-Host "🚀 Run 'presentation.ps1' to start the demo" -ForegroundColor Cyan
    }
    
    "server" {
        Write-Host "📡 Server-Only Mode" -ForegroundColor Magenta
        Write-Host "==================" -ForegroundColor Magenta
        Write-Host ""
        
        Run-Step "1" "cargo build --profile fastdev" "Building project"
        
        Write-Host "🌐 Starting demo server..." -ForegroundColor Yellow
        Write-Host "📡 WebSocket: ws://127.0.0.1:8080/ws" -ForegroundColor Cyan
        Write-Host "🌐 Web UI: http://127.0.0.1:8080" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "💡 Connect clients manually:" -ForegroundColor Gray
        Write-Host "   Desktop: just run-desktop" -ForegroundColor Gray
        Write-Host "   Python: cd clients/python && python trading_client.py" -ForegroundColor Gray
        Write-Host ""
        Write-Host "Press Ctrl+C to stop server" -ForegroundColor Yellow
        Write-Host ""
        
        cargo run --bin demo-server
    }
    
    "desktop" {
        Write-Host "💻 Desktop Client Mode" -ForegroundColor Magenta
        Write-Host "======================" -ForegroundColor Magenta
        Write-Host ""
        
        Run-Step "1" "cd clients/desktop; cargo build --profile fastdev" "Building desktop client"
        
        Write-Host "🖥️ Starting desktop client..." -ForegroundColor Yellow
        Write-Host "🔗 Will connect to: ws://127.0.0.1:8080/ws" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "Make sure server is running first!" -ForegroundColor Yellow
        Write-Host ""
        
        cd clients/desktop
        cargo run --bin desktop-client --profile fastdev
    }
    
    "full" {
        Write-Host "🎯 Full Presentation Mode" -ForegroundColor Magenta
        Write-Host "========================" -ForegroundColor Magenta
        Write-Host ""
        
        Run-Step "1" "cargo build --profile fastdev" "Building project"
        Run-Step "2" "cd clients/desktop; cargo build --profile fastdev" "Building desktop client"
        
        Write-Host "🚀 Starting full presentation..." -ForegroundColor Yellow
        Write-Host ""
        Write-Host "📡 Server: ws://127.0.0.1:8080/ws" -ForegroundColor Cyan
        Write-Host "🌐 Web UI: http://127.0.0.1:8080" -ForegroundColor Cyan
        Write-Host ""
        
        # Start desktop client in new window
        Write-Host "🖥️ Starting desktop client..." -ForegroundColor Yellow
        Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd clients/desktop; cargo run --bin desktop-client --profile fastdev"
        
        # Start Python clients if available
        if (Test-Path "clients/python/trading_client.py") {
            Write-Host "🐍 Starting Python trading client..." -ForegroundColor Yellow
            Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd clients/python; python trading_client.py"
        }
        
        if (Test-Path "clients/python/trading_bot.py") {
            Write-Host "🤖 Starting Python trading bot..." -ForegroundColor Yellow
            Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd clients/python; python trading_bot.py"
        }
        
        Write-Host ""
        Write-Host "🎯 Full presentation started!" -ForegroundColor Green
        Write-Host "📊 Multiple clients running in separate windows" -ForegroundColor Gray
        Write-Host "🔄 Server will handle all connections" -ForegroundColor Gray
        Write-Host ""
        Write-Host "Press Ctrl+C to stop server (clients will continue)" -ForegroundColor Yellow
        Write-Host ""
        
        cargo run --bin demo-server
    }
}

Write-Host ""
Write-Host "👋 Presentation ended!" -ForegroundColor Cyan
Write-Host "📝 Check logs for any errors or issues" -ForegroundColor Gray
