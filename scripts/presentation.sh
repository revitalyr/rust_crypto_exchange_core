#!/bin/bash
# Crypto Exchange Presentation Script
# Bash script for launching the complete demo

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
GRAY='\033[0;37m'
NC='\033[0m' # No Color

# Default mode
MODE="full"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --mode|-m)
            MODE="$2"
            shift 2
            ;;
        full|server|desktop|setup)
            MODE="$1"
            shift
            ;;
        -h|--help)
            echo "Crypto Exchange Presentation Script"
            echo "Usage: $0 [MODE]"
            echo ""
            echo "Modes:"
            echo "  full     - Full presentation with multiple clients (default)"
            echo "  server   - Server only demo"
            echo "  desktop  - Desktop client only"
            echo "  setup    - Build and setup environment"
            echo ""
            echo "Examples:"
            echo "  $0              # Full presentation"
            echo "  $0 server        # Server only"
            echo "  $0 --mode setup  # Setup mode"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use -h for help"
            exit 1
            ;;
    esac
done

# Function to check if command exists
check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}❌ Missing: $1${NC}"
        return 1
    fi
}

# Function to run step with error handling
run_step() {
    local step=$1
    local command=$2
    local description=$3
    
    echo -e "${YELLOW}Step $step: $description${NC}"
    echo -e "${GRAY}----------------------------------------${NC}"
    
    if eval "$command"; then
        echo -e "${GREEN}✅ Step $step completed successfully!${NC}"
    else
        echo -e "${RED}❌ Step $step failed with exit code $?${NC}"
        if [ "$MODE" != "setup" ]; then
            echo -e "${YELLOW}Continue anyway? (y/n): ${NC}" 
            read -r continue
            if [ "$continue" != "y" ]; then
                exit 1
            fi
        fi
    fi
    echo ""
}

# Header
echo -e "${CYAN}🚀 Crypto Exchange Presentation Script${NC}"
echo -e "${CYAN}=====================================${NC}"
echo ""

# Check dependencies
echo -e "${BLUE}🔍 Checking dependencies...${NC}"
missing_deps=()

if ! check_command cargo; then
    missing_deps+=("cargo (Rust toolchain)")
fi
if ! check_command python; then
    missing_deps+=("python")
fi

