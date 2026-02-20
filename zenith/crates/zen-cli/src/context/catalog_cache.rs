use std::path::Path;
use std::time::Duration;

use anyhow::Context;

#[derive(Clone)]
pub struct CatalogCache {
    conn: libsql::Connection,
    ttl: Duration,
}

#[derive(Debug)]
pub enum CacheLookup {
    Miss,
    Fresh(Vec<String>),
    Stale(Vec<String>),
}

impl CatalogCache {
    pub async fn open(path: &Path, ttl: Duration) -> anyhow::Result<Self> {
        let db = libsql::Builder::new_local(path)
            .build()
            .await
            .with_context(|| {
                format!(
                    "failed to open catalog cache database at {}",
                    path.display()
                )
            })?;
        let conn = db.connect().context("failed to connect to catalog cache")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS catalog_cache (
                ecosystem TEXT NOT NULL,
                package TEXT NOT NULL,
                version TEXT NOT NULL,
                scope TEXT NOT NULL,
                lance_path TEXT NOT NULL,
                fetched_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                PRIMARY KEY (ecosystem, package, version, scope, lance_path)
            )",
            (),
        )
        .await
        .context("failed to initialize catalog cache schema")?;

        Ok(Self { conn, ttl })
    }

    pub async fn get_paths(
        &self,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
        scope: &str,
    ) -> anyhow::Result<CacheLookup> {
        let version = version.unwrap_or("");
        let mut rows = self
            .conn
            .query(
                "SELECT lance_path, expires_at FROM catalog_cache
                 WHERE ecosystem = ?1 AND package = ?2 AND version = ?3 AND scope = ?4
                 ORDER BY fetched_at DESC",
                libsql::params![ecosystem, package, version, scope],
            )
            .await
            .context("failed to query catalog cache")?;

        let now = chrono::Utc::now().timestamp();
        let mut fresh = Vec::new();
        let mut stale = Vec::new();

        while let Some(row) = rows.next().await.context("failed to read cache row")? {
            let path: String = row.get(0).context("failed to decode cached lance_path")?;
            if !is_canonical_lance_locator(&path) {
                continue;
            }
            let expires_at: i64 = row.get(1).context("failed to decode cache expiry")?;
            if expires_at > now {
                fresh.push(path);
            } else {
                stale.push(path);
            }
        }

        if !fresh.is_empty() {
            return Ok(CacheLookup::Fresh(fresh));
        }
        if !stale.is_empty() {
            return Ok(CacheLookup::Stale(stale));
        }
        Ok(CacheLookup::Miss)
    }

    pub async fn put_paths(
        &self,
        ecosystem: &str,
        package: &str,
        version: Option<&str>,
        scope: &str,
        paths: &[String],
    ) -> anyhow::Result<()> {
        let version = version.unwrap_or("");
        self.conn
            .execute(
                "DELETE FROM catalog_cache
                 WHERE ecosystem = ?1 AND package = ?2 AND version = ?3 AND scope = ?4",
                libsql::params![ecosystem, package, version, scope],
            )
            .await
            .context("failed to clear previous cache rows")?;

        if paths.is_empty() {
            return Ok(());
        }

        let fetched_at = chrono::Utc::now().timestamp();
        let ttl_secs = i64::try_from(self.ttl.as_secs()).unwrap_or(86_400);
        let expires_at = fetched_at.saturating_add(ttl_secs);

        for path in paths {
            if !is_canonical_lance_locator(path) {
                continue;
            }
            self.conn
                .execute(
                    "INSERT INTO catalog_cache (
                        ecosystem, package, version, scope, lance_path, fetched_at, expires_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    libsql::params![
                        ecosystem,
                        package,
                        version,
                        scope,
                        path.clone(),
                        fetched_at,
                        expires_at
                    ],
                )
                .await
                .context("failed to write cache row")?;
        }

        Ok(())
    }
}

fn is_canonical_lance_locator(path: &str) -> bool {
    path.contains(".lance") && !path.contains('#')
}
