use zen_core::enums::TaskStatus;
use zen_db::updates::task::TaskUpdateBuilder;

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(id: &str, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let update = TaskUpdateBuilder::new().status(TaskStatus::Done).build();
    let task = ctx.service.update_task(&session_id, id, update).await?;
    output(&task, flags.format)
}
