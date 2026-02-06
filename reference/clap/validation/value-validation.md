# Value Validation

## Overview

clap provides multiple ways to validate user input, ensuring your application receives correct and safe data.

## Built-in Validation

### Range Validation

```rust
#[derive(Parser)]
struct Cli {
    /// Port number (1-65535)
    #[arg(value_parser = clap::value_parser!(u16).range(1..))]
    port: u16,
    
    /// Percentage (0-100)
    #[arg(value_parser = clap::value_parser!(u8).range(0..=100))]
    percentage: u8,
    
    /// Positive number only
    #[arg(value_parser = clap::value_parser!(i32).range(1..))]
    count: i32,
}
```

### Enum Validation

```rust
use clap::ValueEnum;

#[derive(ValueEnum, Clone)]
enum Color {
    Auto,
    Always,
    Never,
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long, value_enum)]
    color: Color,
}
```

### String Validation

```rust
#[derive(Parser)]
struct Cli {
    /// Must not be empty
    #[arg(value_parser = clap::builder::NonEmptyStringValueParser::new())]
    name: String,
    
    /// Must match regex pattern
    #[arg(value_parser = regex_validator(r"^\d{4}-\d{2}-\d{2}$"))]
    date: String,
}
```

## Custom Validators

### FromStr Implementation

```rust
use std::str::FromStr;
use std::net::IpAddr;

#[derive(Clone)]
struct Port(u16);

impl FromStr for Port {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let port = s.parse::<u16>()
            .map_err(|_| format!("'{}' is not a valid port", s))?;
        
        if port == 0 {
            return Err("Port cannot be 0".to_string());
        }
        
        if port < 1024 {
            return Err(format!(
                "Port {} is reserved (must be >= 1024)",
                port
            ));
        }
        
        Ok(Port(port))
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    port: Port,
}
```

### Custom Value Parser

```rust
fn validate_email(s: &str) -> Result<String, String> {
    if s.contains('@') && s.contains('.') {
        Ok(s.to_string())
    } else {
        Err(format!("'{}' is not a valid email address", s))
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long, value_parser = validate_email)]
    email: String,
}
```

### Path Validation

```rust
use std::path::PathBuf;

fn validate_path(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    
    if !path.exists() {
        return Err(format!("Path does not exist: {}", s));
    }
    
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", s));
    }
    
    Ok(path)
}

#[derive(Parser)]
struct Cli {
    #[arg(value_parser = validate_path)]
    input: PathBuf,
}
```

## Complex Validation

### Duration Parser

```rust
use std::time::Duration;

fn parse_duration(s: &str) -> Result<Duration, String> {
    let num: u64 = s.chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .map_err(|_| format!("Invalid number in duration: {}", s))?;
    
    let unit = s.chars()
        .skip_while(|c| c.is_ascii_digit())
        .collect::<String>();
    
    match unit.as_str() {
        "s" | "sec" | "secs" | "second" | "seconds" => {
            Ok(Duration::from_secs(num))
        }
        "m" | "min" | "mins" | "minute" | "minutes" => {
            Ok(Duration::from_secs(num * 60))
        }
        "h" | "hr" | "hrs" | "hour" | "hours" => {
            Ok(Duration::from_secs(num * 3600))
        }
        "d" | "day" | "days" => {
            Ok(Duration::from_secs(num * 86400))
        }
        _ => Err(format!("Unknown time unit: {}", unit)),
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long, value_parser = parse_duration)]
    timeout: Duration,
}

// Usage:
// myapp --timeout 30s
// myapp --timeout 5m
// myapp --timeout 1h
// myapp --timeout 2d
```

### Size Parser

```rust
fn parse_size(s: &str) -> Result<u64, String> {
    let num: f64 = s.chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .parse()
        .map_err(|_| format!("Invalid size: {}", s))?;
    
    let unit = s.chars()
        .skip_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>()
        .to_uppercase();
    
    let multiplier = match unit.as_str() {
        "B" | "" => 1,
        "KB" | "K" => 1024,
        "MB" | "M" => 1024 * 1024,
        "GB" | "G" => 1024 * 1024 * 1024,
        "TB" | "T" => 1024 * 1024 * 1024 * 1024,
        _ => return Err(format!("Unknown size unit: {}", unit)),
    };
    
    Ok((num * multiplier as f64) as u64)
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long, value_parser = parse_size)]
    max_size: u64,
}

// Usage:
// myapp --max-size 100MB
// myapp --max-size 2GB
// myapp --max-size 1.5TB
```

