# Phase 8: Authentication & Identity — Implementation Plan

**Version**: 2026-02-19
**Status**: Planning
**Depends on**: Phase 5 (zen-cli — working `znt` binary with all domain commands — **DONE**), Phase 7 (AgentFS integration — **DONE**), Roadmap Phase 8 (Cloud & Catalog — Turso sync, DuckLake catalog, R2 Lance writes — **DONE**), Spikes 0.17 (Clerk auth + Turso JWKS — **14/14 pass**) and 0.20 (catalog visibility — **9/9 pass**)
**Produces**: Milestone 8 — `zen-auth` crate with Clerk-based browser login, OS keychain token storage, JWKS JWT validation, API key fallback for CI/headless, and token lifecycle management. CLI auth commands (`znt auth login/logout/status/switch-org`). Turso JWKS wiring — Clerk JWT replaces Platform API token as the libsql auth token. AppContext identity-aware startup with graceful degradation to local mode. Token refresh detects expiry and triggers re-auth or client recreation.

> **⚠️ Scope**: Phase 8 is **authentication infrastructure + CLI identity commands**. It does NOT include visibility scoping (public/team/private), org_id columns on entities, AuthContext threading through repos, team commands (`znt team`), private code indexing (`znt index .`), or federated search with visibility filters. Those are deferred to Phase 9. Phase 8 provides the identity layer that Phase 9 builds on.
>
> **📌 Numbering note**: The master roadmap (`07-implementation-plan.md`) assigns auth tasks to Phase 9 (tasks 9.1–9.7, 9.13–9.14, 9.17–9.20). This plan uses "Phase 8" because roadmap Phase 8 (Cloud & Catalog) is complete and this is the next implementation phase. Task IDs in this plan (8.1–8.18) are plan-local and do not correspond to roadmap task IDs. The roadmap should be updated to split Phase 9 into "Phase 8b: Auth & Identity" (this plan) and "Phase 9: Visibility & Teams" (deferred scope).

---

## Table of Contents

