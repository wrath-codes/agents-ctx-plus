use serde::Serialize;

use crate::cli::GlobalFlags;
use crate::output::output;

#[derive(Serialize)]
struct AuthLogoutResponse {
    cleared: bool,
}

pub async fn handle(flags: &GlobalFlags) -> anyhow::Result<()> {
    zen_auth::logout()?;
    output(&AuthLogoutResponse { cleared: true }, flags.format)
}
