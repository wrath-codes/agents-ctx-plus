use zen_core::enums::Confidence;
use zen_db::updates::finding::FindingUpdateBuilder;

use crate::cli::GlobalFlags;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    id: &str,
    content: Option<&str>,
    source: Option<String>,
    confidence: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;

    if content.is_none() && source.is_none() && confidence.is_none() {
        anyhow::bail!("At least one of --content, --source, or --confidence must be provided");
    }

    let mut builder = FindingUpdateBuilder::new();
    if let Some(content) = content {
        builder = builder.content(content);
    }
    if let Some(source) = source {
        builder = builder.source(Some(source));
    }
    if let Some(confidence) = confidence {
        builder = builder.confidence(parse_enum::<Confidence>(confidence, "confidence")?);
    }

    let finding = ctx
        .service
        .update_finding(&session_id, id, builder.build())
        .await?;
    output(&finding, flags.format)
}
