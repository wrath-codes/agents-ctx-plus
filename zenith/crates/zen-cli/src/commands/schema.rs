use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::cli::root_commands::SchemaArgs;

/// Handle `znt schema`.
pub fn handle(args: &SchemaArgs, _flags: &GlobalFlags) -> anyhow::Result<()> {
    bail!(
        "znt schema is not implemented yet (type={})",
        args.type_name
    )
}
