use chrono::{DateTime, Utc};
use zen_core::identity::AuthIdentity;

/// Parsed and validated Clerk JWT claims.
///
/// Wraps the relevant fields from `clerk-rs::ClerkJwt` into a Zenith-specific
/// struct. Produced by JWKS validation, consumed by CLI commands and `AppContext`.
#[derive(Debug, Clone)]
pub struct ZenClaims {
    /// Raw JWT string (for passing to Turso).
    pub raw_jwt: String,
    /// Clerk user ID (`sub` claim).
    pub user_id: String,
    /// Organization ID (`org_id` claim). `None` if personal/no-org session.
    pub org_id: Option<String>,
    /// Organization slug (`org_slug` claim).
    pub org_slug: Option<String>,
    /// Organization role (`org_role` claim, e.g. `"org:admin"`).
    pub org_role: Option<String>,
    /// Token expiration time (from `exp` claim).
    pub expires_at: DateTime<Utc>,
}

impl ZenClaims {
    /// Convert to a lightweight `AuthIdentity` for cross-crate passing.
    #[must_use]
    pub fn to_identity(&self) -> AuthIdentity {
        AuthIdentity {
            user_id: self.user_id.clone(),
            org_id: self.org_id.clone(),
            org_slug: self.org_slug.clone(),
            org_role: self.org_role.clone(),
        }
    }

    /// Check if the token is expired or expires within `buffer_secs`.
    #[must_use]
    pub fn is_near_expiry(&self, buffer_secs: i64) -> bool {
        let threshold = Utc::now() + chrono::TimeDelta::seconds(buffer_secs);
        self.expires_at <= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_claims(expires_at: DateTime<Utc>) -> ZenClaims {
        ZenClaims {
            raw_jwt: "test.jwt.token".into(),
            user_id: "user_test123".into(),
            org_id: Some("org_abc".into()),
            org_slug: Some("my-org".into()),
            org_role: Some("org:admin".into()),
            expires_at,
        }
    }

    #[test]
    fn to_identity_maps_all_fields() {
        let claims = make_claims(Utc::now() + chrono::TimeDelta::hours(1));
        let identity = claims.to_identity();
        assert_eq!(identity.user_id, "user_test123");
        assert_eq!(identity.org_id.as_deref(), Some("org_abc"));
        assert_eq!(identity.org_slug.as_deref(), Some("my-org"));
        assert_eq!(identity.org_role.as_deref(), Some("org:admin"));
    }

    #[test]
    fn is_near_expiry_false_when_far_future() {
        let claims = make_claims(Utc::now() + chrono::TimeDelta::hours(1));
        assert!(!claims.is_near_expiry(60));
    }

    #[test]
    fn is_near_expiry_true_when_past() {
        let claims = make_claims(Utc::now() - chrono::TimeDelta::seconds(10));
        assert!(claims.is_near_expiry(60));
    }

    #[test]
    fn is_near_expiry_true_within_buffer() {
        let claims = make_claims(Utc::now() + chrono::TimeDelta::seconds(30));
        assert!(claims.is_near_expiry(60));
    }

    #[test]
    fn is_near_expiry_false_just_outside_buffer() {
        let claims = make_claims(Utc::now() + chrono::TimeDelta::seconds(120));
        assert!(!claims.is_near_expiry(60));
    }

    #[test]
    fn to_identity_handles_none_org() {
        let claims = ZenClaims {
            raw_jwt: "test.jwt.token".into(),
            user_id: "user_personal".into(),
            org_id: None,
            org_slug: None,
            org_role: None,
            expires_at: Utc::now() + chrono::TimeDelta::hours(1),
        };
        let identity = claims.to_identity();
        assert_eq!(identity.user_id, "user_personal");
        assert!(identity.org_id.is_none());
        assert!(identity.org_slug.is_none());
        assert!(identity.org_role.is_none());
    }
}
