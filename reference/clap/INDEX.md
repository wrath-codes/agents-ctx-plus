# clap — Sub-Index

> Rust CLI argument parser with derive macros and builder API (15 files)

### Root

|file|description|
|---|---|
|[README.md](README.md)|Getting started guide|
| |↳ [Key Features](README.md#key-features) · [Quick Start](README.md#quick-start) · [Installation](README.md#installation) · [Documentation Map](README.md#documentation-map) · [Choosing Between Derive and Builder](README.md#choosing-between-derive-and-builder) · [Comparison with Alternatives](README.md#comparison-with-alternatives) · [Next Steps](README.md#next-steps) · [Resources](README.md#resources) · +1 more|

### [getting-started](getting-started/)

|file|description|
|---|---|
|[installation.md](getting-started/installation.md)|Installation — Cargo.toml setup|
| |↳ [Requirements](getting-started/installation.md#requirements) · [Adding to Your Project](getting-started/installation.md#adding-to-your-project) · [Feature Flags](getting-started/installation.md#feature-flags) · [Version Compatibility](getting-started/installation.md#version-compatibility) · [Cargo.toml Examples](getting-started/installation.md#cargotoml-examples) · [Verifying Installation](getting-started/installation.md#verifying-installation) · [Updating clap](getting-started/installation.md#updating-clap) · [Development Setup](getting-started/installation.md#development-setup) · +2 more|
|[first-cli.md](getting-started/first-cli.md)|First CLI — hello world walkthrough|
| |↳ [Creating the Project](getting-started/first-cli.md#creating-the-project) · [Basic CLI](getting-started/first-cli.md#basic-cli) · [Adding Help](getting-started/first-cli.md#adding-help) · [Adding Version](getting-started/first-cli.md#adding-version) · [Optional Arguments](getting-started/first-cli.md#optional-arguments) · [Multiple Values](getting-started/first-cli.md#multiple-values) · [Complete Example](getting-started/first-cli.md#complete-example) · [Next Steps](getting-started/first-cli.md#next-steps)|
|[project-setup.md](getting-started/project-setup.md)|Project setup — structure and organization|
| |↳ [Project Structure](getting-started/project-setup.md#project-structure) · [Cargo.toml Structure](getting-started/project-setup.md#cargotoml-structure) · [Module Organization](getting-started/project-setup.md#module-organization) · [Error Handling](getting-started/project-setup.md#error-handling) · [Testing](getting-started/project-setup.md#testing) · [Documentation](getting-started/project-setup.md#documentation) · [Build Scripts](getting-started/project-setup.md#build-scripts) · [Distribution](getting-started/project-setup.md#distribution) · +1 more|

### [core-concepts](core-concepts/)

|file|description|
|---|---|
|[arguments.md](core-concepts/arguments.md)|Arguments — positional, optional, required|
| |↳ [Positional Arguments](core-concepts/arguments.md#positional-arguments) · [Options](core-concepts/arguments.md#options) · [Flags](core-concepts/arguments.md#flags) · [Argument Types](core-concepts/arguments.md#argument-types) · [Required Arguments](core-concepts/arguments.md#required-arguments) · [Argument Relationships](core-concepts/arguments.md#argument-relationships) · [Documentation](core-concepts/arguments.md#documentation) · [Next Steps](core-concepts/arguments.md#next-steps)|
|[options-and-flags.md](core-concepts/options-and-flags.md)|Options/flags — -s, --long, bool flags|
| |↳ [Options](core-concepts/options-and-flags.md#options) · [Flags](core-concepts/options-and-flags.md#flags) · [Combining Options](core-concepts/options-and-flags.md#combining-options) · [Advanced Patterns](core-concepts/options-and-flags.md#advanced-patterns) · [Best Practices](core-concepts/options-and-flags.md#best-practices) · [Next Steps](core-concepts/options-and-flags.md#next-steps)|
|[subcommands.md](core-concepts/subcommands.md)|Subcommands — enum-based command dispatch|
| |↳ [Basic Subcommands](core-concepts/subcommands.md#basic-subcommands) · [Subcommand with Arguments](core-concepts/subcommands.md#subcommand-with-arguments) · [Nested Subcommands](core-concepts/subcommands.md#nested-subcommands) · [External Subcommand Pattern](core-concepts/subcommands.md#external-subcommand-pattern) · [Subcommand Aliases](core-concepts/subcommands.md#subcommand-aliases) · [Global Arguments](core-concepts/subcommands.md#global-arguments) · [Subcommand Groups](core-concepts/subcommands.md#subcommand-groups) · [Subcommand with Struct](core-concepts/subcommands.md#subcommand-with-struct) · +3 more|

### [derive-macro](derive-macro/)

|file|description|
|---|---|
|[overview.md](derive-macro/overview.md)|Derive — #[derive(Parser)], #[command], #[arg]|
| |↳ [Basic Usage](derive-macro/overview.md#basic-usage) · [Struct-Level Attributes](derive-macro/overview.md#struct-level-attributes) · [Field Attributes](derive-macro/overview.md#field-attributes) · [Supported Types](derive-macro/overview.md#supported-types) · [Custom Types](derive-macro/overview.md#custom-types) · [Subcommands](derive-macro/overview.md#subcommands) · [Advanced Patterns](derive-macro/overview.md#advanced-patterns) · [Best Practices](derive-macro/overview.md#best-practices) · +2 more|
|[attributes.md](derive-macro/attributes.md)|Attributes — all derive macro attributes|
| |↳ [Command Attributes (Struct-Level)](derive-macro/attributes.md#command-attributes-struct-level) · [Argument Attributes (Field-Level)](derive-macro/attributes.md#argument-attributes-field-level) · [Complete Attribute Reference](derive-macro/attributes.md#complete-attribute-reference) · [Examples](derive-macro/attributes.md#examples) · [Next Steps](derive-macro/attributes.md#next-steps)|

### [builder-api](builder-api/)

|file|description|
|---|---|
|[overview.md](builder-api/overview.md)|Builder — Command::new(), Arg::new() programmatic API|
| |↳ [Basic Usage](builder-api/overview.md#basic-usage) · [Command Construction](builder-api/overview.md#command-construction) · [Arguments](builder-api/overview.md#arguments) · [Argument Configuration](builder-api/overview.md#argument-configuration) · [Parsing](builder-api/overview.md#parsing) · [Comparison with Derive](builder-api/overview.md#comparison-with-derive) · [When to Use Builder](builder-api/overview.md#when-to-use-builder) · [Next Steps](builder-api/overview.md#next-steps)|

### [validation](validation/)

|file|description|
|---|---|
|[value-validation.md](validation/value-validation.md)|Validation — value_parser, custom validators|
| |↳ [Built-in Validation](validation/value-validation.md#built-in-validation) · [Custom Validators](validation/value-validation.md#custom-validators) · [Complex Validation](validation/value-validation.md#complex-validation) · [Regex Validation](validation/value-validation.md#regex-validation) · [Multi-Field Validation](validation/value-validation.md#multi-field-validation) · [Error Messages](validation/value-validation.md#error-messages) · [Validation Chaining](validation/value-validation.md#validation-chaining) · [Best Practices](validation/value-validation.md#best-practices) · +1 more|

### [testing](testing/)

|file|description|
|---|---|
|[unit-testing.md](testing/unit-testing.md)|Testing — try_parse_from, assert_cmd|
| |↳ [Unit Testing](testing/unit-testing.md#unit-testing) · [Integration Testing](testing/unit-testing.md#integration-testing) · [Snapshot Testing](testing/unit-testing.md#snapshot-testing) · [Testing Error Handling](testing/unit-testing.md#testing-error-handling) · [Testing Environment Variables](testing/unit-testing.md#testing-environment-variables) · [Best Practices](testing/unit-testing.md#best-practices) · [Next Steps](testing/unit-testing.md#next-steps)|

### [examples](examples/)

|file|description|
|---|---|
|[basic.md](examples/basic.md)|Examples — complete working examples|
| |↳ [Example 1: Hello World CLI](examples/basic.md#example-1-hello-world-cli) · [Example 2: File Processor](examples/basic.md#example-2-file-processor) · [Example 3: HTTP Client](examples/basic.md#example-3-http-client) · [Example 4: Configuration CLI](examples/basic.md#example-4-configuration-cli) · [Example 5: Counter CLI](examples/basic.md#example-5-counter-cli) · [Common Patterns](examples/basic.md#common-patterns) · [Next Steps](examples/basic.md#next-steps)|

### [appendix](appendix/)

|file|description|
|---|---|
|[cargo-integrations.md](appendix/cargo-integrations.md)|Cargo — version from Cargo.toml, completions|
| |↳ [Cargo.toml Integration](appendix/cargo-integrations.md#cargotoml-integration) · [Feature Flags with Cargo](appendix/cargo-integrations.md#feature-flags-with-cargo) · [Build Scripts](appendix/cargo-integrations.md#build-scripts) · [Distribution](appendix/cargo-integrations.md#distribution) · [CI/CD Integration](appendix/cargo-integrations.md#cicd-integration) · [Package Managers](appendix/cargo-integrations.md#package-managers) · [Best Practices](appendix/cargo-integrations.md#best-practices) · [Next Steps](appendix/cargo-integrations.md#next-steps)|
|[troubleshooting.md](appendix/troubleshooting.md)|Troubleshooting — common issues|
| |↳ [Common Issues and Solutions](appendix/troubleshooting.md#common-issues-and-solutions) · [Performance Issues](appendix/troubleshooting.md#performance-issues) · [Getting Help](appendix/troubleshooting.md#getting-help) · [Debugging Tips](appendix/troubleshooting.md#debugging-tips) · [FAQ](appendix/troubleshooting.md#faq)|

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
*15 files*
