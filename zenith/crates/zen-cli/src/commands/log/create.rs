use crate::cli::GlobalFlags;
use crate::cli::root_commands::LogArgs;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

use super::parse_location::parse_location;

pub async fn run(args: &LogArgs, ctx: &AppContext, flags: &GlobalFlags) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let task_id = required_task_id(args)?;
    let parsed = parse_location(&args.location)?;

    let log = ctx
        .service
        .create_impl_log(
            &session_id,
            task_id,
            &parsed.file_path,
            parsed.start_line,
            parsed.end_line,
            args.description.as_deref(),
        )
        .await?;

    output(&log, flags.format)
}

fn required_task_id(args: &LogArgs) -> anyhow::Result<&str> {
    args.task
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--task is required for znt log"))
}

#[cfg(test)]
mod tests {
    use super::required_task_id;
    use crate::cli::root_commands::LogArgs;

    #[test]
    fn rejects_missing_task() {
        let args = LogArgs {
            location: String::from("src/main.rs#1-2"),
            task: None,
            description: None,
        };
        assert!(required_task_id(&args).is_err());
    }

    #[test]
    fn accepts_present_task() {
        let args = LogArgs {
            location: String::from("src/main.rs#1-2"),
            task: Some(String::from("tsk-1")),
            description: None,
        };
        assert_eq!(required_task_id(&args).expect("task should exist"), "tsk-1");
    }
}
