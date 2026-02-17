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
    status: Option<&str>,
    conditions: Option<String>,
    finding: Option<String>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    if let Some(existing) = ctx
        .service
        .get_compat_by_packages(package_a, package_b)
        .await?
    {
        let status = resolve_status(status, Some(existing.status))?;

        let mut builder = CompatUpdateBuilder::new().status(status);
        if let Some(conditions) = conditions {
            builder = builder.conditions(Some(conditions));
        }
        if let Some(finding) = finding {
            builder = builder.finding_id(Some(finding));
        }

        let update = builder.build();

        let compat = ctx
            .service
            .update_compat(&session_id, &existing.id, update)
            .await?;
        output(&compat, flags.format)
    } else {
        let status = resolve_status(status, None)?;

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

fn resolve_status(
    input: Option<&str>,
    current: Option<CompatStatus>,
) -> anyhow::Result<CompatStatus> {
    match input {
        Some(value) => parse_enum::<CompatStatus>(value, "status"),
        None => Ok(current.unwrap_or(CompatStatus::Unknown)),
    }
}

#[cfg(test)]
mod tests {
    use zen_core::enums::CompatStatus;

    use super::resolve_status;

    #[test]
    fn keeps_current_status_when_omitted() {
        let status = resolve_status(None, Some(CompatStatus::Conditional)).expect("should resolve");
        assert_eq!(status, CompatStatus::Conditional);
    }

    #[test]
    fn defaults_to_unknown_when_creating_without_status() {
        let status = resolve_status(None, None).expect("should resolve");
        assert_eq!(status, CompatStatus::Unknown);
    }
}
