use zen_core::enums::StudyMethodology;

use crate::cli::GlobalFlags;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    topic: &str,
    library: Option<&str>,
    methodology: Option<&str>,
    research: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let methodology = methodology
        .map(|value| parse_enum::<StudyMethodology>(value, "methodology"))
        .transpose()?
        .unwrap_or(StudyMethodology::Explore);

    let study = ctx
        .service
        .create_study(&session_id, topic, library, methodology, research)
        .await?;

    output(&study, flags.format)
}
