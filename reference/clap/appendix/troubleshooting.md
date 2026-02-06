# Troubleshooting

## Common Issues and Solutions

### Compilation Errors

#### Error: "cannot find derive macro `Parser`"

**Cause**: Missing `derive` feature

**Solution**:
```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
```

#### Error: "no method named `parse` found for struct"

**Cause**: Forgot to import `Parser` trait

**Solution**:
```rust
use clap::Parser;  // Make sure this is imported

#[derive(Parser)]
struct Cli {
    // ...
}
```

#### Error: "the trait `Clone` is not implemented"

**Cause**: Custom types need to implement `Clone`

**Solution**:
```rust
#[derive(Clone)]  // Add this
struct MyType {
    // ...
}

#[derive(Parser)]
struct Cli {
    #[arg(value_parser = parse_my_type)]
    my_arg: MyType,
}
```

#### Error: "expected struct `String`, found struct `&str`"

**Cause**: Using wrong type in `parse_from`

**Solution**:
```rust
// Wrong
Cli::parse_from(["app", "arg"]);

// Correct
Cli::parse_from(["app", "arg"].iter().map(String::from));
// or
Cli::parse_from(&["app", "arg"]);
```

### Runtime Issues

#### Application panics on invalid input

**Cause**: Using `parse()` instead of error handling

**Solution**:
```rust
// Bad - will panic
let cli = Cli::parse();

// Good - graceful error handling
let cli = match Cli::try_parse() {
    Ok(cli) => cli,
    Err(e) => {
        eprintln!("{}", e);
        std::process::exit(1);
    }
};
```

#### Help text not showing

**Cause**: Help disabled or custom handling

**Solution**:
```rust
#[derive(Parser)]
#[command(disable_help_flag = false)]  // Make sure this is false (default)
struct Cli {
    // ...
}
```

#### Version not showing

**Cause**: Version not set

**Solution**:
```rust
#[derive(Parser)]
#[command(version)]  // Enable version flag
// or
#[command(version = "1.0.0")]  // Set specific version
struct Cli {
    // ...
}
```

### Logic Issues

#### Arguments not being parsed

**Cause**: Wrong argument order

**Solution**:
```rust
#[derive(Parser)]
struct Cli {
    // Positional arguments must come before options in the struct
    input: String,  // Positional
    
    #[arg(short, long)]
    output: Option<String>,  // Option
}

// Usage: myapp input.txt --output out.txt
```

#### Default values not working

**Cause**: Using wrong attribute

**Solution**:
```rust
// For String
#[arg(default_value = "default")]
name: String,

// For numbers
#[arg(default_value_t = 10)]
count: u32,

// For bool
#[arg(default_value_t = true)]
enabled: bool,
```

#### Optional arguments not optional

**Cause**: Missing `Option<T>` wrapper

**Solution**:
```rust
// Required
name: String,

// Optional
name: Option<String>,

// Optional with default
#[arg(default_value = "default")]
name: String,
```

### Validation Issues

#### Custom validator not working

**Cause**: Wrong return type or error type

**Solution**:
```rust
// Must return Result<T, String> or Result<T, &'static str>
fn validate_port(s: &str) -> Result<u16, String> {
    match s.parse::<u16>() {
        Ok(0) => Err("Port cannot be 0".to_string()),
        Ok(n) => Ok(n),
        Err(_) => Err(format!("Invalid port: {}", s)),
    }
}
```

#### Range validation not working

**Cause**: Using wrong type

**Solution**:
```rust
// Correct
#[arg(value_parser = clap::value_parser!(u8).range(1..=100))]
percentage: u8,

// Incorrect - ranges only work on integer types
#[arg(value_parser = clap::value_parser!(f64).range(0.0..=1.0))]  // Won't work
```

### Subcommand Issues

#### Subcommand not recognized

**Cause**: Wrong subcommand definition

**Solution**:
```rust
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,  // Must use this exact attribute
}

#[derive(Subcommand)]  // Must derive Subcommand, not Parser
enum Commands {
    Add { name: String },
    Remove { name: String },
}
```

#### External subcommands not working

**Cause**: Missing `external_subcommand` attribute

**Solution**:
```rust
#[derive(Subcommand)]
enum Commands {
    Known,
    
    #[command(external_subcommand)]
    External(Vec<String>),
}
```

## Performance Issues

### Slow compilation

**Solutions**:

1. **Use thinner LTO**:
```toml
[profile.release]
lto = "thin"
```

2. **Reduce codegen units**:
```toml
[profile.release]
codegen-units = 1
```

3. **Use debug build for development**:
```bash
cargo build  # Not --release
```

### Large binary size

**Solutions**:

1. **Strip symbols**:
```toml
[profile.release]
strip = true
```

2. **Use panic=abort**:
```toml
[profile.release]
panic = "abort"
```

3. **Minimize features**:
```toml
[dependencies]
clap = { version = "4.5", default-features = false, features = ["std", "derive"] }
```

## Getting Help

### Debug Mode

Enable debug features:
```toml
[dependencies]
clap = { version = "4.5", features = ["derive", "debug"] }
```

### Logging

Add logging to understand parsing:
```rust
use clap::Parser;

#[derive(Parser, Debug)]  // Add Debug derive
struct Cli {
    // ...
}

fn main() {
    let cli = Cli::parse();
    eprintln!("Parsed CLI: {:?}", cli);  // Debug output
}
```

### Community Resources

- **GitHub Issues**: https://github.com/clap-rs/clap/issues
- **Discord**: https://discord.gg/clap-rs
- **Discussions**: GitHub Discussions
- **Stack Overflow**: Tag with `clap-rs`

## Debugging Tips

### 1. Check Feature Flags

```bash
cargo tree -p clap -f "{p} {f}"
```

### 2. Expand Macros

```bash
cargo expand  # Requires cargo-expand
```

### 3. Test Individual Parts

```rust
#[test]
fn test_parsing() {
    let cli = Cli::parse_from(["app", "--option", "value"]);
    assert_eq!(cli.option, "value");
}
```

### 4. Check Generated Help

```bash
cargo run -- --help
```

### 5. Use Clippy

```bash
cargo clippy -- -W clippy::pedantic
```

## FAQ

**Q: How do I make an argument optional?**
A: Use `Option<T>`: `arg: Option<String>`

**Q: How do I set a default value?**
A: Use `#[arg(default_value = "x")]` for strings, `#[arg(default_value_t = 10)]` for numbers

**Q: How do I require at least one value?**
A: Use `#[arg(required = true)]` or `#[arg(num_args = 1..)]`

**Q: How do I validate input?**
A: Use `value_parser` with a custom function or built-in validators

**Q: How do I add subcommands?**
A: Use `#[command(subcommand)]` with an enum deriving `Subcommand`

**Q: How do I read environment variables?**
A: Use `#[arg(env = "VAR_NAME")]`

**Q: How do I generate shell completions?**
A: Use `clap_complete` crate in a build script

**Q: How do I make a flag that can be repeated?**
A: Use `action = clap::ArgAction::Count` with a numeric type

**Q: How do I hide an argument from help?**
A: Use `#[arg(hide = true)]`

**Q: How do I make arguments global to subcommands?**
A: Use `#[arg(global = true)]`