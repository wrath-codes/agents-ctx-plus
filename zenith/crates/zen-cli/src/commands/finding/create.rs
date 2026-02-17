use serde_json::json;
use zen_core::enums::Confidence;

use crate::cli::GlobalFlags;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    content: &str,
    source: Option<&str>,
    confidence: Option<&str>,
    research: Option<&str>,
    tags: &[String],
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let confidence = confidence
        .map(|value| parse_enum::<Confidence>(value, "confidence"))
        .transpose()?
        .unwrap_or(Confidence::Medium);

    let finding = ctx
        .service
        .create_finding(&session_id, content, source, confidence, research)
        .await?;

    let mut applied_tags: Vec<String> = Vec::new();
    for tag in tags {
        if let Err(error) = ctx.service.tag_finding(&session_id, &finding.id, tag).await {
            return Err(anyhow::anyhow!(
                "failed to apply tag '{}' to finding '{}'; applied tags before failure: {:?}: {}",
                tag,
                finding.id,
                applied_tags,
                error
            ));
        }
        applied_tags.push(tag.clone());
    }

    output(
        &json!({
            "finding": finding,
            "tags": applied_tags,
        }),
        flags.format,
    )
}
