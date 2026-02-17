use chrono::Utc;

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
                 (id, snapshot_id, ecosystem, package, version, lance_path, visibility, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'public', ?7)
                 ON CONFLICT(ecosystem, package, version, lance_path)
                 DO NOTHING",
                libsql::params![
                    file_id.as_str(),
                    snapshot_id.as_str(),
                    ecosystem,
                    package,
                    version,
                    lance_path,
                    now.as_str()
                ],
            )
            .await?;

        Ok(())
    }

    /// Check whether a package version exists in cloud catalog.
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
                 LIMIT 1",
                libsql::params![ecosystem, package, version],
            )
            .await?;
        Ok(rows.next().await?.is_some())
    }

    /// Resolve all catalog lance paths for a package triplet.
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
                    "SELECT lance_path FROM dl_data_file WHERE ecosystem = ?1 AND package = ?2 AND version = ?3 ORDER BY created_at DESC, id DESC",
                    libsql::params![ecosystem, package, version],
                )
                .await?
        } else {
            self.db()
                .conn()
                .query(
                    "SELECT lance_path FROM dl_data_file WHERE ecosystem = ?1 AND package = ?2 ORDER BY created_at DESC, id DESC",
                    libsql::params![ecosystem, package],
                )
                .await?
        };

        while let Some(row) = rows.next().await? {
            paths.push(row.get::<String>(0)?);
        }

        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::helpers::test_service;

    #[tokio::test]
    async fn register_and_query_catalog_package() {
        let svc = test_service().await;

        svc.register_catalog_data_file(
            "rust",
            "tokio",
            "1.40.0",
            "r2://zenith/public/rust/tokio/1.40.0/symbols.lance",
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

        svc.register_catalog_data_file("rust", "serde", "1.0.0", "path/a")
            .await
            .unwrap();
        svc.register_catalog_data_file("rust", "serde", "1.0.0", "path/b")
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

        svc.register_catalog_data_file("rust", "serde", "1.0.0", "path/symbols.lance")
            .await
            .unwrap();
        svc.register_catalog_data_file("rust", "serde", "1.0.0", "path/symbols.lance")
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
}
