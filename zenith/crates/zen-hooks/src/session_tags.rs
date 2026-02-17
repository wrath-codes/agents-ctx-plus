use std::path::Path;

use crate::error::HookError;

pub fn create_session_tag(
    project_root: &Path,
    session_id: &str,
    target: &str,
) -> Result<(), HookError> {
    let repo = gix::discover(project_root)
        .map_err(|_| HookError::NotGitRepo(project_root.to_path_buf()))?;
    let tag_name = format!("refs/tags/zenith/{session_id}");

    if repo.find_reference(&tag_name).is_ok() {
        return Ok(());
    }

    let oid: gix::ObjectId = target
        .parse()
        .map_err(|e| HookError::Git(format!("parse target object id: {e}")))?;

    repo.edit_reference(gix::refs::transaction::RefEdit {
        change: gix::refs::transaction::Change::Update {
            log: gix::refs::transaction::LogChange {
                mode: gix::refs::transaction::RefLog::AndReference,
                force_create_reflog: false,
                message: "zenith session tag".into(),
            },
            expected: gix::refs::transaction::PreviousValue::MustNotExist,
            new: gix::refs::Target::Object(oid),
        },
        name: tag_name
            .as_str()
            .try_into()
            .map_err(|e| HookError::Git(format!("invalid tag name: {e}")))?,
        deref: false,
    })
    .map_err(|e| HookError::Git(format!("create session tag: {e}")))?;

    Ok(())
}
