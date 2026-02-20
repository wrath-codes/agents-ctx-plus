//! Clerk organization API helpers.
//!
//! Calls the Clerk Backend API directly via `reqwest` (clerk-rs doesn't expose
//! organization management endpoints). Requires `config.clerk.secret_key`.

use serde::{Deserialize, Serialize};

use crate::AuthError;

const CLERK_API_BASE: &str = "https://api.clerk.com/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMember {
    pub user_id: String,
    pub role: String,
    pub created_at: String,
    /// Email from `public_user_data`, if available.
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgInvitation {
    pub id: String,
    pub email_address: String,
    pub role: String,
    pub status: String,
}

/// List members of a Clerk organization.
///
/// # Errors
///
/// Returns `AuthError::ClerkApiError` if the API call fails or returns non-200.
pub async fn list_members(secret_key: &str, org_id: &str) -> Result<Vec<OrgMember>, AuthError> {
    let url = format!("{CLERK_API_BASE}/organizations/{org_id}/memberships?limit=100");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {secret_key}"))
        .send()
        .await
        .map_err(|e| AuthError::ClerkApiError(format!("list members: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AuthError::ClerkApiError(format!(
            "list members: HTTP {status}: {body}"
        )));
    }

    #[derive(Deserialize)]
    struct ListResponse {
        data: Vec<MembershipRecord>,
    }
    #[derive(Deserialize)]
    struct MembershipRecord {
        public_user_data: Option<PublicUserData>,
        role: String,
        created_at: i64,
    }
    #[derive(Deserialize)]
    struct PublicUserData {
        user_id: String,
        identifier: Option<String>,
    }

    let list: ListResponse = resp
        .json()
        .await
        .map_err(|e| AuthError::ClerkApiError(format!("parse members: {e}")))?;

    Ok(list
        .data
        .into_iter()
        .filter_map(|m| {
            let pud = m.public_user_data?;
            Some(OrgMember {
                user_id: pud.user_id,
                role: m.role,
                created_at: chrono::DateTime::from_timestamp(m.created_at / 1000, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                email: pud.identifier,
            })
        })
        .collect())
}

/// Invite a user to a Clerk organization by email.
///
/// # Errors
///
/// Returns `AuthError::ClerkApiError` if the API call fails.
pub async fn invite_member(
    secret_key: &str,
    org_id: &str,
    email: &str,
    role: &str,
) -> Result<OrgInvitation, AuthError> {
    let url = format!("{CLERK_API_BASE}/organizations/{org_id}/invitations");
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {secret_key}"))
        .json(&serde_json::json!({
            "email_address": email,
            "role": role,
        }))
        .send()
        .await
        .map_err(|e| AuthError::ClerkApiError(format!("invite member: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AuthError::ClerkApiError(format!(
            "invite member: HTTP {status}: {body}"
        )));
    }

    resp.json()
        .await
        .map_err(|e| AuthError::ClerkApiError(format!("parse invitation: {e}")))
}
