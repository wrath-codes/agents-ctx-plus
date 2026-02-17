use zen_core::enums::{IssueStatus, IssueType};
use zen_db::updates::issue::IssueUpdateBuilder;

use crate::cli::GlobalFlags;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub struct Params {
    pub id: String,
    pub title: Option<String>,
    pub issue_type: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<u8>,
}

pub async fn run(params: Params, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    validate_update_params(&params)?;
    let session_id = require_active_session_id(ctx).await?;

    let mut builder = IssueUpdateBuilder::new();
    if let Some(title) = params.title.as_deref() {
        builder = builder.title(title);
    }
    if let Some(issue_type) = params.issue_type.as_deref() {
        builder = builder.issue_type(parse_enum::<IssueType>(issue_type, "type")?);
    }
    if let Some(description) = params.description {
        builder = builder.description(Some(description));
    }
    if let Some(status) = params.status.as_deref() {
        builder = builder.status(parse_enum::<IssueStatus>(status, "status")?);
    }
    if let Some(priority) = params.priority {
        builder = builder.priority(priority);
    }

    let issue = ctx
        .service
        .update_issue(&session_id, &params.id, builder.build())
        .await?;
    output(&issue, flags.format)
}

fn validate_update_params(params: &Params) -> anyhow::Result<()> {
    if params.title.is_none()
        && params.issue_type.is_none()
        && params.description.is_none()
        && params.status.is_none()
        && params.priority.is_none()
    {
        anyhow::bail!(
            "At least one of --title, --type, --description, --status, or --priority must be provided"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Params, validate_update_params};

    #[test]
    fn rejects_noop_update() {
        let params = Params {
            id: String::from("iss-1"),
            title: None,
            issue_type: None,
            description: None,
            status: None,
            priority: None,
        };
        assert!(validate_update_params(&params).is_err());
    }

    #[test]
    fn accepts_update_with_any_field() {
        let params = Params {
            id: String::from("iss-1"),
            title: Some(String::from("updated")),
            issue_type: None,
            description: None,
            status: None,
            priority: None,
        };
        assert!(validate_update_params(&params).is_ok());
    }
}
