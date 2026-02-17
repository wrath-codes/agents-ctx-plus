use zen_core::entities::ResearchItem;
use zen_core::enums::ResearchStatus;

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    status: Option<&str>,
    search: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = effective_limit(limit, flags.limit, 20);
    let fetch_limit = compute_fetch_limit(limit, status);

    let mut items = if let Some(query) = search {
        ctx.service.search_research(query, fetch_limit).await?
    } else {
        ctx.service.list_research(fetch_limit).await?
    };

    items = filter_by_status(items, status)?;
    items.truncate(usize::try_from(limit)?);

    output(&items, flags.format)
}

fn compute_fetch_limit(limit: u32, status: Option<&str>) -> u32 {
    if status.is_some() {
        limit.saturating_mul(5).min(500)
    } else {
        limit
    }
}

fn filter_by_status(
    mut items: Vec<ResearchItem>,
    status: Option<&str>,
) -> anyhow::Result<Vec<ResearchItem>> {
    if let Some(status) = status {
        let status = parse_enum::<ResearchStatus>(status, "status")?;
        items.retain(|item| item.status == status);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use zen_core::enums::ResearchStatus;

    use super::{compute_fetch_limit, filter_by_status};

    fn mk(status: ResearchStatus) -> zen_core::entities::ResearchItem {
        zen_core::entities::ResearchItem {
            id: String::from("res-1"),
            session_id: Some(String::from("ses-1")),
            title: String::from("title"),
            description: None,
            status,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn filters_status() {
        let items = vec![mk(ResearchStatus::Open), mk(ResearchStatus::Resolved)];
        let filtered = filter_by_status(items, Some("resolved")).expect("filter should work");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].status, ResearchStatus::Resolved);
    }

    #[test]
    fn boosts_fetch_limit_when_status_present() {
        assert_eq!(compute_fetch_limit(20, Some("open")), 100);
        assert_eq!(compute_fetch_limit(20, None), 20);
    }
}
