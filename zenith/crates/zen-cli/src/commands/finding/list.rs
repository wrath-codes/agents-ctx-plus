use serde::Serialize;
use std::collections::HashSet;
use zen_core::entities::Finding;
use zen_core::enums::Confidence;

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct FindingListResponse {
    findings: Vec<Finding>,
}

pub async fn run(
    search: Option<&str>,
    research: Option<&str>,
    confidence: Option<&str>,
    tag: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = effective_limit(limit, flags.limit, 20);
    let fetch_limit = compute_fetch_limit(limit, research, confidence, tag);
    let mut findings = if let Some(query) = search {
        ctx.service.search_findings(query, fetch_limit).await?
    } else {
        ctx.service.list_findings(fetch_limit).await?
    };

    findings = filter_basic(findings, research, confidence)?;
    if let Some(tag) = tag {
        let ids = ctx.service.list_finding_ids_by_tag(tag).await?;
        let id_set: HashSet<String> = ids.into_iter().collect();
        findings.retain(|finding| id_set.contains(&finding.id));
    }
    findings.truncate(usize::try_from(limit)?);

    output(&FindingListResponse { findings }, flags.format)
}

fn compute_fetch_limit(
    limit: u32,
    research: Option<&str>,
    confidence: Option<&str>,
    tag: Option<&str>,
) -> u32 {
    if research.is_some() || confidence.is_some() || tag.is_some() {
        limit.saturating_mul(5).min(500)
    } else {
        limit
    }
}

fn filter_basic(
    mut findings: Vec<Finding>,
    research: Option<&str>,
    confidence: Option<&str>,
) -> anyhow::Result<Vec<Finding>> {
    if let Some(research_id) = research {
        findings.retain(|finding| finding.research_id.as_deref() == Some(research_id));
    }
    if let Some(confidence) = confidence {
        let confidence = parse_enum::<Confidence>(confidence, "confidence")?;
        findings.retain(|finding| finding.confidence == confidence);
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use zen_core::entities::Finding;
    use zen_core::enums::Confidence;

    use super::{compute_fetch_limit, filter_basic};

    fn mk(id: &str, research_id: Option<&str>, confidence: Confidence) -> Finding {
        Finding {
            id: id.to_string(),
            research_id: research_id.map(str::to_string),
            session_id: Some(String::from("ses-1")),
            content: String::from("content"),
            source: None,
            confidence,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn filters_by_research_and_confidence() {
        let findings = vec![
            mk("f1", Some("res-1"), Confidence::High),
            mk("f2", Some("res-1"), Confidence::Low),
            mk("f3", Some("res-2"), Confidence::High),
        ];
        let filtered =
            filter_basic(findings, Some("res-1"), Some("high")).expect("filter should work");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "f1");
    }

    #[test]
    fn boosts_fetch_limit_when_any_filter_present() {
        assert_eq!(compute_fetch_limit(20, Some("res-1"), None, None), 100);
        assert_eq!(compute_fetch_limit(20, None, Some("high"), None), 100);
        assert_eq!(compute_fetch_limit(20, None, None, Some("tag")), 100);
        assert_eq!(compute_fetch_limit(20, None, None, None), 20);
    }
}
