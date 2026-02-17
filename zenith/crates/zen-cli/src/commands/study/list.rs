use zen_core::entities::Study;
use zen_core::enums::StudyStatus;

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    status: Option<&str>,
    library: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = effective_limit(limit, flags.limit, 20);
    let fetch_limit = compute_fetch_limit(limit, status, library);
    let mut studies: Vec<Study> = ctx.service.list_studies(fetch_limit).await?;

    studies = filter_basic(studies, status, library)?;
    studies.truncate(usize::try_from(limit)?);

    output(&studies, flags.format)
}

fn compute_fetch_limit(limit: u32, status: Option<&str>, library: Option<&str>) -> u32 {
    if status.is_some() || library.is_some() {
        limit.saturating_mul(5).min(500)
    } else {
        limit
    }
}

fn filter_basic(
    mut studies: Vec<Study>,
    status: Option<&str>,
    library: Option<&str>,
) -> anyhow::Result<Vec<Study>> {
    if let Some(status) = status {
        let status = parse_enum::<StudyStatus>(status, "status")?;
        studies.retain(|item| item.status == status);
    }
    if let Some(library) = library {
        studies.retain(|item| item.library.as_deref() == Some(library));
    }
    Ok(studies)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use zen_core::entities::Study;
    use zen_core::enums::{StudyMethodology, StudyStatus};

    use super::{compute_fetch_limit, filter_basic};

    fn mk(id: &str, status: StudyStatus, library: Option<&str>) -> Study {
        Study {
            id: id.to_string(),
            session_id: Some(String::from("ses-1")),
            research_id: None,
            topic: String::from("topic"),
            library: library.map(str::to_string),
            methodology: StudyMethodology::Explore,
            status,
            summary: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn filters_by_status_and_library() {
        let studies = vec![
            mk("s1", StudyStatus::Active, Some("tokio")),
            mk("s2", StudyStatus::Completed, Some("tokio")),
            mk("s3", StudyStatus::Active, Some("axum")),
        ];
        let filtered =
            filter_basic(studies, Some("active"), Some("tokio")).expect("filter should work");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "s1");
    }

    #[test]
    fn boosts_fetch_limit_when_filters_present() {
        assert_eq!(compute_fetch_limit(20, Some("active"), None), 100);
        assert_eq!(compute_fetch_limit(20, None, Some("tokio")), 100);
        assert_eq!(compute_fetch_limit(20, None, None), 20);
    }
}