if [ ${#missing_deps[@]} -gt 0 ]; then
    echo -e "${RED}❌ Missing dependencies:${NC}"
    for dep in "${missing_deps[@]}"; do
        echo -e "${RED}  - $dep${NC}"
    done
    echo ""
    echo -e "${YELLOW}Please install missing dependencies and try again.${NC}"
    exit 1
fi

echo -e "${GREEN}✅ All dependencies found!${NC}"
echo ""

# Main presentation logic
case $MODE in
    "setup")
        echo -e "${MAGENTA}🔧 Presentation Setup Mode${NC}"
        echo -e "${MAGENTA}=========================${NC}"
        echo ""
        
        run_step "1" "cargo clean" "Cleaning previous builds"
        run_step "2" "cargo build --profile fastdev" "Building project"
        run_step "3" "cd clients/desktop && cargo build --profile fastdev" "Building desktop client"
        run_step "4" "cargo test" "Running tests"
        
        echo -e "${GREEN}🎯 Setup complete!${NC}"
        echo -e "${CYAN}🚀 Run './scripts/presentation.sh' to start the demo${NC}"
        ;;
    
    "server")
        echo -e "${MAGENTA}📡 Server-Only Mode${NC}"
        echo -e "${MAGENTA}==================${NC}"
        echo ""
        
        run_step "1" "cargo build --profile fastdev" "Building project"
        
        echo -e "${YELLOW}🌐 Starting demo server...${NC}"
        echo -e "${CYAN}📡 WebSocket: ws://127.0.0.1:8080/ws${NC}"
        echo -e "${CYAN}🌐 Web UI: http://127.0.0.1:8080${NC}"
        echo ""
        echo -e "${GRAY}💡 Connect clients manually:${NC}"
        echo -e "${GRAY}   Desktop: just run-desktop${NC}"
        echo -e "${GRAY}   Python: cd clients/python && python trading_client.py${NC}"
        echo ""
        echo -e "${YELLOW}Press Ctrl+C to stop server${NC}"
        echo ""
        
        cargo run --bin demo-server
        ;;
    
    "desktop")
        echo -e "${MAGENTA}💻 Desktop Client Mode${NC}"
        echo -e "${MAGENTA}======================${NC}"
        echo ""
        
        run_step "1" "cd clients/desktop && cargo build --profile fastdev" "Building desktop client"
        
        echo -e "${YELLOW}🖥️ Starting desktop client...${NC}"
        echo -e "${CYAN}🔗 Will connect to: ws://127.0.0.1:8080/ws${NC}"
        echo ""
        echo -e "${YELLOW}Make sure server is running first!${NC}"
        echo ""
        
        cd clients/desktop
        cargo run --bin desktop-client --profile fastdev
        ;;
    
    "full")
        echo -e "${MAGENTA}🎯 Full Presentation Mode${NC}"
        echo -e "${MAGENTA}========================${NC}"
        echo ""
        
        run_step "1" "cargo build --profile fastdev" "Building project"
        run_step "2" "cd clients/desktop && cargo build --profile fastdev" "Building desktop client"
        
        echo -e "${YELLOW}🚀 Starting full presentation...${NC}"
        echo ""
        echo -e "${CYAN}📡 Server: ws://127.0.0.1:8080/ws${NC}"
        echo -e "${CYAN}🌐 Web UI: http://127.0.0.1:8080${NC}"
        echo ""
        
        # Start desktop client in new terminal
        echo -e "${YELLOW}🖥️ Starting desktop client...${NC}"
        if command -v gnome-terminal &> /dev/null; then
            gnome-terminal -- bash -c "cd clients/desktop && cargo run --bin desktop-client --profile fastdev; exec bash"
        elif command -v xterm &> /dev/null; then
            xterm -e "cd clients/desktop && cargo run --bin desktop-client --profile fastdev"
        else
            echo -e "${YELLOW}Could not detect terminal emulator. Starting desktop client in background...${NC}"
            cd clients/desktop && cargo run --bin desktop-client --profile fastdev &
        fi
        
        # Start Python clients if available
        if [ -f "clients/python/trading_client.py" ]; then
            echo -e "${YELLOW}🐍 Starting Python trading client...${NC}"
            if command -v gnome-terminal &> /dev/null; then
                gnome-terminal -- bash -c "cd clients/python && python trading_client.py; exec bash"
            elif command -v xterm &> /dev/null; then
                xterm -e "cd clients/python && python trading_client.py"
            else
                cd clients/python && python trading_client.py &
            fi
        fi
        
        if [ -f "clients/python/trading_bot.py" ]; then
            echo -e "${YELLOW}🤖 Starting Python trading bot...${NC}"
            if command -v gnome-terminal &> /dev/null; then
                gnome-terminal -- bash -c "cd clients/python && python trading_bot.py; exec bash"
            elif command -v xterm &> /dev/null; then
                xterm -e "cd clients/python && python trading_bot.py"
            else
                cd clients/python && python trading_bot.py &
            fi
        fi
        
        echo ""
        echo -e "${GREEN}🎯 Full presentation started!${NC}"
        echo -e "${GRAY}📊 Multiple clients running in separate terminals${NC}"
        echo -e "${GRAY}🔄 Server will handle all connections${NC}"
        echo ""
        echo -e "${YELLOW}Press Ctrl+C to stop server (clients will continue)${NC}"
        echo ""
        
        cargo run --bin demo-server
        ;;
esac

echo ""
echo -e "${CYAN}👋 Presentation ended!${NC}"
echo -e "${GRAY}📝 Check logs for any errors or issues${NC}"
