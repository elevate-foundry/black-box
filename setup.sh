#!/usr/bin/env bash
#
# SAL (Black Box) - Cross-Platform Setup Script
# Supports: macOS, Linux (Debian/Ubuntu, Fedora/RHEL, Arch)
#
# Usage: curl -fsSL https://raw.githubusercontent.com/elevate-foundry/black-box/main/setup.sh | bash
#    or: ./setup.sh
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_step() {
    echo -e "${BLUE}==>${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Darwin*)
            OS="macos"
            ;;
        Linux*)
            if [ -f /etc/debian_version ]; then
                OS="debian"
            elif [ -f /etc/fedora-release ]; then
                OS="fedora"
            elif [ -f /etc/arch-release ]; then
                OS="arch"
            elif [ -f /etc/redhat-release ]; then
                OS="rhel"
            else
                OS="linux-unknown"
            fi
            ;;
        *)
            print_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
    print_success "Detected OS: $OS"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Install system dependencies based on OS
install_system_deps() {
    print_step "Installing system dependencies..."
    
    case "$OS" in
        macos)
            if ! command_exists brew; then
                print_step "Installing Homebrew..."
                /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
            fi
            # Tauri dependencies for macOS
            brew install --quiet pkg-config 2>/dev/null || true
            ;;
        debian)
            print_step "Installing Tauri dependencies (requires sudo)..."
            sudo apt-get update -qq
            sudo apt-get install -y -qq \
                libwebkit2gtk-4.1-dev \
                build-essential \
                curl \
                wget \
                file \
                libxdo-dev \
                libssl-dev \
                libayatana-appindicator3-dev \
                librsvg2-dev \
                libgtk-3-dev \
                libsoup-3.0-dev \
                libjavascriptcoregtk-4.1-dev
            ;;
        fedora|rhel)
            print_step "Installing Tauri dependencies (requires sudo)..."
            sudo dnf install -y \
                webkit2gtk4.1-devel \
                openssl-devel \
                curl \
                wget \
                file \
                libappindicator-gtk3-devel \
                librsvg2-devel \
                gtk3-devel \
                libsoup3-devel \
                javascriptcoregtk4.1-devel \
                @development-tools
            ;;
        arch)
            print_step "Installing Tauri dependencies (requires sudo)..."
            sudo pacman -Syu --noconfirm --needed \
                webkit2gtk-4.1 \
                base-devel \
                curl \
                wget \
                file \
                openssl \
                appmenu-gtk-module \
                gtk3 \
                libappindicator-gtk3 \
                librsvg \
                libsoup3
            ;;
        *)
            print_warning "Unknown Linux distribution. Please install Tauri dependencies manually."
            print_warning "See: https://tauri.app/start/prerequisites/"
            ;;
    esac
    
    print_success "System dependencies installed"
}

# Install Rust
install_rust() {
    if command_exists rustc; then
        RUST_VERSION=$(rustc --version | cut -d' ' -f2)
        print_success "Rust already installed: $RUST_VERSION"
        
        # Check if version is recent enough (1.70+)
        MAJOR=$(echo "$RUST_VERSION" | cut -d'.' -f1)
        MINOR=$(echo "$RUST_VERSION" | cut -d'.' -f2)
        if [ "$MAJOR" -lt 1 ] || ([ "$MAJOR" -eq 1 ] && [ "$MINOR" -lt 70 ]); then
            print_step "Updating Rust to latest version..."
            rustup update stable
        fi
    else
        print_step "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        print_success "Rust installed: $(rustc --version)"
    fi
}

# Install Node.js
install_node() {
    if command_exists node; then
        NODE_VERSION=$(node --version)
        print_success "Node.js already installed: $NODE_VERSION"
        
        # Check if version is 18+
        MAJOR=$(echo "$NODE_VERSION" | sed 's/v//' | cut -d'.' -f1)
        if [ "$MAJOR" -lt 18 ]; then
            print_warning "Node.js version $NODE_VERSION is old. Recommend upgrading to 18+."
        fi
    else
        print_step "Installing Node.js..."
        case "$OS" in
            macos)
                brew install node
                ;;
            debian)
                # Install via NodeSource
                curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
                sudo apt-get install -y nodejs
                ;;
            fedora|rhel)
                sudo dnf install -y nodejs npm
                ;;
            arch)
                sudo pacman -S --noconfirm nodejs npm
                ;;
            *)
                print_error "Please install Node.js manually: https://nodejs.org/"
                exit 1
                ;;
        esac
        print_success "Node.js installed: $(node --version)"
    fi
}

# Install Tauri CLI
install_tauri_cli() {
    if command_exists cargo-tauri || cargo tauri --version >/dev/null 2>&1; then
        print_success "Tauri CLI already installed"
    else
        print_step "Installing Tauri CLI..."
        cargo install tauri-cli
        print_success "Tauri CLI installed"
    fi
}

# Install Ollama (optional, for LLM features)
install_ollama() {
    if command_exists ollama; then
        print_success "Ollama already installed"
    else
        print_step "Installing Ollama (for local LLM)..."
        case "$OS" in
            macos)
                brew install ollama
                ;;
            *)
                # Linux install script
                curl -fsSL https://ollama.com/install.sh | sh
                ;;
        esac
        print_success "Ollama installed"
        print_step "Pulling default model (llama3.2:1b)..."
        ollama pull llama3.2:1b || print_warning "Could not pull model. Run 'ollama pull llama3.2:1b' later."
    fi
}

# Install npm dependencies
install_npm_deps() {
    print_step "Installing npm dependencies..."
    npm install
    print_success "npm dependencies installed"
}

# Main setup flow
main() {
    echo ""
    echo -e "${BLUE}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}          ${GREEN}SAL - The Black Box${NC} Setup Script              ${BLUE}║${NC}"
    echo -e "${BLUE}║${NC}     Privacy-first AI that runs entirely offline         ${BLUE}║${NC}"
    echo -e "${BLUE}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""
    
    detect_os
    install_system_deps
    install_rust
    install_node
    install_tauri_cli
    install_ollama
    
    # Check if we're in the project directory
    if [ -f "package.json" ] && [ -f "src-tauri/Cargo.toml" ]; then
        install_npm_deps
    else
        print_warning "Not in project directory. Run 'npm install' after cloning the repo."
    fi
    
    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║${NC}                    Setup Complete!                        ${GREEN}║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Clone the repo (if not already): git clone https://github.com/elevate-foundry/black-box.git"
    echo "  2. cd black-box"
    echo "  3. ./setup.sh  (if you haven't run it from the project dir)"
    echo "  4. cargo tauri dev"
    echo ""
    echo "For production build: cargo tauri build"
    echo ""
}

main "$@"
