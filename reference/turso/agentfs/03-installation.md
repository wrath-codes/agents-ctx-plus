# Installation

## System Requirements

### Supported Platforms
- **macOS**: 11.0 (Big Sur) or later (Intel & Apple Silicon)
- **Linux**: Ubuntu 20.04+, Debian 11+, Fedora 35+, CentOS 8+
- **Windows**: Windows 10/11 with WSL2 (native Windows support coming)

### Hardware Requirements
- **Minimum**: 2GB RAM, 1GB free disk space
- **Recommended**: 4GB RAM, 10GB free disk space
- **Network**: Internet connection for cloud sync features

## Installation Methods

### macOS

#### Homebrew (Recommended)
```bash
# Add Turso tap
brew tap turso/tap

# Install AgentFS
brew install agentfs

# Verify installation
agentfs --version
```

#### Manual Installation
```bash
# Download latest release
curl -sSfL https://github.com/tursodatabase/agentfs/releases/latest/download/agentfs-darwin-$(uname -m).tar.gz | tar xz

# Move to PATH
sudo mv agentfs /usr/local/bin/

# Verify
agentfs --version
```

### Linux

#### Installation Script
```bash
# Install using official script
curl -sSfL https://get.tur.so/install.sh | bash -s agentfs

# Add to PATH (if not done automatically)
export PATH="$HOME/.turso/bin:$PATH"

# Verify
agentfs --version
```

#### Package Managers

**Ubuntu/Debian:**
```bash
# Add Turso repository
curl -fsSL https://pkg.turso.tech/apt/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/turso-archive-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/turso-archive-keyring.gpg] https://pkg.turso.tech/apt stable main" | sudo tee /etc/apt/sources.list.d/turso.list

# Install
sudo apt update
sudo apt install agentfs
```

**Fedora/RHEL/CentOS:**
```bash
# Add Turso repository
sudo dnf config-manager --add-repo https://pkg.turso.tech/rpm/turso.repo

# Install
sudo dnf install agentfs
```

**Arch Linux:**
```bash
# From AUR
yay -S agentfs
# or
paru -S agentfs
```

#### Manual Installation
```bash
# Download for your architecture
ARCH=$(uname -m)
curl -sSfL https://github.com/tursodatabase/agentfs/releases/latest/download/agentfs-linux-${ARCH}.tar.gz | tar xz

# Move to PATH
sudo mv agentfs /usr/local/bin/

# Verify
agentfs --version
```

### Windows (WSL2)

Since native Windows support is not yet available, use WSL2:

```powershell
# In PowerShell (as Administrator)
wsl --install -d Ubuntu

# Then in WSL2 Ubuntu, follow Linux installation above
```

### Docker

```bash
# Pull AgentFS image
docker pull turso/agentfs:latest

# Run AgentFS in container
docker run -it --rm \
  -v $(pwd):/workspace \
  turso/agentfs:latest \
  agentfs workspace create test --base /workspace
```

### Building from Source

#### Prerequisites
- Rust 1.70+ (install via rustup)
- SQLite 3.40+
- pkg-config
- OpenSSL development libraries

#### Build Steps
```bash
# Clone repository
git clone https://github.com/tursodatabase/agentfs.git
cd agentfs

# Build release binary
cargo build --release

# Install
sudo cp target/release/agentfs /usr/local/bin/

# Verify
agentfs --version
```

## Post-Installation Setup

### Initial Configuration
```bash
# Run setup wizard
agentfs init --setup

# Or initialize in current directory
agentfs init
```

### Configuration File
AgentFS creates a configuration directory:

```
~/.config/agentfs/
├── config.toml          # Global configuration
├── credentials/         # Stored credentials (encrypted)
│   └── turso.tokens
└── workspaces/          # Workspace metadata
    └── index.db
```

### Setting Up Cloud Integration
```bash
# Login to Turso (for cloud sync)
agentfs auth login

# Or use API token
agentfs auth login --token $TURSO_API_TOKEN

# Verify connection
agentfs auth status
```

### Shell Completion

**Bash:**
```bash
agentfs completions bash > ~/.local/share/bash-completion/completions/agentfs
```

**Zsh:**
```bash
agentfs completions zsh > ~/.zfunc/_agentfs
# Add to ~/.zshrc: fpath+=~/.zfunc
```

**Fish:**
```bash
agentfs completions fish > ~/.config/fish/completions/agentfs.fish
```

## Verification

### Check Installation
```bash
# Version check
agentfs --version
# Output: agentfs 0.1.0

# Help
agentfs --help

# List commands
agentfs --list-commands
```

### Test Basic Functionality
```bash
# Create test directory
mkdir -p ~/agentfs-test && cd ~/agentfs-test

# Initialize AgentFS
agentfs init

# Create workspace
agentfs workspace create test-workspace

# Run simple command
agentfs run --workspace test-workspace echo "Hello from AgentFS"

# Check status
agentfs status --workspace test-workspace

# Clean up
agentfs workspace delete test-workspace
cd .. && rm -rf ~/agentfs-test
```

## Troubleshooting

### Installation Issues

**"command not found"**
```bash
# Check if binary is in PATH
which agentfs

# If not found, add to PATH
export PATH="$HOME/.turso/bin:$PATH"
# Add to ~/.bashrc or ~/.zshrc for persistence
```

**Permission denied**
```bash
# Fix permissions
chmod +x /usr/local/bin/agentfs

# Or reinstall with sudo
sudo chmod +x /usr/local/bin/agentfs
```

**Missing dependencies (Linux)**
```bash
# Ubuntu/Debian
sudo apt-get install libsqlite3-dev libssl-dev pkg-config

# Fedora/RHEL
sudo dnf install sqlite-devel openssl-devel pkgconfig

# macOS (should be included)
brew install sqlite openssl
```

### Runtime Issues

**SQLite version too old**
```bash
# Check SQLite version
sqlite3 --version

# Upgrade if needed (see platform-specific instructions)
```

**Disk space issues**
```bash
# Check AgentFS storage usage
agentfs storage usage

# Clean up old workspaces
agentfs workspace list --older-than 30d
agentfs workspace delete --older-than 30d --yes

# Garbage collect
agentfs gc
```

**Slow performance**
```bash
# Check cache settings
agentfs config get cache-size

# Increase cache
agentfs config set --cache-size 500MB

# Check if using SSD (recommended)
df -T /path/to/agentfs
```

## Uninstallation

### macOS (Homebrew)
```bash
brew uninstall agentfs
brew untap turso/tap
```

### Linux
```bash
# Remove binary
sudo rm /usr/local/bin/agentfs

# Remove configuration
rm -rf ~/.config/agentfs

# Remove from package manager
# Ubuntu/Debian:
sudo apt remove agentfs

# Fedora/RHEL:
sudo dnf remove agentfs
```

### Clean Up All Data
```bash
# Remove all AgentFS data
rm -rf ~/.config/agentfs
rm -rf ~/.local/share/agentfs

# Remove from shell config
# Edit ~/.bashrc or ~/.zshrc and remove AgentFS-related lines
```

## Next Steps

- **CLI Reference**: [04-cli-reference.md](./04-cli-reference.md)
- **Configuration**: [05-configuration.md](./05-configuration.md)
- **Quick Start Guide**: Try `agentfs init --tutorial`