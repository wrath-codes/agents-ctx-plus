# Cargo Integrations

## Overview

clap integrates seamlessly with Cargo to provide automatic version detection and metadata.

## Cargo.toml Integration

### Automatic Version

```rust
use clap::Parser;

#[derive(Parser)]
#[command(version)]  // Automatically reads from Cargo.toml
struct Cli {
    // ...
}
```

Your `Cargo.toml`:
```toml
[package]
name = "myapp"
version = "1.2.3"
```

Generated `--version`:
```bash
$ myapp --version
myapp 1.2.3
```

### Author Information

```rust
#[derive(Parser)]
#[command(author)]  // Reads from Cargo.toml authors
struct Cli {
    // ...
}
```

```toml
[package]
authors = ["Your Name <you@example.com>"]
```

### About/Description

```rust
#[derive(Parser)]
#[command(about)]  // Reads from Cargo.toml description
struct Cli {
    // ...
}
```

```toml
[package]
description = "A CLI tool that does something useful"
```

## Feature Flags with Cargo

### Optional Features

```toml
[features]
default = []
advanced = ["serde", "chrono"]
logging = ["env_logger", "log"]

[dependencies]
clap = { version = "4.5", features = ["derive", "cargo"] }
serde = { version = "1.0", optional = true }
chrono = { version = "0.4", optional = true }
env_logger = { version = "0.11", optional = true }
log = { version = "0.4", optional = true }
```

```rust
#[cfg(feature = "logging")]
use log::info;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();
    
    #[cfg(feature = "logging")]
    if cli.verbose {
        env_logger::init();
    }
    
    #[cfg(feature = "logging")]
    info!("Starting application");
    
    println!("Hello!");
}
```

### Feature-Dependent Arguments

```rust
#[derive(Parser)]
struct Cli {
    name: String,
    
    #[cfg(feature = "advanced")]
    #[arg(long)]
    format: Option<String>,
}
```

## Build Scripts

### Generating Shell Completions

```rust
// build.rs
use std::io;

fn main() -> io::Result<()> {
    let outdir = match std::env::var_os("OUT_DIR") {
        Some(outdir) => outdir,
        None => return Ok(()),
    };
    
    // Generate completions only in release builds
    if std::env::var("PROFILE").unwrap() != "release" {
        return Ok(());
    }
    
    let mut cmd = clap::Command::new("myapp")
        .version("1.0")
        .about("A CLI tool");
    
    // Add your arguments here
    cmd = cmd.arg(
        clap::Arg::new("config")
            .short('c')
            .long("config")
    );
    
    let shells = [
        clap_complete::Shell::Bash,
        clap_complete::Shell::Zsh,
        clap_complete::Shell::Fish,
        clap_complete::Shell::PowerShell,
        clap_complete::Shell::Elvish,
    ];
    
    for shell in shells {
        clap_complete::generate_to(
            shell,
            &mut cmd.clone(),
            "myapp",
            &outdir,
        )?;
    }
    
    println!("cargo:warning=Completions generated in {}", outdir.display());
    
    Ok(())
}
```

```toml
[build-dependencies]
clap_complete = "4.5"
```

### Installing Completions

```bash
# bash
sudo cp target/release/build/*/out/myapp.bash /etc/bash_completion.d/myapp

# zsh
mkdir -p ~/.zsh/completions
cp target/release/build/*/out/_myapp ~/.zsh/completions/
echo "fpath+=(~/.zsh/completions)" >> ~/.zshrc

# fish
mkdir -p ~/.config/fish/completions
cp target/release/build/*/out/myapp.fish ~/.config/fish/completions/
```

## Distribution

### Cargo Install

```bash
# Install from crates.io
cargo install myapp

# Install from local path
cargo install --path .

# Install specific version
cargo install myapp --version 1.0.0

# Install with features
cargo install myapp --features advanced
```

### Binary Distribution

```toml
[package]
name = "myapp"
version = "1.0.0"
edition = "2021"
authors = ["Your Name <you@example.com>"]
description = "A CLI tool"
license = "MIT OR Apache-2.0"
repository = "https://github.com/username/myapp"
homepage = "https://myapp.dev"
documentation = "https://docs.myapp.dev"
keywords = ["cli", "tool", "utility"]
categories = ["command-line-utilities"]

[dependencies]
clap = { version = "4.5", features = ["derive", "cargo", "env"] }
```

