use zen_core::entities::Hypothesis;
use zen_core::enums::HypothesisStatus;

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    status: Option<&str>,
    research: Option<&str>,
    search: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = effective_limit(limit, flags.limit, 20);
    let fetch_limit = compute_fetch_limit(limit, status, research);

    let mut hypotheses: Vec<Hypothesis> = if let Some(query) = search {
        ctx.service.search_hypotheses(query, fetch_limit).await?
    } else {
        ctx.service.list_hypotheses(fetch_limit).await?
    };

    hypotheses = filter_basic(hypotheses, status, research)?;
    hypotheses.truncate(usize::try_from(limit)?);

    output(&hypotheses, flags.format)
}

fn compute_fetch_limit(limit: u32, status: Option<&str>, research: Option<&str>) -> u32 {
    if status.is_some() || research.is_some() {
        limit.saturating_mul(5).min(500)
    } else {
        limit
    }
}

fn filter_basic(
    mut hypotheses: Vec<Hypothesis>,
    status: Option<&str>,
    research: Option<&str>,
) -> anyhow::Result<Vec<Hypothesis>> {
    if let Some(status) = status {
        let status = parse_enum::<HypothesisStatus>(status, "status")?;
        hypotheses.retain(|item| item.status == status);
    }
    if let Some(research) = research {
        hypotheses.retain(|item| item.research_id.as_deref() == Some(research));
    }
    Ok(hypotheses)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use zen_core::entities::Hypothesis;
    use zen_core::enums::HypothesisStatus;

    use super::{compute_fetch_limit, filter_basic};

    fn mk(id: &str, research_id: Option<&str>, status: HypothesisStatus) -> Hypothesis {
        Hypothesis {
            id: id.to_string(),
            research_id: research_id.map(str::to_string),
            finding_id: None,
            session_id: Some(String::from("ses-1")),
            content: String::from("content"),
            status,
            reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn filters_by_status_and_research() {
        let hyps = vec![
            mk("h1", Some("res-1"), HypothesisStatus::Confirmed),
            mk("h2", Some("res-1"), HypothesisStatus::Analyzing),
            mk("h3", Some("res-2"), HypothesisStatus::Confirmed),
        ];
        let filtered =
            filter_basic(hyps, Some("confirmed"), Some("res-1")).expect("filter should work");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "h1");
    }

    #[test]
    fn boosts_fetch_limit_when_filters_present() {
        assert_eq!(compute_fetch_limit(20, Some("confirmed"), None), 100);
        assert_eq!(compute_fetch_limit(20, None, Some("res-1")), 100);
        assert_eq!(compute_fetch_limit(20, None, None), 20);
    }
}
