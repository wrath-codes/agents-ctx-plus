# Installation

## Requirements

- **Rust**: 1.70.0 or later
- **Cargo**: Comes with Rust

## Adding to Your Project

### Basic Installation

```toml
[dependencies]
clap = "4.5"
```

### With Derive Macro (Recommended)

```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
```

### Full Featured

```toml
[dependencies]
clap = { version = "4.5", features = [
    "derive",
    "cargo",
    "env",
    "unicode",
    "wrap_help",
] }
```

## Feature Flags

| Feature | Description | Size Impact |
|---------|-------------|-------------|
| `derive` | Enable derive macros | ~20KB |
| `cargo` | Cargo.toml integration | ~1KB |
| `env` | Environment variable support | ~5KB |
| `unicode` | Unicode string support | ~10KB |
| `wrap_help` | Help text wrapping | ~15KB |
| `debug` | Debug assertions | Dev only |
| `unstable-doc` | Unstable features docs | Dev only |

## Version Compatibility

| clap Version | MSRV | Edition | Status |
|--------------|------|---------|--------|
| 4.x | 1.70 | 2021 | Current |
| 3.x | 1.54 | 2018 | Maintenance |
| 2.x | 1.21 | 2015 | Deprecated |

## Cargo.toml Examples

### Simple CLI Tool

```toml
[package]
name = "my-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4.5", features = ["derive"] }

[[bin]]
name = "my-cli"
path = "src/main.rs"
```

### Library with CLI Example

```toml
[package]
name = "my-lib"
version = "0.1.0"
edition = "2021"

[dependencies]
# Library dependencies
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
clap = { version = "4.5", features = ["derive"] }

[[example]]
name = "cli"
required-features = ["cli"]

[features]
cli = ["clap"]
```

### Multi-Binary Project

```toml
[package]
name = "my-tools"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4.5", features = ["derive"] }

[[bin]]
name = "tool-a"
path = "src/bin/tool-a.rs"

[[bin]]
name = "tool-b"
path = "src/bin/tool-b.rs"
```

## Verifying Installation

### Check Version

```bash
cargo tree -p clap
```

### Build Test

```bash
cargo new clap-test
cd clap-test
cargo add clap --features derive
```

Create `src/main.rs`:

```rust
use clap::Parser;

#[derive(Parser)]
struct Args {
    name: String,
}

fn main() {
    let args = Args::parse();
    println!("Hello, {}!", args.name);
}
```

Test:

```bash
cargo run -- Alice
```

## Updating clap

### Check for Updates

```bash
cargo outdated -p clap
```

### Update to Latest

```bash
cargo update -p clap
```

### Migration Between Versions

See [Migration Guide](../appendix/migration-guide.md) for detailed instructions.

## Development Setup

### Enable Debug Features

```toml
[dependencies]
clap = { version = "4.5", features = ["derive", "debug"] }
```

### Use Local clap (for contributing)

```toml
[dependencies]
clap = { path = "../clap/clap" }
```

### Patch for Testing

```toml
[patch.crates-io]
clap = { git = "https://github.com/clap-rs/clap", branch = "master" }
```

## Troubleshooting

### Compilation Errors

**Error: `cannot find derive macro Parser`**
```toml
# Add derive feature
clap = { version = "4.5", features = ["derive"] }
```

**Error: `feature XXX is unstable`**
```bash
# Update to latest stable Rust
rustup update
```

**Error: `minimum supported Rust version is 1.70.0`**
```bash
# Update Rust
rustup update
```

### Dependency Conflicts

**Multiple versions of clap:**
```toml
[patch.crates-io]
clap = { version = "4.5" }
```

### Slow Compilation

**Enable release optimizations:**
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

**Or use thinner LTO:**
```toml
[profile.release]
lto = "thin"
```

## Next Steps

- **[First CLI](first-cli.md)** - Build your first CLI application
- **[Project Setup](project-setup.md)** - Best practices for project structure