#
# SAL (Black Box) - Windows Setup Script (PowerShell)
# 
# Usage: 
#   1. Open PowerShell as Administrator
#   2. Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
#   3. .\setup.ps1
#

$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "==> " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "✓ " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warning {
    param([string]$Message)
    Write-Host "⚠ " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error {
    param([string]$Message)
    Write-Host "✗ " -ForegroundColor Red -NoNewline
    Write-Host $Message
}

function Test-Command {
    param([string]$Command)
    $null = Get-Command $Command -ErrorAction SilentlyContinue
    return $?
}

function Install-Winget {
    if (-not (Test-Command "winget")) {
        Write-Step "Installing winget (Windows Package Manager)..."
        # winget is included in Windows 11 and recent Windows 10 builds
        # For older systems, install App Installer from Microsoft Store
        Write-Warning "winget not found. Please install 'App Installer' from the Microsoft Store."
        Write-Warning "Or download from: https://aka.ms/getwinget"
        Start-Process "ms-windows-store://pdp/?ProductId=9NBLGGH4NNS1"
        Read-Host "Press Enter after installing winget..."
    }
    Write-Success "winget available"
}

function Install-VisualStudioBuildTools {
    Write-Step "Checking Visual Studio Build Tools..."
    
    # Check if VS Build Tools or Visual Studio is installed
    $vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vsWhere) {
        $vsInstalls = & $vsWhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($vsInstalls) {
            Write-Success "Visual Studio Build Tools already installed"
            return
        }
    }
    
    Write-Step "Installing Visual Studio Build Tools (this may take a while)..."
    winget install Microsoft.VisualStudio.2022.BuildTools --silent --accept-package-agreements --accept-source-agreements
    
    # Install required workloads
    $vsInstaller = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vs_installer.exe"
    if (Test-Path $vsInstaller) {
        Write-Step "Installing C++ build tools workload..."
        & $vsInstaller modify --installPath "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools" `
            --add Microsoft.VisualStudio.Workload.VCTools `
            --add Microsoft.VisualStudio.Component.Windows11SDK.22621 `
            --quiet --norestart
    }
    
    Write-Success "Visual Studio Build Tools installed"
}

function Install-WebView2 {
    Write-Step "Checking WebView2 Runtime..."
    
    $webview2Key = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
    if (Test-Path $webview2Key) {
        Write-Success "WebView2 Runtime already installed"
        return
    }
    
    Write-Step "Installing WebView2 Runtime..."
    winget install Microsoft.EdgeWebView2Runtime --silent --accept-package-agreements --accept-source-agreements
    Write-Success "WebView2 Runtime installed"
}

function Install-Rust {
    if (Test-Command "rustc") {
        $rustVersion = rustc --version
        Write-Success "Rust already installed: $rustVersion"
        
        # Update if needed
        Write-Step "Updating Rust..."
        rustup update stable 2>$null
    } else {
        Write-Step "Installing Rust..."
        
        # Download and run rustup-init
        $rustupInit = "$env:TEMP\rustup-init.exe"
        Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit
        & $rustupInit -y --default-toolchain stable
        
        # Add to PATH for current session
        $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
        
        Write-Success "Rust installed: $(rustc --version)"
    }
}

function Install-Node {
    if (Test-Command "node") {
        $nodeVersion = node --version
        Write-Success "Node.js already installed: $nodeVersion"
        
        # Check version
        $major = [int]($nodeVersion -replace 'v(\d+)\..*', '$1')
        if ($major -lt 18) {
            Write-Warning "Node.js version is old. Recommend upgrading to 18+."
        }
    } else {
        Write-Step "Installing Node.js..."
        winget install OpenJS.NodeJS.LTS --silent --accept-package-agreements --accept-source-agreements
        
        # Refresh PATH
        $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "User")
        
        Write-Success "Node.js installed: $(node --version)"
    }
}

function Install-TauriCLI {
    if (Test-Command "cargo-tauri") {
        Write-Success "Tauri CLI already installed"
    } else {
        # Check if cargo tauri works
        $tauriCheck = cargo tauri --version 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Success "Tauri CLI already installed"
        } else {
            Write-Step "Installing Tauri CLI..."
            cargo install tauri-cli
            Write-Success "Tauri CLI installed"
        }
    }
}

function Install-Ollama {
    if (Test-Command "ollama") {
        Write-Success "Ollama already installed"
    } else {
        Write-Step "Installing Ollama (for local LLM)..."
        winget install Ollama.Ollama --silent --accept-package-agreements --accept-source-agreements
        
        # Refresh PATH
        $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "User")
        
        Write-Success "Ollama installed"
        
        Write-Step "Pulling default model (llama3.2:1b)..."
        try {
            ollama pull llama3.2:1b
        } catch {
            Write-Warning "Could not pull model. Run 'ollama pull llama3.2:1b' later."
        }
    }
}

function Install-NpmDeps {
    Write-Step "Installing npm dependencies..."
    npm install
    Write-Success "npm dependencies installed"
}

# Main
function Main {
    Write-Host ""
    Write-Host "╔═══════════════════════════════════════════════════════════╗" -ForegroundColor Blue
    Write-Host "║          SAL - The Black Box Setup Script                 ║" -ForegroundColor Blue
    Write-Host "║     Privacy-first AI that runs entirely offline           ║" -ForegroundColor Blue
    Write-Host "╚═══════════════════════════════════════════════════════════╝" -ForegroundColor Blue
    Write-Host ""
    
    Write-Success "Detected OS: Windows"
    
    Install-Winget
    Install-VisualStudioBuildTools
    Install-WebView2
    Install-Rust
    Install-Node
    Install-TauriCLI
    Install-Ollama
    
    # Check if we're in the project directory
    if ((Test-Path "package.json") -and (Test-Path "src-tauri\Cargo.toml")) {
        Install-NpmDeps
    } else {
        Write-Warning "Not in project directory. Run 'npm install' after cloning the repo."
    }
    
    Write-Host ""
    Write-Host "╔═══════════════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║                    Setup Complete!                        ║" -ForegroundColor Green
    Write-Host "╚═══════════════════════════════════════════════════════════╝" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:"
    Write-Host "  1. Clone the repo (if not already): git clone https://github.com/elevate-foundry/black-box.git"
    Write-Host "  2. cd black-box"
    Write-Host "  3. .\setup.ps1  (if you haven't run it from the project dir)"
    Write-Host "  4. cargo tauri dev"
    Write-Host ""
    Write-Host "For production build: cargo tauri build"
    Write-Host ""
    
    # Note about restart
    Write-Warning "You may need to restart your terminal or computer for PATH changes to take effect."
}

Main