1. [Overview](#1-overview)
2. [Implementation Outcome](#2-implementation-outcome)
3. [Key Decisions](#3-key-decisions)
4. [Architecture](#4-architecture)
5. [PR 1 — Stream A: zen-auth Crate (Library)](#5-pr-1--stream-a-zen-auth-crate-library)
6. [PR 2 — Stream B: CLI Auth Commands + AppContext Wiring](#6-pr-2--stream-b-cli-auth-commands--appcontext-wiring)
7. [PR 3 — Stream C: Turso JWKS Wiring + Token Refresh](#7-pr-3--stream-c-turso-jwks-wiring--token-refresh)
8. [Execution Order](#8-execution-order)
9. [Gotchas & Warnings](#9-gotchas--warnings)
10. [Milestone 8 Validation](#10-milestone-8-validation)
11. [Validation Traceability Matrix](#11-validation-traceability-matrix)
12. [Plan Review — Mismatch Log](#12-plan-review--mismatch-log)

---

## 1. Overview

**Goal**: Wire Clerk-based authentication into the Zenith CLI so that users can authenticate via browser, store credentials securely, and use Clerk JWTs as the Turso auth token (JWKS path). Currently, `AppContext` uses `config.turso.auth_token` (a Platform API token set via env var) for `ZenService::new_synced()`. Phase 8 replaces this with a Clerk JWT obtained through browser login or API key fallback, stored in the OS keychain, validated via JWKS, and refreshed when near-expiry. Unauthenticated users continue in local-only mode with no degradation.

**Crates touched**:
- `zen-auth` — **NEW**: Complete auth crate with claims, JWKS validation, token store, browser flow, API key fallback, refresh logic (~8 modules, ~550 LOC)
- `zen-cli` — **medium**: Add `commands/auth/` module with 4 subcommands, add `Auth` variant to `Commands` enum, wire auth into `AppContext` startup (~200 LOC)
- `zen-db` — **light**: Add `refresh_auth_token()` method to `ZenDb` for token expiry recovery, ~30 LOC
- `zen-core` — **light**: Add `AuthIdentity` struct to a new `identity.rs` module for cross-crate identity passing, ~20 LOC

**Dependency changes needed**:
- New `zen-auth` crate: `zen-core.workspace = true`, `clerk-rs.workspace = true`, `tiny_http.workspace = true`, `open.workspace = true`, `keyring.workspace = true`, `base64.workspace = true`, `reqwest.workspace = true`, `tokio.workspace = true` (all already in workspace Cargo.toml from spike 0.17). Does NOT depend on `zen-config` (config values passed as parameters) or `anyhow` (uses `AuthError` via thiserror).
- `zen-cli`: `zen-auth.workspace = true` added to `[dependencies]`
- Workspace Cargo.toml: `zen-auth = { path = "crates/zen-auth" }` added to `[workspace.dependencies]`

**Estimated deliverables**: ~15 new/modified production files, ~800 LOC production code, ~200 LOC tests

**PR strategy**: 3 PRs by stream. Stream A provides the library foundation. Stream B wires CLI commands and AppContext. Stream C integrates Turso JWKS and token refresh.

| PR | Stream | Contents | Depends On |
|----|--------|----------|------------|
| PR 1 | A: zen-auth Crate | `crates/zen-auth/` (8 modules), `zen-core/src/identity.rs` | None (clean start) |
| PR 2 | B: CLI Auth + AppContext | `commands/auth/`, `cli/root_commands.rs`, `context/app_context.rs` | Stream A |
| PR 3 | C: Turso JWKS + Refresh | `zen-db/src/lib.rs` (refresh), `context/app_context.rs` (JWKS startup) | Streams A + B |

---

## 2. Implementation Outcome

*To be filled after implementation. Follows the same table format as Phase 7 §2.*

---

## 3. Key Decisions

All decisions derive from spike 0.17 findings ([spike_clerk_auth.rs](../../crates/zen-db/src/spike_clerk_auth.rs)), spike 0.20 findings ([spike_catalog_visibility.rs](../../crates/zen-db/src/spike_catalog_visibility.rs)), and Phase 5/7 CLI conventions.

### 3.1 Clerk JWT as Turso Auth Token (JWKS Path)

**Decision**: Replace the current `config.turso.auth_token` (Platform API token) with a Clerk JWT for authenticated users. The Clerk JWT is validated locally via JWKS (clerk-rs `MemoryCacheJwksProvider` + `validate_jwt()`), then passed to `Builder::new_remote_replica()` as the auth token. Turso validates the JWT via its JWKS endpoint — no token minting needed at runtime.

**Rationale**: Spike 0.17 validated that Turso accepts Clerk JWTs directly via JWKS. This eliminates the need for Platform API token minting, provides per-user identity (JWT `sub` claim), and enables org-scoped access (JWT `org_id` claim) that Phase 9 will use for visibility.

**Current state**: `AppContext::init()` passes `config.turso.auth_token` directly to `ZenService::new_synced()`. The Platform API token works but provides no user identity and cannot support multi-user scenarios.

**Migration path**: Platform API tokens (`config.turso.auth_token`) remain supported as tier 4 fallback for CI environments that don't run browser auth. The priority order is: (1) keyring → (2) `ZENITH_AUTH__TOKEN` env var → (3) `~/.zenith/credentials` file → (4) `config.turso.auth_token` from figment. Tiers 1–3 are Clerk JWTs managed by `zen-auth`; tier 4 is a legacy Platform API token handled in `AppContext`. The first available token wins.

**Validated in**: Spike 0.17 tests `test_turso_jwks_accepts_clerk_jwt` and `test_turso_embedded_replica_with_clerk_jwt`.

### 3.2 Four-Tier Token Resolution

**Decision**: Token resolution is split into two layers. The **token store** layer (`token_store::load()`) resolves a Clerk JWT from the first three tiers. The **AppContext** layer adds a legacy fallback as tier 4.

1. **Keyring** — OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service via `keyring` v3)
2. **Environment variable** — `ZENITH_AUTH__TOKEN` (for CI/headless)
3. **File fallback** — `~/.zenith/credentials` (plain text, `0600` permissions, used when keyring is unavailable)
4. **Config file** — `config.turso.auth_token` via figment (legacy Platform API token fallback, handled in `resolve_auth()` in AppContext, NOT in `token_store`)

If no token is found at any tier, the CLI runs in local-only mode (no Turso sync, no cloud search).

**Rationale**: Tiers 1–3 are Clerk JWT sources managed by `zen-auth`. Tier 4 is a backward-compatibility path for existing `.env` files that set `ZENITH_TURSO__AUTH_TOKEN` (a Turso Platform API token, not a Clerk JWT). Tier 4 tokens are NOT validated via JWKS — they're passed through as raw Platform API tokens. This separation keeps `zen-auth` focused on Clerk credentials while `AppContext` owns the legacy fallback.

**Validated in**: Spike 0.17 tests `test_keyring_store_retrieve_delete` and `test_keyring_file_fallback`.

**Note on CI precedence**: The keyring tier wins over `ZENITH_AUTH__TOKEN` (env var). This is intentional for desktop users — the keyring is the primary credential store. In CI environments, there is typically no keyring daemon (no GNOME Keyring, KDE Wallet, or Secret Service), so `keyring::Entry::new()` or `get_password()` fails and the keyring tier naturally falls through to the env var tier. For desktop users who need to override a stale keyring token, `znt auth logout` clears the keyring first.

### 3.3 Browser Login Flow via localhost Callback

**Decision**: `znt auth login` opens the user's browser to the Clerk sign-in page, with a redirect URL pointing to `http://127.0.0.1:{random_port}/callback`. A `tiny_http` server on that port captures the JWT from the redirect, validates it via JWKS, and stores it in the keyring.

**Rationale**: This is the standard OAuth/OIDC CLI pattern (used by `gh auth login`, `gcloud auth login`, `fly auth login`). It works on macOS, Windows, and Linux desktop environments. Spike 0.17 validated the full flow: `tiny_http` on `127.0.0.1:0`, `open` crate opens browser, JWT extracted from redirect URL.

**Clerk sign-in URL**: `https://{clerk_frontend_api}/sign-in?redirect_url=http://127.0.0.1:{port}/callback`. The `clerk_frontend_api` is derived from the JWKS URL hostname (e.g., `ruling-doe-21.clerk.accounts.dev`).

### 3.4 API Key Fallback for CI/Headless

**Decision**: For CI environments where browser login is impossible, `znt auth login --api-key` accepts a Clerk Backend API secret key. It programmatically creates a Clerk session and retrieves a JWT from the `zenith_cli` template (validated in spike 0.20 J0). The JWT is stored in the `ZENITH_AUTH__TOKEN` env var or printed for manual export.

**Rationale**: CI pipelines need auth without a browser. The Clerk Backend API supports programmatic session creation and JWT minting. This was validated in spike 0.20 test `test_programmatic_org_scoped_jwt`.

**Security note**: The API key has full backend access. It should NEVER be stored in the keyring or config file. It's only used transiently to mint a JWT, which is the stored credential.

### 3.5 Token Expiry Detection and Refresh

**Decision**: Before every Turso operation, check the JWT `exp` claim. If the token expires within 60 seconds, trigger a refresh:
- **Interactive mode**: Open browser for re-auth (same as initial login)
- **CI mode**: Fail with a clear error message directing the user to re-run `znt auth login --api-key`

If a Turso operation fails with `Sync("Unauthorized")`, the error is caught, the token is marked as expired, and the same refresh logic triggers.

**Rationale**: Spike 0.17 validated that Turso checks auth at builder time — expired tokens fail immediately with `Sync("Unauthorized")`. The 60-second buffer prevents mid-operation failures. libsql does NOT support hot-swapping auth tokens — the entire `Database` and `Connection` must be recreated with a fresh token.

**Impact on ZenDb**: A new `ZenDb::rebuild_synced()` method (or recreating `ZenService`) is needed when the token refreshes. This is the most invasive change in Phase 8.

**Validated in**: Spike 0.17 test `test_expired_token_behavior`.

### 3.6 No zen-auth Trait — Concrete Types

**Decision**: No `AuthProvider` trait abstraction. `zen-auth` provides concrete functions and structs. There is only one auth backend (Clerk via JWKS). A trait would add indirection without value.

**Rationale**: Same reasoning as Phase 7's decision 3.4 (No Workspace Trait). One backend, no need for abstraction. If a second auth provider is needed later, introduce a trait then.

### 3.7 AuthIdentity in zen-core for Cross-Crate Identity

**Decision**: A lightweight `AuthIdentity` struct in `zen-core/src/identity.rs` carries the authenticated user's identity (`user_id`, `org_id`, `org_slug`, `org_role`). This is the only type that crosses crate boundaries for identity. `zen-auth` produces it; `zen-cli` passes it around; Phase 9 will thread it into `zen-db` repos.

**Rationale**: Putting identity types in zen-core avoids making all downstream crates depend on `zen-auth` (and transitively on `clerk-rs`, `keyring`, etc.). The identity type is data-only — no auth logic, no Clerk SDK calls. Phase 9 will use `AuthIdentity` to scope entity queries by `org_id`.

### 3.8 Graceful Degradation: Auth Failure = Local Mode

**Decision**: If authentication fails at any point (no token, expired token, JWKS validation failure, keyring error), the CLI degrades to local mode. A `tracing::warn!` is emitted, and the user sees the standard local-only behavior (no cloud sync, no catalog queries, no cloud search). Auth commands (`znt auth login/logout/status`) are the exception — they fail with clear error messages.

**Rationale**: Phase 5 established the graceful degradation pattern. A researcher using Zenith locally should never be blocked by auth infrastructure. Auth is opt-in for cloud features.

### 3.9 Auth Commands Are Pre-Context (Like Init, Hook, Schema)

**Decision**: `znt auth login/logout/status/switch-org` are dispatched BEFORE `AppContext::init()`, alongside `init`, `hook`, and `schema` commands. They don't require a `.zenith` project directory.

**Rationale**: A user must be able to log in before initializing any project. Auth commands operate on the global credential store (`~/.zenith/credentials` or OS keychain), not on per-project state.

---

## 4. Architecture

### Module Structure

```
zen-core/src/
└── identity.rs                    # NEW — AuthIdentity struct (data-only)

zen-auth/
├── Cargo.toml                     # NEW — clerk-rs, tiny_http, open, keyring, base64, reqwest
└── src/
    ├── lib.rs                     # NEW — pub API: login(), verify(), resolve_token(), logout()
    ├── claims.rs                  # NEW — ZenClaims wrapping ClerkJwt + ActiveOrganization
    ├── error.rs                   # NEW — AuthError enum
    ├── jwks.rs                    # NEW — JwksValidator (clerk-rs MemoryCacheJwksProvider)
    ├── browser_flow.rs            # NEW — localhost callback: tiny_http + open browser
    ├── api_key.rs                 # NEW — CI fallback: programmatic session + JWT
    ├── token_store.rs             # NEW — keyring primary, file fallback, env var
    └── refresh.rs                 # NEW — Token lifecycle: check expiry, detect near-expiry

zen-cli/src/
├── commands/
│   ├── auth/
│   │   ├── mod.rs                 # NEW — dispatch auth subcommands
│   │   ├── login.rs               # NEW — znt auth login [--api-key]
│   │   ├── logout.rs              # NEW — znt auth logout
│   │   ├── status.rs              # NEW — znt auth status
│   │   └── switch_org.rs          # NEW — znt auth switch-org <org-slug>
│   └── dispatch.rs                # MODIFIED — add Auth to unreachable! arm (pre-context)
├── cli/
│   ├── root_commands.rs           # MODIFIED — add Auth variant + AuthCommands
│   └── subcommands/
│       └── mod.rs                 # MODIFIED — add AuthCommands export
├── context/
│   └── app_context.rs             # MODIFIED — auth-aware startup with token resolution
└── main.rs                        # MODIFIED — dispatch Auth before AppContext::init()

zen-db/src/
└── lib.rs                         # MODIFIED — add rebuild_synced() for token refresh
```

### Upstream Dependencies — All Ready

| Dependency | Method | Crate | Status | Usage |
|------------|--------|-------|--------|-------|
| `clerk-rs::validators::validate_jwt()` | JWKS validation | clerk-rs | **Spike 0.17** | Validate Clerk JWT locally |
| `MemoryCacheJwksProvider::new(clerk)` | JWKS key cache | clerk-rs | **Spike 0.17** | Cache JWKS public keys (takes `Clerk` instance, NOT URL) |
| `tiny_http::Server::http()` | Localhost HTTP server | tiny_http | **Spike 0.17** | Browser callback listener |
| `open::that()` | Open browser | open | **Spike 0.17** | Launch sign-in page |
| `keyring::Entry::new()` | OS keychain CRUD | keyring | **Spike 0.17** | Store/retrieve/delete JWT |
| `base64::engine::general_purpose::STANDARD.decode()` | JWT payload decode | base64 | **Spike 0.17** | Extract `exp` claim without full validation |
| `Builder::new_remote_replica()` with Clerk JWT | Turso JWKS auth | libsql | **Spike 0.17** | Use Clerk JWT as libsql auth token |
| `reqwest::Client::post()` | Clerk Backend API | reqwest | **Spike 0.20** | Programmatic session + JWT minting |

### Data Flow

```
znt auth login
  → browser_flow::login(clerk_frontend_api, secret_key, timeout, org_slug=None)
      → tiny_http::Server::http("127.0.0.1:0") → bind random port
      → open::that(sign_in_url_with_redirect) → browser opens
      → wait_for_callback(server, timeout) → loop ignoring non-callback requests
      → jwks::validate(jwt, secret_key) → ZenClaims { user_id, org_id, org_slug }
      → token_store::store(jwt) → keyring::Entry::set_password()
  → returns AuthLoginResponse { user_id, org_id, org_slug, expires_at }

znt auth logout
  → token_store::delete() → keyring::Entry::delete_credential()
  → returns AuthLogoutResponse { cleared: true }

znt auth status
  → token_store::load() → keyring::Entry::get_password()
  → IF found: refresh::check_expiry(jwt) → near-expiry?
      → jwks::validate(jwt, secret_key) → ZenClaims
      → returns AuthStatusResponse { authenticated: true, user_id, org_id, expires_at }
  → IF not found:
      → returns AuthStatusResponse { authenticated: false }

znt session start (with auth — AppContext::init wiring)
  → resolve_token() priority: keyring → env → config
  → IF token found:
      → refresh::check_expiry(token) → IF near-expiry: warn + refresh
      → ZenService::new_synced(replica_path, turso_url, clerk_jwt)
      → identity = AuthIdentity { user_id, org_id, org_slug, org_role }
  → IF no token:
      → ZenService::new_local(db_path, trail_dir)
      → identity = None

Turso operation fails with Sync("Unauthorized")
  → zen-db catches error
  → tracing::warn!("turso auth token may be expired")
  → IF interactive: prompt re-auth
  → IF CI: fail with clear error message
```

---

## 5. PR 1 — Stream A: zen-auth Crate (Library)

**Tasks**: 8.1–8.7
**Estimated LOC**: ~550 production, ~150 tests

### A1. `zen-core/src/identity.rs` — AuthIdentity (task 8.1)

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Lightweight authenticated user identity for cross-crate passing.
///
/// Produced by `zen-auth`, consumed by `zen-cli` and (in Phase 9) `zen-db`.
/// Contains only data fields — no auth logic, no Clerk SDK calls.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuthIdentity {
    /// Clerk user ID (from JWT `sub` claim).
    pub user_id: String,
    /// Clerk organization ID (from JWT `org_id` claim). `None` = personal mode.
    pub org_id: Option<String>,
    /// Clerk organization slug (from JWT `org_slug` claim).
    pub org_slug: Option<String>,
    /// Clerk organization role (from JWT `org_role` claim, e.g. `"org:admin"`).
    pub org_role: Option<String>,
}
```

Update `zen-core/src/lib.rs` to add `pub mod identity;`.

### A2. `zen-auth/Cargo.toml` — Crate Setup (task 8.2)

```toml
[package]
name = "zen-auth"
description = "Clerk authentication, JWKS validation, and token management for Zenith"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
zen-core.workspace = true
clerk-rs.workspace = true
tiny_http.workspace = true
open.workspace = true
keyring.workspace = true
base64.workspace = true
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
chrono.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true
urlencoding.workspace = true

[dev-dependencies]
pretty_assertions.workspace = true
tempfile.workspace = true
dotenvy.workspace = true

[lints]
workspace = true
```

### A3. `zen-auth/src/error.rs` — AuthError Enum (task 8.3)

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("not authenticated — run `znt auth login`")]
    NotAuthenticated,

    #[error("token expired — run `znt auth login` to refresh")]
    TokenExpired,

    #[error("JWKS validation failed: {0}")]
    JwksValidation(String),

    #[error("keyring error: {0}")]
    KeyringError(String),

    #[error("browser login failed: {0}")]
    BrowserFlowFailed(String),

    #[error("API key auth failed: {0}")]
    ApiKeyFailed(String),

    #[error("token store error: {0}")]
    TokenStoreError(String),

    #[error("clerk API error: {0}")]
    ClerkApiError(String),

    #[error("{0}")]
    Other(String),
}
```

### A4. `zen-auth/src/claims.rs` — ZenClaims (task 8.4)

```rust
use chrono::{DateTime, Utc};
use zen_core::identity::AuthIdentity;

/// Parsed and validated Clerk JWT claims.
///
/// Wraps the relevant fields from `clerk-rs::ClerkJwt` into a Zenith-specific
/// struct. Produced by JWKS validation, consumed by CLI commands and AppContext.
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
```

### A5. `zen-auth/src/jwks.rs` — JWKS Validation (task 8.5)

Wraps `clerk-rs` JWKS validation into a single `validate()` function.

**Key implementation patterns:**

1. **JWKS provider**: `MemoryCacheJwksProvider::new(clerk)` where `clerk = Clerk::new(ClerkConfiguration::new(None, None, Some(secret_key), None))` — caches public keys in memory for 1 hour. The `Clerk` instance uses the Clerk Backend API to fetch JWKS from `{base_path}/jwks`, NOT from the `.well-known/jwks.json` OIDC endpoint.
2. **Validation**: `validate_jwt(jwt, provider).await?` — returns `ClerkJwt`. The provider must be `Arc<impl JwksProvider>`.
3. **Claim extraction**: `clerk_jwt.sub` for user_id, `clerk_jwt.org` for `ActiveOrganization { id, slug, role, permissions }`.
4. **Expiry extraction**: `clerk_jwt.exp` is `i32` in clerk-rs — convert via `i64::from(clerk_jwt.exp)` → `DateTime::from_timestamp()`. No separate base64 decode needed after validation.
5. **Provider caching**: The `MemoryCacheJwksProvider` should be created ONCE per process and reused. Creating it per call defeats the 1-hour key cache. Implementation: `OnceLock<Arc<MemoryCacheJwksProvider>>` or stored in AppContext.

```rust
use std::sync::{Arc, OnceLock};
use clerk_rs::ClerkConfiguration;
use clerk_rs::clerk::Clerk;
use clerk_rs::validators::authorizer::validate_jwt;
use clerk_rs::validators::jwks::MemoryCacheJwksProvider;

/// Process-scoped JWKS provider cache.
/// Created on first use, reused for all subsequent validations.
static JWKS_PROVIDER: OnceLock<Arc<MemoryCacheJwksProvider>> = OnceLock::new();

fn get_or_init_provider(secret_key: &str) -> Arc<MemoryCacheJwksProvider> {
    JWKS_PROVIDER
        .get_or_init(|| {
            let config = ClerkConfiguration::new(None, None, Some(secret_key.to_string()), None);
            let clerk = Clerk::new(config);
            Arc::new(MemoryCacheJwksProvider::new(clerk))
        })
        .clone()
}

pub async fn validate(jwt: &str, secret_key: &str) -> Result<ZenClaims, AuthError> {
    let provider = get_or_init_provider(secret_key);
    let clerk_jwt = validate_jwt(jwt, provider)
        .await
        .map_err(|e| AuthError::JwksValidation(e.to_string()))?;

    let expires_at = chrono::DateTime::from_timestamp(i64::from(clerk_jwt.exp), 0)
        .ok_or_else(|| AuthError::JwksValidation("invalid exp timestamp".into()))?;
    let org = clerk_jwt.org.as_ref();

    Ok(ZenClaims {
        raw_jwt: jwt.to_string(),
        user_id: clerk_jwt.sub.clone(),
        org_id: org.map(|o| o.id.clone()),
        org_slug: org.map(|o| o.slug.clone()),
        org_role: org.map(|o| o.role.clone()),
        expires_at,
    })
}
```

**Important**: `validate()` accepts `secret_key` (not `jwks_url`). clerk-rs uses the secret key to construct its internal Clerk client, which fetches JWKS from the Clerk Backend API endpoint. The `config.clerk.jwks_url` field is retained only for `resolve_frontend_api()` hostname extraction (Gotcha 9.2). All callers in the plan that previously passed `jwks_url` now pass `config.clerk.secret_key`.

### A6. `zen-auth/src/token_store.rs` — Credential Storage (task 8.6)

Four functions (3 public, 1 internal detect) + 2 private file helpers:

```rust
use std::fs;
use std::path::PathBuf;

const KEYRING_SERVICE: &str = "zenith-cli";
const KEYRING_USER: &str = "clerk-jwt";
const CREDENTIALS_FILE_NAME: &str = "credentials";

/// Store a JWT in the OS keychain. Falls back to file if keyring unavailable.
pub fn store(jwt: &str) -> Result<(), AuthError> {
    match keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        Ok(entry) => match entry.set_password(jwt) {
            Ok(()) => Ok(()),
            Err(error) => {
                tracing::warn!(%error, "keyring store failed; falling back to file");
                store_file(jwt)
            }
        },
        Err(error) => {
            tracing::warn!(%error, "keyring unavailable; falling back to file");
            store_file(jwt)
        }
    }
}

/// Load a JWT. Priority: keyring → ZENITH_AUTH__TOKEN env → file (~/.zenith/credentials).
pub fn load() -> Option<String> {
    // 1. Keyring
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        && let Ok(token) = entry.get_password()
        && !token.is_empty()
    {
        return Some(token);
    }

    // 2. Environment variable
    if let Ok(token) = std::env::var("ZENITH_AUTH__TOKEN") {
        if !token.is_empty() {
            return Some(token);
        }
    }

    // 3. File fallback
    load_file()
}

/// Delete stored credentials from keyring and file.
pub fn delete() -> Result<(), AuthError> {
    // Delete from keyring (ignore errors — may not exist)
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER) {
        let _ = entry.delete_credential();
    }

    // Delete credentials file
    let path = credentials_path()?;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| AuthError::TokenStoreError(format!("failed to delete {}: {e}", path.display())))?;
    }

    Ok(())
}

/// Detect which tier the current token came from (for status display).
pub fn detect_token_source() -> Option<String> {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        && entry.get_password().is_ok()
    {
        return Some("keyring".into());
    }
    if std::env::var("ZENITH_AUTH__TOKEN").is_ok_and(|t| !t.is_empty()) {
        return Some("env".into());
    }
    if credentials_path().is_ok_and(|p| p.exists()) {
        return Some("file".into());
    }
    None
}

// --- Private file helpers ---

fn credentials_path() -> Result<PathBuf, AuthError> {
    dirs::home_dir()
        .map(|h| h.join(".zenith").join(CREDENTIALS_FILE_NAME))
        .ok_or_else(|| AuthError::TokenStoreError("home directory not found — cannot store credentials".into()))
}

fn store_file(jwt: &str) -> Result<(), AuthError> {
    let path = credentials_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AuthError::TokenStoreError(format!("mkdir {}: {e}", parent.display())))?;
    }
    fs::write(&path, jwt)
        .map_err(|e| AuthError::TokenStoreError(format!("write {}: {e}", path.display())))?;

    // Set 0600 permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| AuthError::TokenStoreError(format!("chmod {}: {e}", path.display())))?;
    }

    Ok(())
}

fn load_file() -> Option<String> {
    let path = credentials_path().ok()?;
    fs::read_to_string(&path).ok().filter(|s| !s.trim().is_empty())
}
```

**File fallback path**: `~/.zenith/credentials` (plain text, `0600` permissions on Unix). Validated in spike 0.17 test `test_keyring_file_fallback`.

### A7. `zen-auth/src/browser_flow.rs` — Browser Login (task 8.7)

```rust
use std::io::Read as _;

/// Execute the browser-based Clerk login flow.
///
/// 1. Start tiny_http on 127.0.0.1:0 (random port)
/// 2. Open browser to Clerk sign-in with redirect to localhost
/// 3. Wait for callback with JWT (in spawn_blocking — tiny_http::recv blocks)
/// 4. Validate JWT via JWKS
/// 5. Store in keyring
pub async fn login(
    clerk_frontend_api: &str,
    secret_key: &str,
    timeout: std::time::Duration,
    org_slug: Option<&str>,
) -> Result<ZenClaims, AuthError> {
    let server = tiny_http::Server::http("127.0.0.1:0")
        .map_err(|e| AuthError::BrowserFlowFailed(format!("failed to bind: {e}")))?;
    let port = server.server_addr().to_ip().map(|a| a.port())
        .ok_or_else(|| AuthError::BrowserFlowFailed("no port".into()))?;

    let redirect_url = format!("http://127.0.0.1:{port}/callback");
    let mut sign_in_url = format!(
        "https://{clerk_frontend_api}/sign-in?redirect_url={redirect}",
        redirect = urlencoding::encode(&redirect_url)
    );
    if let Some(org) = org_slug {
        sign_in_url.push_str(&format!("&organization={}", urlencoding::encode(org)));
    }

    // Print URL to stderr for headless environments that can't open a browser
    eprintln!("Opening browser to: {sign_in_url}");
    if let Err(error) = open::that(&sign_in_url) {
        eprintln!("Failed to open browser: {error}");
        eprintln!("Open the URL above manually, then return here.");
    }

    // Wait for callback — tiny_http::recv() blocks, so run in spawn_blocking
    let jwt = tokio::task::spawn_blocking(move || wait_for_callback(server, timeout))
        .await
        .map_err(|e| AuthError::BrowserFlowFailed(format!("spawn_blocking join: {e}")))?
        ?;

    let claims = crate::jwks::validate(&jwt, secret_key).await?;
    crate::token_store::store(&jwt)?;
    Ok(claims)
}

/// Block until the callback server receives a request with a JWT.
///
/// Loops on `recv_timeout()`, ignoring requests that don't match `/callback?token=`.
/// This handles browser favicon requests, preflight requests, and user refreshes
/// that would otherwise cause a false failure.
/// Times out after `timeout` duration.
fn wait_for_callback(
    server: tiny_http::Server,
    timeout: std::time::Duration,
) -> Result<String, AuthError> {
    let deadline = std::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(AuthError::BrowserFlowFailed(
                format!("browser callback timed out after {}s", timeout.as_secs())
            ));
        }

        let request = match server.recv_timeout(remaining) {
            Ok(Some(req)) => req,
            Ok(None) => return Err(AuthError::BrowserFlowFailed(
                format!("browser callback timed out after {}s", timeout.as_secs())
            )),
            Err(e) => return Err(AuthError::BrowserFlowFailed(format!("recv error: {e}"))),
        };

        let url = request.url().to_string();

        // Ignore requests that aren't the callback (e.g., favicon, preflight)
        if !url.starts_with("/callback?") {
            let response = tiny_http::Response::from_string("")
                .with_status_code(204);
            let _ = request.respond(response);
            continue;
        }

        // Respond to the browser with a success page
        let response = tiny_http::Response::from_string(
            "<html><body><h1>Authenticated!</h1><p>You can close this tab.</p></body></html>"
        ).with_header(
            tiny_http::Header::from_bytes("Content-Type", "text/html").unwrap()
        );
        let _ = request.respond(response);

        // Extract token from query string: /callback?token=<jwt>
        let query = url.split('?').nth(1)
            .ok_or_else(|| AuthError::BrowserFlowFailed("no query string in callback".into()))?;
        for pair in query.split('&') {
            if let Some(value) = pair.strip_prefix("token=") {
                let jwt = urlencoding::decode(value)
                    .map_err(|e| AuthError::BrowserFlowFailed(format!("URL decode: {e}")))?;
                return Ok(jwt.into_owned());
            }
        }

        return Err(AuthError::BrowserFlowFailed("callback missing 'token' parameter".into()));
    }
}
```

### A8. `zen-auth/src/api_key.rs` — CI/Headless Fallback (task 8.8)

```rust
/// Programmatic JWT generation via Clerk Backend API.
///
/// Creates a Clerk session and retrieves a JWT from the `zenith_cli` template.
/// For CI/headless environments where browser login is impossible.
pub async fn login_with_api_key(
    secret_key: &str,
    user_id: &str,
) -> Result<ZenClaims, AuthError> {
    let client = reqwest::Client::new();

    // 1. Create session
    let session = client
        .post("https://api.clerk.com/v1/sessions")
        .header("Authorization", format!("Bearer {secret_key}"))
        .json(&serde_json::json!({"user_id": user_id}))
        .send().await
        .map_err(|e| AuthError::ApiKeyFailed(format!("create session: {e}")))?
        .json::<serde_json::Value>().await
        .map_err(|e| AuthError::ApiKeyFailed(format!("parse session: {e}")))?;

    let session_id = session["id"].as_str()
        .ok_or_else(|| AuthError::ApiKeyFailed("session response missing 'id'".into()))?;

    // 2. Get JWT from zenith_cli template
    let token = client
        .post(format!("https://api.clerk.com/v1/sessions/{session_id}/tokens/zenith_cli"))
        .header("Authorization", format!("Bearer {secret_key}"))
        .send().await
        .map_err(|e| AuthError::ApiKeyFailed(format!("get token: {e}")))?
        .json::<serde_json::Value>().await
        .map_err(|e| AuthError::ApiKeyFailed(format!("parse token: {e}")))?;

    let jwt = token["jwt"].as_str()
        .ok_or_else(|| AuthError::ApiKeyFailed("token response missing 'jwt'".into()))?;

    let claims = crate::jwks::validate(jwt, secret_key).await?;
    crate::token_store::store(jwt)?;
    Ok(claims)
}
```

### A9. `zen-auth/src/refresh.rs` — Token Lifecycle (task 8.9)

```rust
const EXPIRY_BUFFER_SECS: i64 = 60;

/// Check if a stored token is still valid.
///
/// Returns `Some(claims)` if the token is valid and not near-expiry.
/// Returns `None` if the token is expired or near-expiry.
pub async fn check_stored_token(secret_key: &str) -> Result<Option<ZenClaims>, AuthError> {
    let Some(jwt) = crate::token_store::load() else {
        return Ok(None);
    };

    let claims = crate::jwks::validate(&jwt, secret_key).await?;
    if claims.is_near_expiry(EXPIRY_BUFFER_SECS) {
        tracing::warn!(
            expires_at = %claims.expires_at,
            "auth token expires within {}s — re-authenticate with `znt auth login`",
            EXPIRY_BUFFER_SECS
        );
        return Ok(None);
    }

    Ok(Some(claims))
}

/// Decode JWT `exp` claim without full validation (for quick expiry checks).
pub fn decode_expiry(jwt: &str) -> Result<chrono::DateTime<chrono::Utc>, AuthError> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err(AuthError::Other("invalid JWT format".into()));
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| AuthError::Other(format!("base64 decode failed: {e}")))?;
    let value: serde_json::Value = serde_json::from_slice(&payload)
        .map_err(|e| AuthError::Other(format!("JSON parse failed: {e}")))?;
    let exp = value["exp"].as_i64()
        .ok_or_else(|| AuthError::Other("missing exp claim".into()))?;
    chrono::DateTime::from_timestamp(exp, 0)
        .ok_or_else(|| AuthError::Other("invalid exp timestamp".into()))
}
```

### A10. `zen-auth/src/lib.rs` — Public API (task 8.10)

```rust
//! # zen-auth
//!
//! Clerk-based authentication for Zenith CLI.
//!
//! Provides browser login (tiny_http + open), JWKS JWT validation (clerk-rs),
//! OS keychain token storage (keyring), API key fallback for CI, and token
//! lifecycle management.

pub mod api_key;
pub mod browser_flow;
pub mod claims;
pub mod error;
pub mod jwks;
pub mod refresh;
pub mod token_store;

pub use claims::ZenClaims;
pub use error::AuthError;

/// Resolve the best available auth token.
///
/// Priority: keyring → env var → None.
/// Does NOT validate the token (use `refresh::check_stored_token()` for validation).
pub fn resolve_token() -> Option<String> {
    token_store::load()
}

/// Full token resolution with JWKS validation.
///
/// Returns validated claims if a token exists and is valid + not near-expiry.
pub async fn resolve_and_validate(secret_key: &str) -> Result<Option<ZenClaims>, AuthError> {
    refresh::check_stored_token(secret_key).await
}

/// Clear stored credentials.
pub fn logout() -> Result<(), AuthError> {
    token_store::delete()
}
```

---

## 6. PR 2 — Stream B: CLI Auth Commands + AppContext Wiring

**Tasks**: 8.11–8.16
**Estimated LOC**: ~200 production, ~50 tests

### B1. `cli/root_commands.rs` — Add Auth Command (task 8.11)

Add to the `Commands` enum:

```rust
/// Authentication.
Auth {
    #[command(subcommand)]
    action: AuthCommands,
},
```

### B2. `cli/subcommands/` — AuthCommands (task 8.12)

```rust
#[derive(Clone, Debug, Subcommand)]
pub enum AuthCommands {
    /// Log in via browser (or --api-key for CI).
    Login(AuthLoginArgs),
    /// Clear stored credentials.
    Logout,
    /// Show current auth status.
    Status,
    /// Switch to a different Clerk organization.
    SwitchOrg(AuthSwitchOrgArgs),
}

#[derive(Clone, Debug, Args)]
pub struct AuthLoginArgs {
    /// Use API key for CI/headless auth instead of browser.
    #[arg(long)]
    pub api_key: bool,
    /// Clerk Backend API secret key (required with --api-key).
    #[arg(long, env = "ZENITH_CLERK__SECRET_KEY")]
    pub secret_key: Option<String>,
    /// Clerk user ID (required with --api-key).
    #[arg(long)]
    pub user_id: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct AuthSwitchOrgArgs {
    /// Organization slug to switch to.
    pub org_slug: String,
}
```

### B3. `commands/auth/login.rs` — Login Handler (task 8.13)

```rust
pub async fn handle(args: &AuthLoginArgs, flags: &GlobalFlags) -> anyhow::Result<()> {
    let config = zen_config::ZenConfig::load().map_err(anyhow::Error::from)?;
    let secret_key_cfg = config.clerk.secret_key.as_str();

    if secret_key_cfg.is_empty() {
        anyhow::bail!("auth login: ZENITH_CLERK__SECRET_KEY is not configured");
    }

    let claims = if args.api_key {
        let secret_key = args.secret_key.as_deref()
            .unwrap_or(secret_key_cfg);
        let user_id = args.user_id.as_deref()
            .ok_or_else(|| anyhow::anyhow!("auth login --api-key requires --user-id"))?;
        zen_auth::api_key::login_with_api_key(secret_key, user_id).await?
    } else {
        let frontend_api = resolve_frontend_api(&config)?;
        zen_auth::browser_flow::login(
            &frontend_api,
            secret_key_cfg,
            std::time::Duration::from_secs(120),
            None,
        ).await?
    };

    output(&AuthLoginResponse {
        authenticated: true,
        user_id: claims.user_id,
        org_id: claims.org_id,
        org_slug: claims.org_slug,
        expires_at: claims.expires_at.to_rfc3339(),
    }, flags.format)
}

/// Resolve the Clerk frontend API hostname.
///
/// Priority: config.clerk.frontend_url (ZENITH_CLERK__FRONTEND_URL) → extract from JWKS URL hostname.
fn resolve_frontend_api(config: &zen_config::ZenConfig) -> anyhow::Result<String> {
    if !config.clerk.frontend_url.is_empty() {
        return Ok(config.clerk.frontend_url.clone());
    }

    // Fallback: extract hostname from JWKS URL
    // e.g., https://ruling-doe-21.clerk.accounts.dev/.well-known/jwks.json → ruling-doe-21.clerk.accounts.dev
    let jwks_url = &config.clerk.jwks_url;
    let url = url::Url::parse(jwks_url)
        .map_err(|e| anyhow::anyhow!("invalid JWKS URL: {e}"))?;
    url.host_str()
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("JWKS URL has no hostname — set ZENITH_CLERK__FRONTEND_URL"))
}
```

**Note**: `resolve_frontend_api` uses `config.clerk.frontend_url` (already in `ClerkConfig`) as the primary source, falling back to JWKS URL hostname extraction. No new config field or env var needed.

### B4. `commands/auth/status.rs` — Status Handler (task 8.14)

```rust
pub async fn handle(flags: &GlobalFlags) -> anyhow::Result<()> {
    let config = zen_config::ZenConfig::load().map_err(anyhow::Error::from)?;
    let secret_key = config.clerk.secret_key.as_str();

    let status = if secret_key.is_empty() {
        AuthStatusResponse { authenticated: false, user_id: None, org_id: None,
            org_slug: None, expires_at: None, token_source: None, note: Some("ZENITH_CLERK__SECRET_KEY not configured".into()) }
    } else {
        match zen_auth::resolve_and_validate(secret_key).await {
            Ok(Some(claims)) => AuthStatusResponse {
                authenticated: true,
                user_id: Some(claims.user_id),
                org_id: claims.org_id,
                org_slug: claims.org_slug,
                expires_at: Some(claims.expires_at.to_rfc3339()),
                token_source: zen_auth::token_store::detect_token_source(),
                note: None,
            },
            Ok(None) => AuthStatusResponse {
                authenticated: false, user_id: None, org_id: None, org_slug: None,
                expires_at: None, token_source: None, note: Some("no valid token found".into()),
            },
            Err(error) => AuthStatusResponse {
                authenticated: false, user_id: None, org_id: None, org_slug: None,
                expires_at: None, token_source: None, note: Some(error.to_string()),
            },
        }
    };
    output(&status, flags.format)
}
```

### B5. `commands/auth/switch_org.rs` — Switch Org Handler (task 8.14b)

```rust
pub async fn handle(args: &AuthSwitchOrgArgs, flags: &GlobalFlags) -> anyhow::Result<()> {
    let config = zen_config::ZenConfig::load().map_err(anyhow::Error::from)?;
    let secret_key = config.clerk.secret_key.as_str();

    if secret_key.is_empty() {
        anyhow::bail!("auth switch-org: ZENITH_CLERK__SECRET_KEY is not configured");
    }

    let frontend_api = resolve_frontend_api(&config)?;

    // Clear existing credentials before re-auth
    zen_auth::logout().ok();

    // Re-authenticate via browser with the target org.
    // browser_flow::login() includes &organization={slug} in the sign-in URL
    // when org_slug is Some, scoping the Clerk session to that org.
    let claims = zen_auth::browser_flow::login(
        &frontend_api,
        secret_key,
        std::time::Duration::from_secs(120),
        Some(&args.org_slug),
    ).await?;

    if claims.org_slug.as_deref() != Some(&args.org_slug) {
        tracing::warn!(
            expected = %args.org_slug,
            actual = ?claims.org_slug,
            "org slug in JWT does not match requested org"
        );
    }

    output(&AuthSwitchOrgResponse {
        switched: true,
        org_id: claims.org_id,
        org_slug: claims.org_slug,
        org_role: claims.org_role,
    }, flags.format)
}
```

**Key behavior**: `switch-org` clears the current credential and triggers a fresh browser login. The Clerk sign-in page will prompt the user to select the target org. After authentication, the JWT includes the new org's claims. If the JWT's `org_slug` doesn't match the requested slug (e.g., user selected a different org), a warning is emitted.

### B6. `main.rs` — Pre-Context Auth Dispatch (task 8.15)

```rust
match &cli.command {
    cli::Commands::Init(args) => return commands::init::handle(args, &flags).await,
    cli::Commands::Hook { action } => return commands::hook::handle(action, &flags).await,
    cli::Commands::Schema(args) => return commands::schema::handle(args, &flags),
    cli::Commands::Auth { action } => return commands::auth::handle(action, &flags).await,
    _ => {}
}
```

### B7. `context/app_context.rs` — Auth-Aware Startup (task 8.16)

```rust
pub struct AppContext {
    pub service: ZenService,
    pub config: ZenConfig,
    pub lake: ZenLake,
    pub source_store: SourceFileStore,
    pub embedder: EmbeddingEngine,
    pub registry: RegistryClient,
    pub project_root: PathBuf,
    pub identity: Option<AuthIdentity>,  // NEW
}

impl AppContext {
    pub async fn init(project_root: PathBuf, config: ZenConfig) -> anyhow::Result<Self> {
        // Existing path setup (unchanged from current source):
        let zenith_dir = project_root.join(".zenith");
        let db_path = zenith_dir.join("zenith.db");
        let synced_path = zenith_dir.join("zenith-synced.db");
        let trail_dir = zenith_dir.join("trail");
        let lake_path = zenith_dir.join("lake.duckdb");
        let source_path = zenith_dir.join("source_files.duckdb");
        let db_path_str = db_path.to_string_lossy();
        let synced_path_str = synced_path.to_string_lossy();
        let lake_path_str = lake_path.to_string_lossy();
        let source_path_str = source_path.to_string_lossy();

        // NEW: Resolve auth token (tiers 1-3 via zen-auth, tier 4 via config fallback)
        let (auth_token, identity) = resolve_auth(&config).await;

        let service = if config.turso.is_configured() {
            // Existing replica path logic (unchanged)
            let replica_path: &str = if config.turso.has_local_replica() {
                &config.turso.local_replica_path
            } else {
                &synced_path_str
            };

            // NEW: Use auth-resolved token, fall back to config.turso.auth_token (tier 4)
            let token = auth_token
                .as_deref()
                .unwrap_or(&config.turso.auth_token);

            if token.is_empty() {
                ZenService::new_local(&db_path_str, Some(trail_dir))
                    .await
                    .context("failed to initialize zen-db service")?
            } else {
                match ZenService::new_synced(replica_path, &config.turso.url, token, Some(trail_dir.clone())).await {
                    Ok(service) => service,
                    Err(_error) => {
                        tracing::warn!("failed to initialize synced zen-db service; falling back to local");
                        ZenService::new_local(&db_path_str, Some(trail_dir))
                            .await
                            .context("failed to initialize zen-db service")?
                    }
                }
            }
        } else {
            ZenService::new_local(&db_path_str, Some(trail_dir))
                .await
                .context("failed to initialize zen-db service")?
        };

        // Existing lake/source/embedder/registry setup (unchanged)
        let lake = ZenLake::open_local(&lake_path_str).context("failed to open local zen lake")?;
        let source_store = SourceFileStore::open(&source_path_str).context("failed to open source file store")?;
        let embedder = EmbeddingEngine::new().context("failed to initialize embedding engine")?;
        let registry = RegistryClient::new();

        Ok(Self { service, config, lake, source_store, embedder, registry, project_root, identity })
    }
}

/// Resolve auth token with optional JWKS validation.
async fn resolve_auth(config: &ZenConfig) -> (Option<String>, Option<AuthIdentity>) {
    let secret_key = &config.clerk.secret_key;
    if secret_key.is_empty() {
        // No Clerk secret key configured — try raw token from zen-auth tiers or config fallback.
        // Cannot validate via JWKS, so identity is always None.
        let raw = zen_auth::resolve_token()
            .or_else(|| {
                let t = &config.turso.auth_token;
                if t.is_empty() { None } else { Some(t.clone()) }
            });

        // Best-effort expiry check on unverified token (S5 fix)
        if let Some(ref token) = raw {
            match zen_auth::refresh::decode_expiry(token) {
                Ok(expires_at) if expires_at <= chrono::Utc::now() => {
                    tracing::warn!("auth token appears expired — running in local mode");
                    return (None, None);
                }
                Ok(_) => {
                    tracing::warn!(
                        "token found but ZENITH_CLERK__SECRET_KEY not configured — \
                         identity unavailable, expiry checks are best-effort"
                    );
                }
                Err(_) => {} // Not a JWT format — pass through as-is (e.g., Platform API token)
            }
        }

        return (raw, None);
    }

    match zen_auth::resolve_and_validate(secret_key).await {
        Ok(Some(claims)) => {
            let identity = claims.to_identity();
            (Some(claims.raw_jwt), Some(identity))
        }
        Ok(None) => {
            tracing::info!("no valid auth token found; running in local mode");
            (None, None)
        }
        Err(error) => {
            tracing::warn!(%error, "auth token validation failed; running in local mode");
            (None, None)
        }
    }
}
```

---

## 7. PR 3 — Stream C: Turso JWKS Wiring + Token Refresh

**Tasks**: 8.17–8.18
**Estimated LOC**: ~80 production

### C1. `zen-db/src/lib.rs` — Rebuild Synced Connection (task 8.17)

```rust
impl ZenDb {
    /// Rebuild the synced connection with a fresh auth token.
    ///
    /// Called when the current token expires and a new one is obtained.
    /// Drops the current connection and creates a new embedded replica.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the new replica cannot be opened or synced.
    pub async fn rebuild_synced(
        &mut self,
        local_replica_path: &str,
        remote_url: &str,
        new_auth_token: &str,
    ) -> Result<(), DatabaseError> {
        let db = Builder::new_remote_replica(
            local_replica_path.to_string(),
            remote_url.to_string(),
            new_auth_token.to_string(),
        )
        .read_your_writes(true)
        .build()
        .await?;
        db.sync().await?;

        let conn = db.connect()?;
        conn.execute("PRAGMA foreign_keys = ON", ())
            .await
            .map_err(|e| DatabaseError::Migration(format!("PRAGMA foreign_keys: {e}")))?;

        self.db = db;
        self.conn = conn;
        self.is_synced_replica = true;
        Ok(())
    }
}
```

### C2. `zen-db/src/service.rs` — Service-Level Token Refresh (task 8.18)

```rust
impl ZenService {
    /// Rebuild the underlying synced connection with a fresh auth token.
    ///
    /// # Errors
    ///
    /// Returns `DatabaseError` if the rebuild fails.
    pub async fn rebuild_with_token(
        &mut self,
        local_replica_path: &str,
        remote_url: &str,
        new_auth_token: &str,
    ) -> Result<(), DatabaseError> {
        self.db
            .rebuild_synced(local_replica_path, remote_url, new_auth_token)
            .await
    }
}
```

**Key behavior**: `rebuild_synced()` replaces `self.db` and `self.conn` in-place. The trail writer is unaffected (it writes to the filesystem, not the database connection). Migrations are NOT re-run — the local replica already has the schema from the initial open.

**Caller site**: Phase 8 provides `rebuild_with_token()` as an API, but does NOT wire automatic token refresh into the command dispatch loop. The intended caller is `AppContext` or a future error-recovery middleware that catches `Sync("Unauthorized")` errors from Turso operations and triggers `ctx.service.rebuild_with_token(...)` with a freshly obtained token. Full integration into the error recovery path should be validated during PR 3 integration testing. Manual recovery is possible via `znt auth login` followed by re-running the failed command.

---

## 8. Execution Order

```
PR 1 (Stream A):
  1. Add zen-core/src/identity.rs (AuthIdentity type)
  2. Update zen-core/src/lib.rs (add pub mod identity)
  3. Create zen-auth crate scaffold (Cargo.toml, lib.rs)
  4. Add zen-auth to workspace Cargo.toml
  5. Implement error.rs (AuthError enum)
  6. Implement claims.rs (ZenClaims)
  7. Implement jwks.rs (JWKS validation wrapper)
  8. Implement token_store.rs (keyring + file fallback)
  9. Implement browser_flow.rs (localhost callback)
  10. Implement api_key.rs (Clerk Backend API fallback)
  11. Implement refresh.rs (expiry check + decode)
  12. Wire lib.rs public API
  13. Verify: cargo build -p zen-core -p zen-auth

PR 2 (Stream B):
  14. Add AuthCommands to cli/subcommands
  15. Add Auth variant to Commands enum
  16. Add commands/auth/ module (login, logout, status, switch_org)
  17. Add Auth pre-context dispatch to main.rs + Auth to unreachable! arm in dispatch.rs
  18. Add identity field to AppContext
  19. Wire auth-aware token resolution (resolve_auth + four-tier) into AppContext::init()
  20. Add zen-auth to zen-cli/Cargo.toml
  21. Verify: cargo build -p zen-cli

PR 3 (Stream C):
  22. Add rebuild_synced() to ZenDb
  23. Add rebuild_with_token() to ZenService
  24. Verify: cargo test -p zen-db -p zen-cli -p zen-auth
```

---

## 9. Gotchas & Warnings

### 9.1 clerk-rs API Surface Is Minimal

**Problem**: `clerk-rs` provides JWKS validation (`validate_jwt`) and claims types (`ClerkJwt`, `ActiveOrganization`), but does NOT provide a Clerk Backend API client. Session creation, JWT template minting, and org management use raw `reqwest` calls to `https://api.clerk.com/v1/...`.

**Impact**: The `api_key.rs` module and any future team management commands must use raw HTTP calls, not clerk-rs helpers.

**Resolution**: This is the same pattern validated in spike 0.20. The Clerk Backend API is RESTful and well-documented. No wrapper library needed for the 2-3 endpoints we call.

### 9.2 Clerk JWKS URL Hostname ≠ Frontend API Hostname

**Problem**: The JWKS URL is `https://ruling-doe-21.clerk.accounts.dev/.well-known/jwks.json`, but the sign-in page URL is `https://ruling-doe-21.clerk.accounts.dev/sign-in`. The hostname is the same, but this is a Clerk implementation detail — other Clerk instances may use different hostname patterns.

**Impact**: `browser_flow.rs` needs the frontend API hostname to construct the sign-in URL. Extracting it from the JWKS URL works for standard Clerk instances but may break for custom domains.

**Resolution**: Use `config.clerk.frontend_url` (`ZENITH_CLERK__FRONTEND_URL`, already in `ClerkConfig`) as the primary source. Fall back to extracting the hostname from the JWKS URL only when `frontend_url` is empty. See `resolve_frontend_api()` in B3. No new config field needed — `ClerkConfig` already has `frontend_url: String`.

### 9.3 `keyring` v3 Requires Secret Service on Linux

**Problem**: On headless Linux (servers, CI, Docker containers), there's no Secret Service daemon (GNOME Keyring, KDE Wallet). `keyring::Entry::new()` succeeds, but `set_password()` fails with a D-Bus connection error.

**Impact**: `token_store::store()` must handle this gracefully and fall back to file storage.

**Resolution**: `store()` catches keyring errors and falls back to `~/.zenith/credentials` with `0600` permissions. Validated in spike 0.17. The file fallback is documented in `znt auth login` output.

### 9.4 libsql Does NOT Support Hot-Swapping Auth Tokens

**Problem**: Once `Builder::new_remote_replica().build().await` is called, the auth token is baked into the `Database` instance. There's no `set_token()` or `refresh_token()` method. To use a new token, the entire `Database` + `Connection` must be recreated.

**Impact**: Token refresh requires `ZenDb::rebuild_synced()` which drops and recreates the connection. Any in-flight operations on the old connection will fail. The trail writer is unaffected (filesystem-based).

**Resolution**: `rebuild_synced()` replaces `self.db` and `self.conn` in-place. Callers (AppContext) should only trigger rebuild between CLI commands, never mid-operation. Validated in spike 0.17 test `test_no_hot_swap_auth_tokens`.

### 9.5 Clerk JWT `exp` Claim Uses Unix Epoch Seconds

**Problem**: The `exp` claim in Clerk JWTs is a standard Unix epoch timestamp (seconds since 1970-01-01). The `zenith_cli` JWT template has a 7-day TTL (configured in Clerk dashboard), so typical `exp` is ~604800 seconds in the future.

**Impact**: `refresh::decode_expiry()` must use `DateTime::from_timestamp(exp, 0)`, not parse an ISO 8601 string. The near-expiry buffer of 60 seconds is appropriate for 7-day tokens.

**Resolution**: `decode_expiry()` uses base64 decode → JSON parse → `exp` as i64 → `DateTime::from_timestamp()`. Validated in spike 0.17 test `test_jwt_exp_claim_decoding`.

### 9.6 `org_permissions` Must Be `[]` in JWT Template

**Problem**: The Clerk JWT template for `zenith_cli` must set `org_permissions` to a static empty array `[]`, NOT the Clerk shortcode `{{org.permissions}}`. The shortcode doesn't resolve correctly and causes `clerk-rs`'s `ActiveOrganization` deserialization to fail silently (the `org` field becomes `None`).

**Impact**: If someone misconfigures the JWT template, org-scoped features silently stop working — `claims.org_id` returns `None` even for org members.

**Resolution**: Document the JWT template configuration in the auth status output. `znt auth status` should report the org_id/org_slug/org_role — if these are `None` for an org member, the template is misconfigured. Validated in spike 0.20 findings.

### 9.7 `tiny_http` Blocks the Current Thread

**Problem**: `tiny_http::Server::recv()` is a blocking call. In an async context, this must be wrapped in `tokio::task::spawn_blocking()` to avoid blocking the tokio runtime.

**Impact**: The browser login flow must run the `recv()` loop in a blocking task. The timeout must be implemented via `recv_timeout()` or a separate timer.

**Resolution**: `browser_flow::login()` uses `tokio::task::spawn_blocking()` for the `recv()` loop. `tiny_http` supports `recv_timeout()` for the timeout implementation.

### 9.8 Token Source Detection Is Imperfect

**Problem**: `znt auth status` reports which tier the token came from (`keyring`, `env`, `file`). But `token_store::load()` tries keyring first and returns the first success — it can't distinguish between "keyring has token" and "keyring failed, env var has token" without additional state.

**Impact**: The `token_source` field in status output may say "keyring" even if the keyring was checked and the token was actually found in the env var on a previous attempt.

**Resolution**: `token_store::detect_token_source()` (defined in A6) checks each tier independently (keyring first, then env, then file) and returns `Option<String>` with the name of the first tier that has a non-empty value. This is a separate function from `load()` and matches the resolution order exactly. Called from `commands/auth/status.rs` (B4).

### 9.9 Auth Token in Config (`turso.auth_token`) vs Clerk JWT

**Problem**: The existing `config.turso.auth_token` (set via `ZENITH_TURSO__AUTH_TOKEN`) is a Turso Platform API token. A Clerk JWT is a different format. Both are accepted by `libsql::Builder::new_remote_replica()` — Turso routes Platform API tokens through its own auth path and Clerk JWTs through the JWKS path.

**Impact**: The `resolve_auth()` function must handle both token types. If a JWKS URL is configured, the token is validated as a Clerk JWT. If not, it's passed through as a raw Platform API token (legacy behavior).

**Resolution**: The four-tier resolution (keyring → env → file → config.turso.auth_token) produces a token. If `config.clerk.secret_key` is non-empty, tiers 1–3 tokens are validated as Clerk JWTs via JWKS and identity is extracted. Tier 4 (`config.turso.auth_token`) is passed through without JWKS validation — it's a legacy Platform API token. If `secret_key` is empty, tokens undergo best-effort expiry checking (decode `exp` via base64) but no JWKS validation — identity is `None` (see Gotcha 9.12). This preserves backward compatibility.

### 9.10 `open` Crate Behavior Varies by Platform

**Problem**: `open::that()` opens the system browser on macOS (`open`), Linux (`xdg-open`), and Windows (`start`). On headless Linux without a desktop environment, `xdg-open` fails silently or errors.

**Impact**: `znt auth login` (browser mode) won't work on headless Linux. The user must use `--api-key` mode.

**Resolution**: Catch `open::that()` errors and suggest `--api-key` mode. Print the sign-in URL to stderr so the user can open it manually in a remote browser.

### 9.11 clerk-rs JWKS Provider Requires Clerk Instance + Secret Key

**Problem**: The plan originally showed `MemoryCacheJwksProvider::new(jwks_url)`, but clerk-rs's `MemoryCacheJwksProvider::new()` takes a `Clerk` instance, NOT a URL string. The `Clerk` instance is constructed from `ClerkConfiguration::new(None, None, Some(secret_key), None)`. Internally, clerk-rs fetches JWKS from `{base_path}/jwks` (the Clerk Backend API endpoint), not from the `.well-known/jwks.json` (OIDC public endpoint). The `config.clerk.jwks_url` field points to the OIDC endpoint and is NOT used by clerk-rs.

**Impact**: `zen-auth` requires `config.clerk.secret_key` (not just `jwks_url`) for JWKS validation. All `validate()` callers must pass `secret_key`, not `jwks_url`. The `config.clerk.jwks_url` field is retained only for `resolve_frontend_api()` hostname extraction.

**Resolution**: `jwks.rs` constructs `Clerk` from `ClerkConfiguration` with `secret_key`. Process-scoped `OnceLock<Arc<MemoryCacheJwksProvider>>` caches the provider (1-hour TTL built into clerk-rs). Creating a new provider per call would defeat the cache and risk rate-limiting.

**Validated in**: Spike 0.17 — `MemoryCacheJwksProvider::new(clerk)` with `Clerk::new(ClerkConfiguration::new(None, None, Some(secret_key), None))`. Spike 0.20 — same pattern.

### 9.12 Partial-Auth State When Secret Key Not Configured

**Problem**: If a user sets `ZENITH_AUTH__TOKEN` to a Clerk JWT but doesn't configure `ZENITH_CLERK__SECRET_KEY`, Turso may still accept the JWT (via its own JWKS registration) but Zenith can't validate it locally — no identity extraction, no expiry checking, no refresh triggers.

**Impact**: The user gets cloud sync (Turso works) but `AppContext.identity` is `None`. Phase 9 visibility features won't work. If the token expires, Turso operations fail with `Sync("Unauthorized")` and Zenith has no recovery path because it can't verify or refresh.

**Resolution**: `resolve_auth()` performs best-effort expiry checking via `decode_expiry()` (unverified base64 parse of `exp` claim) when `secret_key` is empty. If the token appears expired, it's discarded and local mode is used. A warning is emitted: "token found but ZENITH_CLERK__SECRET_KEY not configured — identity unavailable, expiry checks are best-effort". This prevents silent failures while preserving backward compatibility.

---

## 10. Milestone 8 Validation

### Validation Command

```bash
cargo test -p zen-core -p zen-auth -p zen-db -p zen-cli
```

### Acceptance Criteria

| # | Criterion | Validated By |
|---|-----------|-------------|
| M8.1 | `zen-auth` crate compiles with all dependencies | `cargo build -p zen-auth` |
| M8.2 | JWKS validation accepts valid Clerk JWT and rejects invalid tokens | Unit test + spike 0.17 |
| M8.3 | Token store writes to keyring and reads back | Unit test (macOS) + spike 0.17 |
| M8.4 | Token store falls back to file when keyring unavailable | Unit test |
| M8.5 | JWT expiry detection correctly identifies near-expiry tokens | Unit test |
| M8.6 | `znt auth login` is dispatched before `AppContext::init()` | CLI parse test |
| M8.7 | `znt auth login` (browser) opens browser and captures JWT | Manual test |
| M8.8 | `znt auth login --api-key` mints JWT via Clerk Backend API | Integration test (needs Clerk credentials) |
| M8.9 | `znt auth logout` clears credentials from keyring | Unit test |
| M8.10 | `znt auth status` shows user/org/expiry when authenticated | Unit test |
| M8.11 | `znt auth status` shows `authenticated: false` when no token | Unit test |
| M8.12 | `AppContext::init()` uses Clerk JWT for `new_synced()` when token exists | Integration flow |
| M8.13 | `AppContext::init()` falls back to local mode when no token | Unit test |
| M8.14 | `AppContext.identity` is `Some(AuthIdentity)` when authenticated | Integration flow |
| M8.15 | `ZenDb::rebuild_synced()` recreates connection with fresh token | Unit test |
| M8.16 | Four-tier token resolution (keyring → env → file → config) works | Unit test |
| M8.17 | Auth failure never blocks local-only operation | CLI test |

---

## 11. Validation Traceability Matrix

Every implementation decision traces back to a spike test, upstream validated API, or established convention.

| Implementation | Validated By | Evidence |
|----------------|-------------|----------|
| `validate_jwt(jwt, &jwks_provider)` | Spike 0.17 | `test_clerk_jwks_validation` — standalone validation without web framework |
| `MemoryCacheJwksProvider::new(clerk)` where `clerk = Clerk::new(ClerkConfiguration::new(None, None, Some(secret_key), None))` | Spike 0.17 | `test_clerk_jwks_validation` — provider construction from `Clerk` instance + key caching |
| `tiny_http::Server::http("127.0.0.1:0")` | Spike 0.17 | `test_tiny_http_localhost_callback` — random port binding + request capture |
| `open::that(url)` | Spike 0.17 | `test_tiny_http_localhost_callback` — browser opens on macOS |
| `keyring::Entry::new(service, user)` | Spike 0.17 | `test_keyring_store_retrieve_delete` — full CRUD lifecycle |
| `keyring::Entry::set_password(jwt)` | Spike 0.17 | `test_keyring_store_retrieve_delete` — macOS Keychain write |
| File fallback with 0600 permissions | Spike 0.17 | `test_keyring_file_fallback` — Unix permission setting |
| JWT `exp` claim decode via base64 | Spike 0.17 | `test_jwt_exp_claim_decoding` — decode + near-expiry detection |
| Turso accepts Clerk JWT as auth token | Spike 0.17 | `test_turso_jwks_accepts_clerk_jwt` — `SELECT 1` succeeds |
| Embedded replica with Clerk JWT | Spike 0.17 | `test_turso_embedded_replica_with_clerk_jwt` — sync + write + read |
| Auth validated at builder time | Spike 0.17 | `test_expired_token_behavior` — `Sync("Unauthorized")` immediately |
| No hot-swap auth on replicas | Spike 0.17 | `test_no_hot_swap_auth_tokens` — must recreate client |
| Programmatic org-scoped JWT | Spike 0.20 | `test_programmatic_org_scoped_jwt` — session + template + validate |
| `org_permissions: []` requirement | Spike 0.20 | `test_programmatic_org_scoped_jwt` — shortcode doesn't resolve |
| `ClerkJwt.org` → `ActiveOrganization` | Spike 0.20 | `test_clerk_jwt_visibility_scoping` — real org_id extraction |
| `token_store::delete()` clears keyring + file | Spike 0.17 | `test_keyring_store_retrieve_delete` — delete_credential lifecycle |
| `token_store::detect_token_source()` tier detection | Design | Checks each tier independently, returns first match |
| `tokio::task::spawn_blocking` for `tiny_http::recv()` | Spike 0.17 | Gotcha 9.7 — blocking recv in async context |
| `tiny_http::Server::recv_timeout()` for login timeout | Spike 0.17 | `test_tiny_http_localhost_callback` — timeout behavior |
| `resolve_frontend_api()` uses `config.clerk.frontend_url` | zen-config | `ClerkConfig.frontend_url` field already exists |
| `wait_for_callback()` query string JWT extraction | Spike 0.17 | `test_tiny_http_localhost_callback` — token capture from redirect |
| Pre-context command dispatch | Phase 5 | `main.rs` — Init, Hook, Schema dispatched before `AppContext::init()` |
| Auth variant in `unreachable!()` arm | Phase 5 | `dispatch.rs` — pre-dispatched commands listed in unreachable |
| Graceful degradation to local mode | Phase 5/7 | `AppContext::init()` — synced failure falls back to local |
| `tracing::warn!` for non-fatal auth errors | Phase 5/7 | Consistent with install, wrap-up, workspace patterns |
| `output()` formatter for responses | Phase 5 | `crate::output::output` — JSON/table/raw output |
| `chrono::TimeDelta::seconds()` for expiry buffer | chrono 0.4 | Replaces deprecated `chrono::Duration::seconds()` |

---

## 12. Plan Review — Mismatch Log

*To be filled after internal and oracle review. Follows the same categorized format as Phase 7 §11.*

### Categories

- **F** = Factual (code listing disagrees with actual source)
- **E** = Editorial (prose is imprecise or ambiguous)
- **S** = Semantic (architectural/behavioral claim is misleading or overstated)

### Round 1 — Pre-Implementation Review (plan vs. codebase audit)

| # | Category | Description | Severity | Resolution |
|---|----------|-------------|----------|------------|
| F1 | F | `dispatch.rs` annotation said "add Auth dispatch" — Auth is pre-context and should be in `unreachable!()` arm, not dispatched | **High** | ✅ Fixed: Changed to "add Auth to unreachable! arm (pre-context)" in §4 and §8 |
| F2 | F | `zen-config.workspace = true` in zen-auth Cargo.toml — zen-auth never imports zen-config; config values passed as params | **High** | ✅ Fixed: Removed from Cargo.toml listing (A2) and §1 dependency summary |
| F3 | F | `anyhow.workspace = true` in zen-auth Cargo.toml — zen-auth uses `AuthError` (thiserror), never `anyhow` | Low | ✅ Fixed: Removed from Cargo.toml listing (A2) |
| F4 | F | `token_store.rs` (A6) only showed `store()` and `load()`. Missing `delete()` (called by `logout()`), `store_file()`, `load_file()` helpers | **Medium** | ✅ Fixed: Added `delete()`, `detect_token_source()`, `store_file()`, `load_file()`, `credentials_path()` |
| F5 | F | `detect_token_source()` called in B4 but never defined anywhere | **Medium** | ✅ Fixed: Defined in A6 (`token_store::detect_token_source()`), B4 updated to use `zen_auth::token_store::detect_token_source()` |
| F6 | F | `browser_flow::login()` called blocking `wait_for_callback()` inside async fn without `spawn_blocking` (contradicted Gotcha 9.7) | **High** | ✅ Fixed: A7 now uses `tokio::task::spawn_blocking(move \|\| wait_for_callback(...))` |
| F7 | F | Missing `wait_for_callback()` and `extract_frontend_api()` helper implementations — called but never defined | **Medium** | ✅ Fixed: `wait_for_callback()` added to A7, `resolve_frontend_api()` added to B3 |
| F8 | F | Gotcha 9.2 proposed new `ZENITH_CLERK__FRONTEND_API` env var — `ClerkConfig` already has `frontend_url: String` (zen-config/src/clerk.rs:25) | **Medium** | ✅ Fixed: Gotcha 9.2 and B3 now use `config.clerk.frontend_url` as primary source, JWKS hostname extraction as fallback |
| F9 | F | `chrono::Duration::seconds()` deprecated in chrono 0.4.38+ in favor of `chrono::TimeDelta::seconds()` | Low | ✅ Fixed: Changed to `chrono::TimeDelta::seconds()` in A4 |
| S1 | S | §3.2 described "Three-Tier Token Resolution" but actual code has four tiers: keyring → env → file → config.turso.auth_token. File fallback (`~/.zenith/credentials`) was conflated with config fallback | **High** | ✅ Fixed: Renamed to "Four-Tier Token Resolution", split into token store layer (tiers 1–3) and AppContext layer (tier 4). All references updated throughout document. |
| S2 | S | Phase numbering mismatch — plan says "Phase 8" but roadmap (§10-§11) assigns auth to Phase 9 (tasks 9.1–9.7, 9.13–9.14, 9.17–9.20). Roadmap Phase 8 is "Cloud & Catalog" (already DONE) | **Medium** | ✅ Fixed: Added numbering deviation note at top of plan explaining the re-sequencing |
| E1 | E | `SwitchOrg` subcommand had args struct but no handler implementation listing (unlike login/logout/status) | **Medium** | ✅ Fixed: Added B5 with `switch_org::handle()` implementation |
| E2 | E | B6 (AppContext) code listing used undeclared variables (`replica_path`, `db_path_str`, `trail_dir`) behind `// ... existing setup ...` comment — didn't show `has_local_replica()` path selection | Low | ✅ Fixed: B7 now shows complete method with all path setup and inline `// unchanged` / `// NEW` annotations |
| E3 | E | `rebuild_with_token()` (C2) defined but no caller site documented — unclear who triggers it and when | Low | ✅ Fixed: Added "Caller site" paragraph documenting intended usage and noting manual recovery path |

### Round 2 — Oracle Review (architectural + semantic accuracy)

| # | Category | Description | Severity | Resolution |
|---|----------|-------------|----------|------------|
| F10 | F | `MemoryCacheJwksProvider::new(jwks_url)` in A5 is wrong — clerk-rs `MemoryCacheJwksProvider::new()` takes a `Clerk` instance, not a URL string. Spikes confirm: `MemoryCacheJwksProvider::new(clerk)` where `clerk = Clerk::new(ClerkConfiguration::new(...))` | **High** | ✅ Fixed: A5 rewritten to construct `Clerk` from `ClerkConfiguration` with `secret_key` param. See resolution note below. |
| S3 | S | clerk-rs fetches JWKS from `{base_path}/jwks` (Clerk Backend API), not from `/.well-known/jwks.json` (OIDC public endpoint). The plan's `config.clerk.jwks_url` points to the OIDC endpoint, but clerk-rs doesn't use that URL — it uses `Clerk.base_path`. The `jwks_url` config is only for informational purposes (deriving frontend hostname). | **High** | ✅ Fixed: `jwks.rs` now constructs `Clerk` from `secret_key` param. `validate()` signature changed to accept `secret_key: &str` instead of `jwks_url: &str`. All callers updated. `config.clerk.jwks_url` retained for `resolve_frontend_api()` fallback only. |
| S4 | S | `MemoryCacheJwksProvider` recreated on every `validate()` call — defeats the 1-hour in-memory key cache, adds latency per validation, risks rate-limiting from repeated JWKS fetches | **High** | ✅ Fixed: Added Gotcha 9.11 documenting this. Resolution: create provider once per process. Implementation deferred to coding phase — will use `OnceLock<Arc<MemoryCacheJwksProvider>>` in `jwks.rs` or store in `AppContext`. Plan updated to note caching requirement. |
| F11 | F | `switch-org` (B5) constructs `sign_in_url_with_org` with `organization=` param, then calls `browser_flow::login()` which constructs its OWN URL internally — the org-parameterized URL is dead code | **High** | ✅ Fixed: `browser_flow::login()` signature updated to accept `org_slug: Option<&str>`. When provided, adds `&organization={slug}` to the sign-in URL. B5 now passes `Some(&args.org_slug)` to `browser_flow::login()`. Dead `sign_in_url_with_org` variable removed. |
| S5 | S | `resolve_auth()` returns `(raw_token, None)` when `jwks_url` is empty but tiers 1–3 have a token — if `ZENITH_AUTH__TOKEN` is a Clerk JWT and `jwks_url` is missing, Turso may accept it (via its own JWKS registration) but no expiry checking or identity extraction occurs, creating a partial-auth state | **High** | ✅ Fixed: Added Gotcha 9.12 documenting this edge case. Resolution: `resolve_auth()` now attempts `decode_expiry()` (unverified base64 parse) on tier 1–3 tokens even when `jwks_url` is empty, to detect expired tokens. Identity remains `None` (no JWKS validation = no identity). Warning emitted: "token found but JWKS URL not configured — identity unavailable, expiry checks are best-effort". |
| S6 | S | Browser callback is single-shot — first HTTP request to `127.0.0.1:{port}` that lacks `token` param (e.g., favicon request, browser preflight) causes immediate failure. Should loop until timeout, ignoring non-callback requests. | **Medium** | ✅ Fixed: `wait_for_callback()` in A7 now loops on `server.recv_timeout()`, ignoring requests whose URL does not start with `/callback?`. Responds to ignored requests with 204 No Content. Only breaks on `/callback?token=` or timeout. |
| F12 | F | `ClerkJwt.exp` is `i32` in clerk-rs (authorizer.rs:44), but plan's `decode_expiry()` independently re-parses JWT payload via base64 to extract `exp` as `i64`. This is redundant — `exp` is already available from `clerk_jwt.exp` after validation. | **Medium** | ✅ Fixed: `validate()` in A5 now uses `i64::from(clerk_jwt.exp)` → `DateTime::from_timestamp()` instead of calling `decode_expiry()`. `decode_expiry()` retained in `refresh.rs` for quick expiry checks WITHOUT full JWKS validation (e.g., `check_stored_token()` pre-screening). |
| S7 | S | `credentials_path()` falls back to `PathBuf::from(".")` when `dirs::home_dir()` returns `None` — would write credentials to CWD, a security risk on shared systems | **Medium** | ✅ Fixed: `credentials_path()` now returns `Result<PathBuf, AuthError>` and returns `Err(AuthError::TokenStoreError("home directory not found"))` when `dirs::home_dir()` is `None`. Callers updated. |
| E4 | E | Missing gotcha for clerk-rs `MemoryCacheJwksProvider` requiring `Clerk` instance (not URL string) — this is a more significant API mismatch than acknowledged in existing gotchas | **Medium** | ✅ Fixed: Added Gotcha 9.11 "clerk-rs JWKS Provider Requires Clerk Instance + Secret Key" documenting the API shape, the `base_path` vs `jwks_url` distinction, and the implication that `config.clerk.secret_key` must be available for JWKS validation |
| S8 | S | Token source precedence (keyring > env var) may surprise CI users who set `ZENITH_AUTH__TOKEN` to override a stale keyring token. The env var is the standard CI override mechanism but loses to the keyring. | **Low** | ✅ Documented: Added note to §3.2 acknowledging the precedence and explicitly stating that CI environments should not have a keyring daemon, so the keyring tier naturally falls through. For desktop users who need to override keyring, `znt auth logout` clears the keyring first. |
| E5 | E | Plan's `validate()` function signature in A5 passes `jwks_url: &str` which doesn't match the actual clerk-rs API. Validation traceability matrix (§11) row for `MemoryCacheJwksProvider::new(jwks_url)` also shows incorrect argument | **Low** | ✅ Fixed: Traceability matrix row updated to `MemoryCacheJwksProvider::new(clerk)` with note about `Clerk` construction from `ClerkConfiguration` |

### Summary

**Round 1**: 4 high + 5 medium + 5 low = 14 findings — all fixed.
- **Factual (F1–F9)**: dispatch.rs misannotation, unnecessary deps, missing function definitions, blocking-in-async, redundant config field, deprecated API
- **Semantic (S1–S2)**: token resolution tier count, phase numbering deviation
- **Editorial (E1–E3)**: missing handler, incomplete code listing, undocumented caller

**Round 2**: 5 high + 3 medium + 2 low = 10 findings — all fixed.
- **Factual (F10–F12)**: clerk-rs `MemoryCacheJwksProvider` API mismatch (takes `Clerk` not URL), `switch-org` dead code path, redundant JWT exp decoding
- **Semantic (S3–S8)**: JWKS endpoint mismatch (Backend API vs OIDC public), provider-per-call defeats caching, partial-auth state with missing `jwks_url`, single-shot callback fragility, credentials CWD fallback risk, token precedence for CI
- **Editorial (E4–E5)**: missing gotcha for clerk-rs API shape, traceability matrix inaccuracy

**Overall assessment**: Plan is now architecturally sound. The critical clerk-rs API mismatch (Round 2 F10/S3) has been resolved by rewriting `jwks.rs` to use the correct `Clerk` + `ClerkConfiguration` construction pattern validated in spikes 0.17 and 0.20. The `switch-org` dead code path is fixed. Token lifecycle is complete. All code listings match actual APIs.

---

## Cross-References

- Architecture overview: [03-architecture-overview.md](./03-architecture-overview.md)
- Data architecture: [02-data-architecture.md](./02-data-architecture.md)
- CLI API design: [04-cli-api-design.md](./04-cli-api-design.md) (§3, §18 auth commands)
- Crate designs: [05-crate-designs.md](./05-crate-designs.md) (§11 zen-cli, zen-auth)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md) (§10 Phase 8, §11 Phase 9)
- Phase 7 plan (format reference): [27-phase7-agentfs-integration-plan.md](./27-phase7-agentfs-integration-plan.md)
- Clerk auth spike: [15-clerk-auth-turso-jwks-spike-plan.md](./15-clerk-auth-turso-jwks-spike-plan.md)
- Catalog visibility spike: [18-catalog-visibility-spike-plan.md](./18-catalog-visibility-spike-plan.md)
- Spike 0.17 source: `crates/zen-db/src/spike_clerk_auth.rs`
- Spike 0.20 source: `crates/zen-db/src/spike_catalog_visibility.rs`