### Cross-Compilation

```toml
# Cross.toml for cross tool
[target.x86_64-unknown-linux-musl]
image = "ghcr.io/cross-rs/x86_64-unknown-linux-musl:main"

[target.x86_64-pc-windows-gnu]
image = "ghcr.io/cross-rs/x86_64-pc-windows-gnu:main"
```

```bash
# Build for Linux
cross build --release --target x86_64-unknown-linux-musl

# Build for Windows
cross build --release --target x86_64-pc-windows-gnu

# Build for macOS (requires macOS)
cargo build --release --target x86_64-apple-darwin
```

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
            suffix: ''
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            suffix: '.exe'
          - os: macos-latest
            target: x86_64-apple-darwin
            suffix: ''
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}
      
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Package
        run: |
          cd target/${{ matrix.target }}/release
          tar czf ../../../myapp-${{ matrix.target }}.tar.gz myapp${{ matrix.suffix }}
      
      - name: Upload
        uses: actions/upload-release-asset@v1
        with:
          asset_path: ./myapp-${{ matrix.target }}.tar.gz
          asset_name: myapp-${{ matrix.target }}.tar.gz
          asset_content_type: application/gzip
```

## Package Managers

### Homebrew (macOS/Linux)

```ruby
# Formula/myapp.rb
class Myapp < Formula
  desc "A CLI tool that does something useful"
  homepage "https://myapp.dev"
  url "https://github.com/username/myapp/archive/v1.0.0.tar.gz"
  sha256 "..."
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
    
    # Install shell completions
    bash_completion.install "completions/myapp.bash" => "myapp"
    zsh_completion.install "completions/_myapp"
    fish_completion.install "completions/myapp.fish"
    
    # Install man page
    man1.install "myapp.1"
  end

  test do
    assert_match "myapp #{version}", shell_output("#{bin}/myapp --version")
  end
end
```

### Debian/Ubuntu (.deb)

```toml
# Cargo.toml
[package.metadata.deb]
maintainer = "Your Name <you@example.com>"
copyright = "2024, Your Name"
extended-description = """\
A CLI tool that does something useful.
Supports various features and options."""
section = "utils"
priority = "optional"
assets = [
    ["target/release/myapp", "usr/bin/", "755"],
    ["README.md", "usr/share/doc/myapp/README", "644"],
    ["LICENSE", "usr/share/doc/myapp/LICENSE", "644"],
    ["completions/myapp.bash", "usr/share/bash-completion/completions/myapp", "644"],
    ["completions/_myapp", "usr/share/zsh/vendor-completions/", "644"],
    ["completions/myapp.fish", "usr/share/fish/vendor_completions.d/", "644"],
    ["myapp.1", "usr/share/man/man1/", "644"],
]
```

```bash
# Install cargo-deb
cargo install cargo-deb

# Build .deb package
cargo deb

# Build for specific target
cargo deb --target x86_64-unknown-linux-musl
```

### Windows Installer

```toml
# wix configuration in Cargo.toml
[package.metadata.wix]
upgrade-guid = "YOUR-UPGRADE-GUID"
path-guid = "YOUR-PATH-GUID"
license = false
eula = false
```

```bash
# Install cargo-wix
cargo install cargo-wix

# Generate installer
cargo wix
```

## Best Practices

### 1. Version Management

```rust
#[derive(Parser)]
#[command(version)]  // Always use Cargo.toml version
struct Cli {
    // ...
}
```

### 2. Feature Organization

```toml
[features]
default = []
# Core features
logging = ["tracing", "tracing-subscriber"]
async = ["tokio", "async-trait"]
# Advanced features
full = ["logging", "async", "advanced-validation"]
```

### 3. Documentation

```toml
[package]
description = "Short description for crates.io"
documentation = "https://docs.rs/myapp"
readme = "README.md"
keywords = ["cli", "tool"]
categories = ["command-line-utilities"]
```

### 4. Binary Size Optimization

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

## Next Steps

- **[Migration Guide](migration-guide.md)** - Upgrade from older versions
- **[Troubleshooting](troubleshooting.md)** - Common issues and solutions