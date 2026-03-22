@echo off
REM Crypto Exchange Presentation Script (Windows Batch)
REM Simple launcher for the demo

title Crypto Exchange Presentation

echo.
echo 🚀 Crypto Exchange Presentation Script
echo =====================================
echo.

REM Check if cargo is available
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo ❌ Error: cargo not found
    echo Please install Rust toolchain from https://rustup.rs/
    pause
    exit /b 1
)

REM Check if python is available
where python >nul 2>nul
if %errorlevel% neq 0 (
    echo ⚠️  Warning: python not found (Python clients won't work)
    echo.
)

echo 🔍 Dependencies checked
echo.

REM Menu
:menu
echo Choose presentation mode:
echo.
echo 1. Full Presentation (server + clients)
echo 2. Server Only
echo 3. Desktop Client Only
echo 4. Setup (build only)
echo 5. Exit
echo.
set /p choice="Enter your choice (1-5): "

if "%choice%"=="1" goto full
if "%choice%"=="2" goto server
if "%choice%"=="3" goto desktop
if "%choice%"=="4" goto setup
if "%choice%"=="5" goto end

echo Invalid choice. Please try again.
echo.
goto menu

:full
echo.
echo 🎯 Starting Full Presentation...
echo ================================
echo.

echo Step 1: Building project...
cargo build --profile fastdev
if %errorlevel% neq 0 (
    echo ❌ Build failed!
    pause
    goto menu
)

echo.
echo Step 2: Building desktop client...
cd clients/desktop
cargo build --profile fastdev
cd ../..
if %errorlevel% neq 0 (
    echo ❌ Desktop client build failed!
    pause
    goto menu
)

echo.
echo 🚀 Starting demo server...
echo 📡 WebSocket: ws://127.0.0.1:8080/ws
echo 🌐 Web UI: http://127.0.0.1:8080
echo.
echo 🖥️ Starting desktop client in new window...
start "Desktop Client" cmd /k "cd clients/desktop && cargo run --bin desktop-client --profile fastdev"

echo.
echo 🐍 Starting Python clients...
if exist "clients\python\trading_client.py" (
    start "Python Client" cmd /k "cd clients/python && python trading_client.py"
)
if exist "clients\python\trading_bot.py" (
    start "Python Bot" cmd /k "cd clients/python && python trading_bot.py"
)

echo.
echo 🎯 Full presentation started!
echo 📊 Multiple clients running in separate windows
echo 🔄 Server will handle all connections
echo.
echo Press Ctrl+C to stop server (clients will continue)
echo.

cargo run --bin demo-server
goto end

:server
echo.
echo 📡 Server-Only Mode
echo =================
echo.

echo Step 1: Building project...
cargo build --profile fastdev
if %errorlevel% neq 0 (
    echo ❌ Build failed!
    pause
    goto menu
)

echo.
echo 🌐 Starting demo server...
echo 📡 WebSocket: ws://127.0.0.1:8080/ws
echo 🌐 Web UI: http://127.0.0.1:8080
echo.
echo 💡 Connect clients manually:
echo    Desktop: just run-desktop
echo    Python: cd clients/python && python trading_client.py
echo.
echo Press Ctrl+C to stop server
echo.

cargo run --bin demo-server
goto end

:desktop
echo.
echo 💻 Desktop Client Mode
echo ======================
echo.

echo Step 1: Building desktop client...
cd clients/desktop
cargo build --profile fastdev
if %errorlevel% neq 0 (
    echo ❌ Build failed!
    cd ../..
    pause
    goto menu
)

echo.
echo 🖥️ Starting desktop client...
echo 🔗 Will connect to: ws://127.0.0.1:8080/ws
echo.
echo Make sure server is running first!
echo.

cargo run --bin desktop-client --profile fastdev
cd ../..
goto end

:setup
echo.
echo 🔧 Presentation Setup Mode
echo =========================
echo.

echo Step 1: Cleaning previous builds...
cargo clean

echo.
echo Step 2: Building project...
cargo build --profile fastdev
if %errorlevel% neq 0 (
    echo ❌ Build failed!
    pause
    goto menu
)

echo.
echo Step 3: Building desktop client...
cd clients/desktop
cargo build --profile fastdev
cd ../..
if %errorlevel% neq 0 (
    echo ❌ Desktop client build failed!
    pause
    goto menu
)

echo.
echo Step 4: Running tests...
cargo test

echo.
echo ✅ Setup complete!
echo 🚀 Run this script again and choose option 1 for full presentation
echo.
pause
goto menu

:end
echo.
echo 👋 Presentation ended!
echo 📝 Check logs for any errors or issues
echo.
pause
