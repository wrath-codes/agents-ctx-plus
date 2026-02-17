use serde::Serialize;
use zen_core::enums::{Confidence, EntityType, HypothesisStatus};
use zen_db::updates::hypothesis::HypothesisUpdateBuilder;

use crate::cli::GlobalFlags;
use crate::commands::shared::session::require_active_session_id;
use crate::context::AppContext;
use crate::output::output;

#[derive(Debug, Serialize)]
struct TestResponse {
    study_id: String,
    assumption_id: String,
    finding_id: String,
    result: String,
}

pub async fn run(
    study_id: &str,
    assumption_id: &str,
    result: &str,
    evidence: Option<&str>,
    ctx: &AppContext,
    flags: &GlobalFlags,
) -> anyhow::Result<()> {
    let session_id = require_active_session_id(ctx).await?;
    let (status, confidence) = map_result(result)?;

    let finding_id = ctx
        .service
        .record_test_result(
            &session_id,
            study_id,
            assumption_id,
            evidence.unwrap_or(result),
            confidence,
        )
        .await?;

    let update = HypothesisUpdateBuilder::new()
        .status(status)
        .reason(Some(evidence.unwrap_or(result).to_string()))
        .build();
    if let Err(error) = ctx
        .service
        .update_hypothesis(&session_id, assumption_id, update)
        .await
    {
        let rollback_error = rollback_test_finding(ctx, &session_id, &finding_id)
            .await
            .err();
        return match rollback_error {
            Some(rollback_error) => Err(anyhow::anyhow!(
                "recorded test finding '{}' but failed to update hypothesis '{}': {}; rollback also failed: {}",
                finding_id,
                assumption_id,
                error,
                rollback_error
            )),
            None => Err(anyhow::anyhow!(
                "recorded test finding '{}' but failed to update hypothesis '{}': {}; rollback succeeded",
                finding_id,
                assumption_id,
                error
            )),
        };
    }

    output(
        &TestResponse {
            study_id: study_id.to_string(),
            assumption_id: assumption_id.to_string(),
            finding_id,
            result: result.to_string(),
        },
        flags.format,
    )
}

async fn rollback_test_finding(
    ctx: &AppContext,
    session_id: &str,
    finding_id: &str,
) -> anyhow::Result<()> {
    let mut link_ids = Vec::new();

    let from_links = ctx
        .service
        .get_links_from(EntityType::Finding, finding_id)
        .await?;
    link_ids.extend(from_links.into_iter().map(|link| link.id));

    let to_links = ctx
        .service
        .get_links_to(EntityType::Finding, finding_id)
        .await?;
    for link in to_links {
        if !link_ids.iter().any(|id| id == &link.id) {
            link_ids.push(link.id);
        }
    }

    for link_id in link_ids {
        ctx.service.delete_link(session_id, &link_id).await?;
    }

    ctx.service.delete_finding(session_id, finding_id).await?;
    Ok(())
}

fn map_result(result: &str) -> anyhow::Result<(HypothesisStatus, Confidence)> {
    match result {
        "validated" => Ok((HypothesisStatus::Confirmed, Confidence::High)),
        "invalidated" => Ok((HypothesisStatus::Debunked, Confidence::High)),
        "inconclusive" => Ok((HypothesisStatus::Inconclusive, Confidence::Medium)),
        _ => Err(anyhow::anyhow!(
            "invalid result '{}': expected validated|invalidated|inconclusive",
            result
        )),
    }
}

#[cfg(test)]
mod tests {
    use zen_core::enums::{Confidence, HypothesisStatus};

    use super::map_result;

    #[test]
    fn map_result_validated() {
        let mapped = map_result("validated").expect("validated should parse");
        assert_eq!(mapped, (HypothesisStatus::Confirmed, Confidence::High));
    }

    #[test]
    fn map_result_invalidated() {
        let mapped = map_result("invalidated").expect("invalidated should parse");
        assert_eq!(mapped, (HypothesisStatus::Debunked, Confidence::High));
    }

    #[test]
    fn map_result_rejects_unknown() {
        let err = map_result("maybe").expect_err("should fail");
        assert!(err.to_string().contains("invalid result 'maybe'"));
    }
}
