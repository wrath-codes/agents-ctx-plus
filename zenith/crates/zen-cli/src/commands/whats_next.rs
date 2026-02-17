use anyhow::bail;

use crate::cli::GlobalFlags;
use crate::context::AppContext;

/// Handle `znt whats-next`.
pub async fn handle(_ctx: &mut AppContext, _flags: &GlobalFlags) -> anyhow::Result<()> {
    bail!("znt whats-next is not implemented yet")
}
