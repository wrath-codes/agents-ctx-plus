use zen_core::entities::CompatCheck;
use zen_core::enums::CompatStatus;

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    status: Option<&str>,
    package: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = effective_limit(limit, flags.limit, 20);
    let fetch_limit = compute_fetch_limit(limit, status, package);
    let mut rows: Vec<CompatCheck> = ctx.service.list_compat(fetch_limit).await?;

    if let Some(status) = status {
        let status = parse_enum::<CompatStatus>(status, "status")?;
        rows.retain(|row| row.status == status);
    }
    if let Some(package) = package {
        rows.retain(|row| row.package_a.contains(package) || row.package_b.contains(package));
    }
    rows.truncate(usize::try_from(limit)?);

    output(&rows, flags.format)
}

fn compute_fetch_limit(limit: u32, status: Option<&str>, package: Option<&str>) -> u32 {
    if status.is_some() || package.is_some() {
        limit.saturating_mul(5).min(500)
    } else {
        limit
    }
}
