use zen_core::enums::CompatStatus;
use zen_db::updates::compat::CompatUpdateBuilder;

use crate::cli::GlobalFlags;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    package_a: &str,
    package_b: &str,
    status: &str,
    conditions: Option<String>,
    finding: Option<String>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let status = parse_enum::<CompatStatus>(status, "status")?;

    if let Some(existing) = ctx
        .service
        .get_compat_by_packages(package_a, package_b)
        .await?
    {
        let update = CompatUpdateBuilder::new()
            .status(status)
            .conditions(conditions)
            .finding_id(finding)
            .build();

        let compat = ctx
            .service
            .update_compat(&session_id, &existing.id, update)
            .await?;
        output(&compat, flags.format)
    } else {
        let compat = ctx
            .service
            .create_compat(
                &session_id,
                package_a,
                package_b,
                status,
                conditions.as_deref(),
                finding.as_deref(),
            )
            .await?;
        output(&compat, flags.format)
    }
}
