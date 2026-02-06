# Testing

## Overview

Testing CLI applications requires special techniques. This guide covers unit testing, integration testing, and best practices for testing clap-based applications.

## Unit Testing

### Testing Argument Parsing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    
    #[test]
    fn test_basic_parsing() {
        let cli = Cli::parse_from(["myapp", "Alice"]);
        assert_eq!(cli.name, "Alice");
    }
    
    #[test]
    fn test_with_options() {
        let cli = Cli::parse_from([
            "myapp",
            "--count", "5",
            "--verbose",
            "Alice"
        ]);
        assert_eq!(cli.name, "Alice");
        assert_eq!(cli.count, 5);
        assert!(cli.verbose);
    }
    
    #[test]
    fn test_short_options() {
        let cli = Cli::parse_from([
            "myapp",
            "-c", "3",
            "-v",
            "Bob"
        ]);
        assert_eq!(cli.count, 3);
        assert!(cli.verbose);
    }
}
```

### Testing Subcommands

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_add_command() {
        let cli = Cli::parse_from(["myapp", "add", "item1"]);
        
        match cli.command {
            Commands::Add { name, force } => {
                assert_eq!(name, "item1");
                assert!(!force);
            }
            _ => panic!("Expected Add command"),
        }
    }
    
    #[test]
    fn test_add_with_flag() {
        let cli = Cli::parse_from([
            "myapp", "add", "--force", "item1"
        ]);
        
        match cli.command {
            Commands::Add { name, force } => {
                assert_eq!(name, "item1");
                assert!(force);
            }
            _ => panic!("Expected Add command"),
        }
    }
}
```

### Testing Default Values

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_default_values() {
        let cli = Cli::parse_from(["myapp"]);
        assert_eq!(cli.count, 1); // default value
        assert!(!cli.verbose);    // default value
    }
}
```

### Testing Validation

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_valid_port() {
        let cli = Cli::parse_from(["myapp", "--port", "8080"]);
        assert_eq!(cli.port.0, 8080);
    }
    
    #[test]
    fn test_invalid_port_zero() {
        let result = Cli::try_parse_from(["myapp", "--port", "0"]);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_invalid_port_reserved() {
        let result = Cli::try_parse_from(["myapp", "--port", "80"]);
        assert!(result.is_err());
    }
}
```

## Integration Testing

### Using assert_cmd

```toml
[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
```

```rust
// tests/integration_test.rs
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Options:"));
}

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("myapp 0.1.0"));
}

#[test]
fn test_basic_command() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    
    cmd.args(["Alice"]);
    cmd.assert()
        .success()
        .stdout("Hello, Alice!\n");
}

#[test]
fn test_with_options() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    
    cmd.args(["--count", "3", "Alice"]);
    cmd.assert()
        .success()
        .stdout("Hello, Alice!\nHello, Alice!\nHello, Alice!\n");
}

#[test]
fn test_missing_required() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    
    // Missing required argument
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_invalid_option() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    
    cmd.args(["--invalid-option", "Alice"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}
```

### Testing File Operations

```rust
#[test]
fn test_file_input() {
    let temp_dir = tempfile::tempdir().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    fs::write(&input_file, "Hello, World!").unwrap();
    
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    cmd.args(["--input", input_file.to_str().unwrap()]);
    
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Hello, World!"));
}

#[test]
fn test_file_output() {
    let temp_dir = tempfile::tempdir().unwrap();
    let output_file = temp_dir.path().join("output.txt");
    
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    cmd.args([
        "--output", output_file.to_str().unwrap(),
        "Alice"
    ]);
    
    cmd.assert().success();
    
    let output = fs::read_to_string(&output_file).unwrap();
    assert_eq!(output, "Hello, Alice!\n");
}
```

### Testing Subcommands

```rust
#[test]
fn test_add_subcommand() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    
    cmd.args(["add", "new-item"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Adding: new-item"));
}

#[test]
fn test_remove_subcommand() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    
    cmd.args(["remove", "old-item"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Removing: old-item"));
}

#[test]
fn test_list_subcommand() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    
    cmd.arg("list");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Listing"));
}
```

## Snapshot Testing

### Using insta

```toml
[dev-dependencies]
insta = "1.0"
```

```rust
#[test]
fn test_help_snapshot() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    cmd.arg("--help");
    
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_error_snapshot() {
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    // Missing required arg
    
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    
    insta::assert_snapshot!(stderr);
}
```

## Testing Error Handling

### Testing Custom Errors

```rust
#[test]
fn test_custom_validation_error() {
    let result = Cli::try_parse_from(["myapp", "--port", "0"]);
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Port cannot be 0"));
}

#[test]
fn test_argument_conflicts() {
    let result = Cli::try_parse_from([
        "myapp",
        "--verbose",
        "--quiet",
        "file.txt"
    ]);
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("conflict"));
}
```

## Testing Environment Variables

```rust
#[test]
fn test_env_var() {
    use std::env;
    
    // Set environment variable
    env::set_var("MYAPP_API_KEY", "secret123");
    
    let cli = Cli::parse_from(["myapp"]);
    assert_eq!(cli.api_key, "secret123");
    
    // Clean up
    env::remove_var("MYAPP_API_KEY");
}

#[test]
fn test_env_var_override() {
    use std::env;
    
    env::set_var("MYAPP_API_KEY", "from_env");
    
    // Command line takes precedence
    let cli = Cli::parse_from([
        "myapp",
        "--api-key", "from_cli"
    ]);
    assert_eq!(cli.api_key, "from_cli");
    
    env::remove_var("MYAPP_API_KEY");
}
```

## Best Practices

### 1. Test Happy Path and Error Cases

```rust
#[test]
fn test_success() {
    // Test successful execution
}

#[test]
fn test_invalid_input() {
    // Test error handling
}

#[test]
fn test_missing_required() {
    // Test required argument errors
}
```

### 2. Use Temporary Files

```rust
use tempfile::tempdir;

#[test]
fn test_file_operations() {
    let temp_dir = tempdir().unwrap();
    // Use temp_dir for files
    // Automatically cleaned up
}
```

### 3. Test Help and Version

```rust
#[test]
fn test_help_and_version() {
    // Always test these
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    cmd.arg("--help");
    cmd.assert().success();
    
    let mut cmd = Command::cargo_bin("myapp").unwrap();
    cmd.arg("--version");
    cmd.assert().success();
}
```

### 4. Test Edge Cases

```rust
#[test]
fn test_empty_string() {
    let cli = Cli::parse_from(["myapp", ""]);
    assert_eq!(cli.name, "");
}

#[test]
fn test_special_characters() {
    let cli = Cli::parse_from(["myapp", "Hello\nWorld"]);
    assert_eq!(cli.name, "Hello\nWorld");
}

#[test]
fn test_unicode() {
    let cli = Cli::parse_from(["myapp", "Hello 🦀!"]);
    assert_eq!(cli.name, "Hello 🦀!");
}
```

### 5. Use Descriptive Test Names

```rust
// Good
#[test]
fn test_add_command_creates_new_item() {
    // ...
}

// Bad
#[test]
fn test1() {
    // ...
}
```

## Next Steps

- **[Integration Testing](integration-testing.md)** - More integration test patterns
- **[Testing Techniques](testing-techniques.md)** - Advanced testing strategies