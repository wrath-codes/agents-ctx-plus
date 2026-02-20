use chrono::Utc;
use zen_core::enums::Visibility;
use zen_core::identity::AuthIdentity;

use crate::error::DatabaseError;
use crate::service::ZenService;

fn stable_key(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Build a visibility filter SQL clause.
///
/// When identity is available:
///   `AND (visibility = 'public' OR (visibility = 'team' AND org_id = ?N) OR (visibility = 'private' AND owner_sub = ?N+1))`
/// When identity is `None`:
///   `AND visibility = 'public'`
fn visibility_filter_sql(
    identity: Option<&AuthIdentity>,
    start_param: u32,
) -> (String, Vec<libsql::Value>) {
    match identity {
        Some(id) => {
            let mut params: Vec<libsql::Value> = Vec::new();
            let mut clauses = vec!["visibility = 'public'".to_string()];

            let mut idx = start_param;
            if let Some(ref org_id) = id.org_id {
                clauses.push(format!("(visibility = 'team' AND org_id = ?{idx})"));
                params.push(org_id.as_str().into());
                idx += 1;
            }

            clauses.push(format!("(visibility = 'private' AND owner_sub = ?{idx})"));
            params.push(id.user_id.as_str().into());

            (format!("AND ({})", clauses.join(" OR ")), params)
        }
        None => ("AND visibility = 'public'".to_string(), vec![]),
    }
}

impl ZenService {
    /// Register a lance data path for an indexed package in catalog.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if snapshot or data file writes fail.
    pub async fn register_catalog_data_file(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
        lance_path: &str,
        visibility: Visibility,
        org_id: Option<&str>,
        owner_sub: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();
        let snapshot_id = format!(
            "dls-{}-{}-{}",
            stable_key(ecosystem),
            stable_key(package),
            stable_key(version)
        );
        let file_id = self.db().generate_id("dlf").await?;

        self.db()
            .conn()
            .execute(
                "INSERT OR IGNORE INTO dl_snapshot (id, created_at, note) VALUES (?1, ?2, ?3)",
                libsql::params![snapshot_id.as_str(), now.as_str(), "auto"],
            )
            .await?;

        self.db()
            .conn()
            .execute(
                "INSERT INTO dl_data_file
                 (id, snapshot_id, ecosystem, package, version, lance_path, visibility, org_id, owner_sub, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(ecosystem, package, version, lance_path)
                 DO NOTHING",
                libsql::params![
                    file_id.as_str(),
                    snapshot_id.as_str(),
                    ecosystem,
                    package,
                    version,
                    lance_path,
                    visibility.as_str(),
                    org_id,
                    owner_sub,
                    now.as_str()
                ],
            )
            .await?;

        Ok(())
    }

    /// Check whether a public package version exists in cloud catalog.
    ///
    /// Safe default: only checks public entries (used for crowdsource dedup).
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the lookup query fails.
    pub async fn catalog_has_package(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<bool, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT 1 FROM dl_data_file
                 WHERE ecosystem = ?1 AND package = ?2 AND version = ?3
                   AND lance_path LIKE '%symbols.lance%'
                   AND visibility = 'public'
                 LIMIT 1",
                libsql::params![ecosystem, package, version],
            )
            .await?;
        Ok(rows.next().await?.is_some())
    }

    /// Check whether a public package is already indexed in the catalog.
    ///
    /// Returns existing Lance paths if found, `None` if not indexed.
    /// Used by `znt install` to skip re-indexing of crowdsourced packages.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the lookup query fails.
    pub async fn catalog_check_before_index(
        &self,
        ecosystem: &str,
        package: &str,
        version: &str,
    ) -> Result<Option<Vec<String>>, DatabaseError> {
        let mut rows = self
            .db()
            .conn()
            .query(
                "SELECT lance_path FROM dl_data_file
                 WHERE ecosystem = ?1 AND package = ?2 AND version = ?3
                   AND visibility = 'public'
                   AND lance_path LIKE '%symbols.lance%'
                 ORDER BY created_at DESC",
                libsql::params![ecosystem, package, version],
            )
            .await?;

        let mut paths = Vec::new();
        while let Some(row) = rows.next().await? {
            paths.push(row.get::<String>(0)?);
        }

        if paths.is_empty() {
            Ok(None)
        } else {
            Ok(Some(paths))
        }
    }

    /// Resolve catalog lance paths — **public only** (safe default).
    ///
    /// Use `catalog_paths_for_package_scoped()` for multi-tier visibility.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if query execution or row decoding fails.
    pub async fn catalog_paths_for_package(
        &self,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
    ) -> Result<Vec<String>, DatabaseError> {
        let mut paths = Vec::new();

        let mut rows = if let Some(version) = version {
            self.db()
                .conn()
                .query(
                    "SELECT lance_path FROM dl_data_file
                     WHERE ecosystem = ?1 AND package = ?2 AND version = ?3
                       AND visibility = 'public'
                     ORDER BY created_at DESC, id DESC",
                    libsql::params![ecosystem, package, version],
                )
                .await?
        } else {
            self.db()
                .conn()
                .query(
                    "SELECT lance_path FROM dl_data_file
                     WHERE ecosystem = ?1 AND package = ?2
                       AND visibility = 'public'
                     ORDER BY created_at DESC, id DESC",
                    libsql::params![ecosystem, package],
                )
                .await?
        };

        while let Some(row) = rows.next().await? {
            paths.push(row.get::<String>(0)?);
        }

        Ok(paths)
    }

    /// Resolve catalog lance paths scoped to the current identity's visibility.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if query execution or row decoding fails.
    pub async fn catalog_paths_for_package_scoped(
        &self,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
    ) -> Result<Vec<String>, DatabaseError> {
        let identity = self.identity();
        let mut paths = Vec::new();

        let mut all_params: Vec<libsql::Value> = vec![ecosystem.into(), package.into()];

        let (version_clause, next_param) = if let Some(v) = version {
            all_params.push(v.into());
            ("AND version = ?3", 4u32)
        } else {
            ("", 3u32)
        };

        let (vis_filter, vis_params) = visibility_filter_sql(identity, next_param);
        all_params.extend(vis_params);

        let sql = format!(
            "SELECT lance_path FROM dl_data_file
             WHERE ecosystem = ?1 AND package = ?2 {version_clause}
             {vis_filter}
             ORDER BY created_at DESC, id DESC"
        );

        let mut rows = self
            .db()
            .conn()
            .query(&sql, libsql::params_from_iter(all_params))
            .await?;

        while let Some(row) = rows.next().await? {
            paths.push(row.get::<String>(0)?);
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use zen_core::enums::Visibility;
    use zen_core::identity::AuthIdentity;

    use crate::test_support::helpers::{test_service, test_service_with_identity};

    #[tokio::test]
    async fn register_and_query_catalog_package() {
        let svc = test_service().await;

        svc.register_catalog_data_file(
            "rust",
            "tokio",
            "1.40.0",
            "r2://zenith/public/rust/tokio/1.40.0/symbols.lance",
            Visibility::Public,
            None,
            None,
        )
        .await
        .unwrap();

        assert!(
            svc.catalog_has_package("rust", "tokio", "1.40.0")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn catalog_paths_returns_latest_first() {
        let svc = test_service().await;

        svc.register_catalog_data_file("rust", "serde", "1.0.0", "path/a", Visibility::Public, None, None)
            .await
            .unwrap();
        svc.register_catalog_data_file("rust", "serde", "1.0.0", "path/b", Visibility::Public, None, None)
            .await
            .unwrap();

        let paths = svc
            .catalog_paths_for_package("rust", "serde", Some("1.0.0"))
            .await
            .unwrap();

        assert_eq!(paths.first().map(String::as_str), Some("path/b"));
        assert!(paths.contains(&"path/a".to_string()));
    }

    #[tokio::test]
    async fn register_catalog_data_file_is_idempotent() {
        let svc = test_service().await;

        svc.register_catalog_data_file("rust", "serde", "1.0.0", "path/symbols.lance", Visibility::Public, None, None)
            .await
            .unwrap();
        svc.register_catalog_data_file("rust", "serde", "1.0.0", "path/symbols.lance", Visibility::Public, None, None)
            .await
            .unwrap();

        let mut rows = svc
            .db()
            .conn()
            .query(
                "SELECT COUNT(*) FROM dl_data_file WHERE ecosystem = ?1 AND package = ?2 AND version = ?3",
                libsql::params!["rust", "serde", "1.0.0"],
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        assert_eq!(row.get::<i64>(0).unwrap(), 1);

        let mut snap_rows = svc
            .db()
            .conn()
            .query("SELECT COUNT(*) FROM dl_snapshot", ())
            .await
            .unwrap();
        let snap_row = snap_rows.next().await.unwrap().unwrap();
        assert_eq!(snap_row.get::<i64>(0).unwrap(), 1);
    }

    #[tokio::test]
    async fn catalog_has_package_public_only() {
        let svc = test_service().await;

        // Private entry should NOT be found by catalog_has_package
        svc.register_catalog_data_file(
            "rust", "secret", "1.0.0", "path/symbols.lance",
            Visibility::Private, None, Some("user_1"),
        )
        .await
        .unwrap();
        assert!(!svc.catalog_has_package("rust", "secret", "1.0.0").await.unwrap());

        // Public entry should be found
        svc.register_catalog_data_file(
            "rust", "public_pkg", "1.0.0", "path/symbols.lance",
            Visibility::Public, None, Some("user_1"),
        )
        .await
        .unwrap();
        assert!(svc.catalog_has_package("rust", "public_pkg", "1.0.0").await.unwrap());
    }

    #[tokio::test]
    async fn catalog_check_before_index_returns_paths() {
        let svc = test_service().await;

        // Not indexed yet
        assert!(svc.catalog_check_before_index("rust", "tokio", "1.49.0").await.unwrap().is_none());

        // Index it
        svc.register_catalog_data_file(
            "rust", "tokio", "1.49.0", "s3://bucket/public/rust/tokio/1.49.0/symbols.lance",
            Visibility::Public, None, Some("user_1"),
        )
        .await
        .unwrap();

        let paths = svc.catalog_check_before_index("rust", "tokio", "1.49.0").await.unwrap();
        assert!(paths.is_some());
        assert_eq!(paths.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn catalog_paths_for_package_defaults_to_public() {
        let svc = test_service().await;

        svc.register_catalog_data_file("rust", "mix", "1.0.0", "path/pub", Visibility::Public, None, None)
            .await.unwrap();
        svc.register_catalog_data_file("rust", "mix", "1.0.0", "path/priv", Visibility::Private, None, Some("u1"))
            .await.unwrap();
        svc.register_catalog_data_file("rust", "mix", "1.0.0", "path/team", Visibility::Team, Some("org_a"), Some("u1"))
            .await.unwrap();

        let paths = svc.catalog_paths_for_package("rust", "mix", Some("1.0.0")).await.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], "path/pub");
    }

    #[tokio::test]
    async fn catalog_paths_scoped_with_identity() {
        let identity = AuthIdentity {
            user_id: "user_1".to_string(),
            org_id: Some("org_a".to_string()),
            org_slug: None,
            org_role: None,
        };
        let svc = test_service_with_identity(identity).await;

        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/pub", Visibility::Public, None, None)
            .await.unwrap();
        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/team_a", Visibility::Team, Some("org_a"), Some("user_1"))
            .await.unwrap();
        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/team_b", Visibility::Team, Some("org_b"), Some("user_2"))
            .await.unwrap();
        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/priv_me", Visibility::Private, None, Some("user_1"))
            .await.unwrap();
        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/priv_other", Visibility::Private, None, Some("user_2"))
            .await.unwrap();

        let paths = svc.catalog_paths_for_package_scoped("rust", "pkg", Some("1.0.0")).await.unwrap();
        // Should see: public, team_a (same org), priv_me (same user)
        // Should NOT see: team_b (different org), priv_other (different user)
        assert!(paths.contains(&"path/pub".to_string()));
        assert!(paths.contains(&"path/team_a".to_string()));
        assert!(paths.contains(&"path/priv_me".to_string()));
        assert!(!paths.contains(&"path/team_b".to_string()));
        assert!(!paths.contains(&"path/priv_other".to_string()));
        assert_eq!(paths.len(), 3);
    }

    #[tokio::test]
    async fn catalog_paths_scoped_no_identity_public_only() {
        let svc = test_service().await;

        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/pub", Visibility::Public, None, None)
            .await.unwrap();
        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/priv", Visibility::Private, None, Some("u1"))
            .await.unwrap();

        let paths = svc.catalog_paths_for_package_scoped("rust", "pkg", Some("1.0.0")).await.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], "path/pub");
    }

    #[tokio::test]
    async fn catalog_paths_scoped_identity_no_org_skips_team() {
        let identity = AuthIdentity {
            user_id: "user_solo".to_string(),
            org_id: None,
            org_slug: None,
            org_role: None,
        };
        let svc = test_service_with_identity(identity).await;

        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/pub", Visibility::Public, None, None)
            .await.unwrap();
        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/team_a", Visibility::Team, Some("org_a"), Some("user_solo"))
            .await.unwrap();
        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/priv_me", Visibility::Private, None, Some("user_solo"))
            .await.unwrap();
        svc.register_catalog_data_file("rust", "pkg", "1.0.0", "path/priv_other", Visibility::Private, None, Some("user_other"))
            .await.unwrap();

        let paths = svc.catalog_paths_for_package_scoped("rust", "pkg", Some("1.0.0")).await.unwrap();
        // No org → team entries are invisible even if owner matches
        assert!(paths.contains(&"path/pub".to_string()));
        assert!(paths.contains(&"path/priv_me".to_string()));
        assert!(!paths.contains(&"path/team_a".to_string()));
        assert!(!paths.contains(&"path/priv_other".to_string()));
        assert_eq!(paths.len(), 2);
    }

    #[tokio::test]
    async fn register_with_visibility_stores_metadata() {
        let svc = test_service().await;

        svc.register_catalog_data_file(
            "rust", "team_pkg", "1.0.0", "path/symbols.lance",
            Visibility::Team, Some("org_x"), Some("user_y"),
        )
        .await
        .unwrap();

        let mut rows = svc
            .db()
            .conn()
            .query(
                "SELECT visibility, org_id, owner_sub FROM dl_data_file WHERE package = ?1",
                libsql::params!["team_pkg"],
            )
            .await
            .unwrap();
        let row = rows.next().await.unwrap().unwrap();
        assert_eq!(row.get::<String>(0).unwrap(), "team");
        assert_eq!(row.get::<String>(1).unwrap(), "org_x");
        assert_eq!(row.get::<String>(2).unwrap(), "user_y");
    }
}
