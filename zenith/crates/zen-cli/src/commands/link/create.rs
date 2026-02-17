use crate::cli::GlobalFlags;
use crate::cli::root_commands::LinkArgs;
use crate::commands::shared::parse::parse_enum;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;
use zen_core::enums::{EntityType, Relation};

pub async fn run(args: &LinkArgs, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let source_type = parse_enum::<EntityType>(&args.source_type, "source_type")?;
    let target_type = parse_enum::<EntityType>(&args.target_type, "target_type")?;
    let relation = parse_enum::<Relation>(&args.relation, "relation")?;

    let link = ctx
        .service
        .create_link(
            &session_id,
            source_type,
            &args.source_id,
            target_type,
            &args.target_id,
            relation,
        )
        .await?;

    output(&link, flags.format)
}
