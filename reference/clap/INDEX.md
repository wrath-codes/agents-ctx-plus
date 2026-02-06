# clap — Sub-Index

> Rust CLI argument parser with derive macros and builder API (16 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|

### [getting-started](getting-started/)

|file|description|
|---|---|
|[installation.md](getting-started/installation.md)|Installation — Cargo.toml setup|
|[first-cli.md](getting-started/first-cli.md)|First CLI — hello world walkthrough|
|[project-setup.md](getting-started/project-setup.md)|Project setup — structure and organization|

### [core-concepts](core-concepts/)

|file|description|
|---|---|
|[arguments.md](core-concepts/arguments.md)|Arguments — positional, optional, required|
|[options-and-flags.md](core-concepts/options-and-flags.md)|Options/flags — -s, --long, bool flags|
|[subcommands.md](core-concepts/subcommands.md)|Subcommands — enum-based command dispatch|

### [derive-macro](derive-macro/)

|file|description|
|---|---|
|[overview.md](derive-macro/overview.md)|Derive — #[derive(Parser)], #[command], #[arg]|
|[attributes.md](derive-macro/attributes.md)|Attributes — all derive macro attributes|

### [builder-api](builder-api/)

|file|description|
|---|---|
|[overview.md](builder-api/overview.md)|Builder — Command::new(), Arg::new() programmatic API|

### [validation](validation/)

|file|description|
|---|---|
|[value-validation.md](validation/value-validation.md)|Validation — value_parser, custom validators|

### [testing](testing/)

|file|description|
|---|---|
|[unit-testing.md](testing/unit-testing.md)|Testing — try_parse_from, assert_cmd|

### [examples](examples/)

|file|description|
|---|---|
|[basic.md](examples/basic.md)|Examples — complete working examples|

### [appendix](appendix/)

|file|description|
|---|---|
|[cargo-integrations.md](appendix/cargo-integrations.md)|Cargo — version from Cargo.toml, completions|
|[troubleshooting.md](appendix/troubleshooting.md)|Troubleshooting — common issues|

### Key Patterns
```rust
#[derive(Parser)]
#[command(name = "app", about = "description")]
struct Cli {
    #[arg(short, long, default_value_t = 1)]
    count: u8,
    #[command(subcommand)]
    command: Commands,
}
```

---
*16 files*