## Regex Validation

```rust
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref SEMVER: Regex = Regex::new(
        r"^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$$"
    ).unwrap();
}

fn validate_semver(s: &str) -> Result<String, String> {
    if SEMVER.is_match(s) {
        Ok(s.to_string())
    } else {
        Err(format!("'{}' is not a valid semantic version", s))
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(value_parser = validate_semver)]
    version: String,
}
```

## Multi-Field Validation

### Cross-Field Dependencies

```rust
#[derive(Parser)]
struct Cli {
    #[arg(long)]
    start_date: String,
    
    #[arg(long)]
    end_date: String,
}

fn validate_dates(cli: &Cli) -> Result<(), String> {
    // Parse dates
    let start = parse_date(&cli.start_date)?;
    let end = parse_date(&cli.end_date)?;
    
    if end < start {
        return Err("End date must be after start date".to_string());
    }
    
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    
    if let Err(e) = validate_dates(&cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    
    // Continue with valid dates
}
```

## Error Messages

### Custom Error Messages

```rust
fn validate_positive(s: &str) -> Result<u32, String> {
    match s.parse::<u32>() {
        Ok(n) if n > 0 => Ok(n),
        Ok(_) => Err("Value must be greater than 0".to_string()),
        Err(_) => Err(format!("'{}' is not a valid number", s)),
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long, value_parser = validate_positive)]
    count: u32,
}
```

### Contextual Errors

```rust
fn validate_file(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    
    if !path.exists() {
        return Err(format!(
            "File not found: {}\n\nPlease provide a valid file path.",
            s
        ));
    }
    
    let metadata = std::fs::metadata(&path)
        .map_err(|e| format!("Cannot access file: {} ({})", s, e))?;
    
    if !metadata.is_file() {
        return Err(format!("Not a file: {}", s));
    }
    
    if metadata.len() == 0 {
        return Err(format!("File is empty: {}", s));
    }
    
    Ok(path)
}
```

## Validation Chaining

```rust
fn validate_and_normalize_email(s: &str) -> Result<String, String> {
    // Check basic format
    if !s.contains('@') {
        return Err("Email must contain @".to_string());
    }
    
    // Check length
    if s.len() > 254 {
        return Err("Email too long".to_string());
    }
    
    // Normalize (lowercase)
    let normalized = s.to_lowercase();
    
    // Check against blacklist
    let blacklist = ["test@test.com", "admin@admin.com"];
    if blacklist.contains(&normalized.as_str()) {
        return Err("Email is blacklisted".to_string());
    }
    
    Ok(normalized)
}
```

## Best Practices

### 1. Validate Early

```rust
#[derive(Parser)]
struct Cli {
    // Validate at parse time, not in main()
    #[arg(value_parser = validate_port)]
    port: u16,
}
```

### 2. Provide Clear Errors

```rust
fn validate_port(s: &str) -> Result<u16, String> {
    match s.parse::<u16>() {
        Ok(0) => Err("Port cannot be 0".to_string()),
        Ok(p) if p < 1024 => Err(format!(
            "Port {} is reserved (use 1024-65535)",
            p
        )),
        Ok(p) => Ok(p),
        Err(_) => Err(format!(
            "'{}' is not a valid port number\n\nPorts must be numbers between 1 and 65535",
            s
        )),
    }
}
```

### 3. Use Type Safety

```rust
// Good: Type-safe wrapper
#[derive(Clone)]
struct Port(u16);

// Bad: Raw validation
#[arg(value_parser = validate_port)]
port: u16,
```

### 4. Document Valid Values

```rust
#[derive(Parser)]
struct Cli {
    /// Compression level (0-9)
    /// 
    /// 0 = no compression, 9 = maximum compression
    #[arg(short, long, value_parser = clap::value_parser!(u8).range(0..=9))]
    level: u8,
}
```

## Next Steps

- **[Custom Validators](custom-validators.md)** - Write complex validators
- **[Conflicts](../validation/conflicts.md)** - Handle argument conflicts