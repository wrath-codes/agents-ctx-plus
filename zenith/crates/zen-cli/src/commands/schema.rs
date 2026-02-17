use crate::cli::GlobalFlags;
use crate::cli::OutputFormat;
use crate::cli::root_commands::SchemaArgs;

/// Handle `znt schema`.
pub fn handle(args: &SchemaArgs, flags: &GlobalFlags) -> anyhow::Result<()> {
    let registry = zen_schema::SchemaRegistry::new();
    let schema = registry
        .get(&args.type_name)
        .ok_or_else(|| anyhow::anyhow!(unknown_type_message(&args.type_name, &registry.list())))?;

    match flags.format {
        OutputFormat::Raw => {
            println!("{}", serde_json::to_string(schema)?);
        }
        OutputFormat::Json | OutputFormat::Table => {
            println!("{}", serde_json::to_string_pretty(schema)?);
        }
    }

    Ok(())
}

fn unknown_type_message(type_name: &str, available: &[&str]) -> String {
    format!(
        "schema: unknown type '{}'. available: {}",
        type_name,
        available.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::unknown_type_message;

    #[test]
    fn unknown_type_message_includes_available_types() {
        let msg = unknown_type_message("nope", &["finding", "trail_operation"]);
        assert!(msg.contains("unknown type 'nope'"));
        assert!(msg.contains("finding, trail_operation"));
    }
}
