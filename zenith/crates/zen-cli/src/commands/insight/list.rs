use zen_core::entities::Insight;
use zen_core::enums::Confidence;

use crate::cli::GlobalFlags;
use crate::commands::shared::limit::effective_limit;
use crate::commands::shared::parse::parse_enum;
use crate::context::AppContext;
use crate::output::output;

pub async fn run(
    search: Option<&str>,
    confidence: Option<&str>,
    research: Option<&str>,
    limit: Option<u32>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let limit = effective_limit(limit, flags.limit, 20);
    let fetch_limit = compute_fetch_limit(limit, confidence, research);

    let mut insights: Vec<Insight> = if let Some(query) = search {
        ctx.service.search_insights(query, fetch_limit).await?
    } else {
        ctx.service.list_insights(fetch_limit).await?
    };

    insights = filter_basic(insights, confidence, research)?;
    insights.truncate(usize::try_from(limit)?);

    output(&insights, flags.format)
}

fn compute_fetch_limit(limit: u32, confidence: Option<&str>, research: Option<&str>) -> u32 {
    if confidence.is_some() || research.is_some() {
        limit.saturating_mul(5).min(500)
    } else {
        limit
    }
}

fn filter_basic(
    mut insights: Vec<Insight>,
    confidence: Option<&str>,
    research: Option<&str>,
) -> anyhow::Result<Vec<Insight>> {
    if let Some(confidence) = confidence {
        let confidence = parse_enum::<Confidence>(confidence, "confidence")?;
        insights.retain(|item| item.confidence == confidence);
    }
    if let Some(research) = research {
        insights.retain(|item| item.research_id.as_deref() == Some(research));
    }
    Ok(insights)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use zen_core::entities::Insight;
    use zen_core::enums::Confidence;

    use super::{compute_fetch_limit, filter_basic};

    fn mk(id: &str, research_id: Option<&str>, confidence: Confidence) -> Insight {
        Insight {
            id: id.to_string(),
            research_id: research_id.map(str::to_string),
            session_id: Some(String::from("ses-1")),
            content: String::from("content"),
            confidence,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn filters_by_confidence_and_research() {
        let insights = vec![
            mk("i1", Some("res-1"), Confidence::High),
            mk("i2", Some("res-1"), Confidence::Low),
            mk("i3", Some("res-2"), Confidence::High),
        ];
        let filtered =
            filter_basic(insights, Some("high"), Some("res-1")).expect("filter should work");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "i1");
    }

    #[test]
    fn boosts_fetch_limit_when_filters_present() {
        assert_eq!(compute_fetch_limit(20, Some("high"), None), 100);
        assert_eq!(compute_fetch_limit(20, None, Some("res-1")), 100);
        assert_eq!(compute_fetch_limit(20, None, None), 20);
    }
}
