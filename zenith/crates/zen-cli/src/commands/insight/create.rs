use zen_core::enums::Confidence;

use crate::cli::GlobalFlags;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    content: &str,
    confidence: Option<&str>,
    research: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let confidence = confidence
        .map(|value| parse_enum::<Confidence>(value, "confidence"))
        .transpose()?
        .unwrap_or(Confidence::Medium);

    let insight = ctx
        .service
        .create_insight(&session_id, content, confidence, research)
        .await?;
    output(&insight, flags.format)
}
