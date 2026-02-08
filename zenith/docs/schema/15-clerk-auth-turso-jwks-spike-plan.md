# Zenith: Clerk Auth + Turso JWKS -- Spike Plan

**Version**: 2026-02-08
**Status**: DONE -- 14/14 tests pass
**Purpose**: Validate the complete CLI authentication flow: Clerk browser OAuth login, JWT validation via `clerk-rs`, Turso JWKS integration (Clerk JWT as Turso auth token), token storage (`keyring` + file fallback), API key CI fallback, and token lifecycle management for embedded replicas.
**Spike ID**: 0.17
**Crate**: zen-db (spike file, reuses existing Turso credentials infrastructure)
**Blocks**: Phase 9 (tasks 9.1-9.12: zen-auth crate, Turso JWKS wiring, team DB changes)

---

## Table of Contents

1. [Motivation](#1-motivation)
2. [Background & Prior Art](#2-background--prior-art)
3. [Architecture](#3-architecture)
4. [What We're Validating](#4-what-were-validating)
5. [Dependencies](#5-dependencies)
6. [Spike Tests](#6-spike-tests)
7. [Evaluation Criteria](#7-evaluation-criteria)
8. [What This Spike Does NOT Test](#8-what-this-spike-does-not-test)
9. [Success Criteria](#9-success-criteria)
10. [Post-Spike Actions](#10-post-spike-actions)

---

## 1. Motivation

Zenith Phase 9 (Team & Pro) requires multi-user authentication and shared cloud databases. The chosen architecture is serverless: CLI authenticates users via Clerk, and Turso validates Clerk-issued JWTs directly via JWKS -- no custom server, no token minting API calls at runtime.

This spike validates every link in the chain before we build the `zen-auth` crate:

1. Can `clerk-rs` validate JWTs in a CLI context (no web framework)?
2. Does Turso's JWKS integration accept Clerk JWTs for `libsql` connections?
3. Can the browser OAuth callback pattern work from a Rust CLI?
4. Does `keyring` provide reliable cross-platform token storage?
5. How do embedded replicas behave when the JWT expires mid-session?

### Why Now (Phase 0 Extension)

All Phase 9 work depends on these answers. If Clerk-rs doesn't work as a standalone validator, we fall back to DIY `jsonwebtoken` + JWKS fetch. If Turso JWKS doesn't accept Clerk JWTs, we fall back to Platform API token minting (already validated in spike 0.3). If the browser flow is too complex, we start with API keys only.

### Prior Art: aether-auth

Aether (`/Users/wrath/projects/aether/crates/aether-auth/`) has a working Clerk integration:
- `clerk-rs` 0.4.2 with `MemoryCacheJwksProvider` for JWKS validation
- `Claims` struct with org_id, org_slug, org_role, session, impersonation
- `AuthError` enum with 7 variants
- tonic interceptor (not applicable to CLI, but validation logic is portable)

Key differences for Zenith:
- No tonic/gRPC -- pure CLI context
- Turso JWKS (Clerk JWT = Turso auth token) instead of Platform API minting
- Browser OAuth flow (aether doesn't have this -- it's a server)
- Token persistence (aether holds tokens in memory per-request)

---

## 2. Background & Prior Art

### Clerk CLI Auth Pattern

Clerk is browser-centric. The proven CLI pattern (documented by Erik Steiger):

```
CLI starts tiny_http on 127.0.0.1:0 (random port)
  -> Opens browser to Clerk Account Portal / hosted sign-in page
  -> User authenticates in browser
  -> Clerk redirects to http://127.0.0.1:{port}/callback?token=eyJ...
  -> CLI captures JWT, stores in OS keychain
```

**Requirements**:
- A hosted page that renders `<SignIn/>` and redirects with JWT (Clerk Account Portal)
- A custom JWT template in Clerk Dashboard with long TTL (7 days) for CLI sessions
- API key fallback for CI/headless environments (`POST /api_keys/verify`)

### Turso JWKS Integration

Instead of minting tokens via Platform API, register Clerk's JWKS endpoint:

```bash
turso org jwks save clerk https://ruling-doe-21.clerk.accounts.dev/.well-known/jwks.json
```

Then Clerk-issued JWTs with Turso permission claims (`p` field) are accepted directly by `libsql` as auth tokens. No runtime API calls to Turso Platform.

**Permission template** (from `turso org jwks template`):
```json
{
  "p": {
    "rw": {
      "ns": ["org-slug.zenith-dev"],
      "tables": {
        "all": { "data_read": true, "data_add": true, "data_update": true, "data_delete": true }
      }
    }
  }
}
```

### Gotchas from Research

1. **Clerk default token lifetime is 60 seconds** -- need custom JWT template with longer TTL
2. **libsql embedded replica**: `authToken` fixed at client creation. No hot-swapping. Must recreate client on token expiry.
3. **Missing `p` claim in JWT = full access to all databases** -- always include permissions
4. **Turso JWKS is beta**: only Clerk and Auth0 supported
5. **`clerk-rs` requires `sk_test_`/`sk_live_` secret key**, not publishable key
6. **`keyring` on headless Linux needs Secret Service daemon** -- file fallback required

---

## 3. Architecture

```
CLI (znt auth login)
     │
     ├── tiny_http on 127.0.0.1:0    ◄── Clerk redirect with JWT
     │
     ├── open browser ──► Clerk Account Portal
     │                    User signs in
     │                    JWT issued (7-day TTL, Turso permissions)
     │
     ├── clerk-rs validates JWT locally (JWKS cached)
     │
     ├── keyring stores JWT (macOS Keychain / Windows Cred / Linux Secret Service)
     │   └── fallback: ~/.zenith/credentials (0600)
     │
     └── libsql connects with Clerk JWT as authToken
         ├── Builder::new_remote(url, clerk_jwt)           ── remote only
         └── Builder::new_remote_replica(path, url, jwt)   ── embedded replica
```

---

## 4. What We're Validating

8 hypotheses that must hold for Phase 9 to proceed:

| # | Hypothesis | Risk if wrong |
|---|---|---|
| H1 | `clerk-rs` 0.4.2 `MemoryCacheJwksProvider` + `validate_jwt()` work in a non-web context (no framework features) | Need DIY jsonwebtoken + JWKS fetch |
| H2 | `tiny_http` localhost callback captures the JWT from Clerk redirect | Browser flow doesn't work, API keys only |
| H3 | `keyring` v3 stores and retrieves tokens on macOS (primary dev platform) | Need file-only storage |
| H4 | Turso JWKS integration accepts Clerk JWT as `libsql` auth token for remote connections | Fall back to Platform API minting (spike 0.3) |
| H5 | Turso JWKS works with embedded replicas (`Builder::new_remote_replica`) | Cloud sync requires Platform API tokens |
| H6 | Token expiry detection works: decode JWT `exp` claim, detect near-expiry | Can't implement refresh flow |
| H7 | Clerk API key verification works via `clerk-rs` for CI/headless fallback | Need manual Backend API HTTP calls |
| H8 | Claims extraction (org_id, org_role, user_id) from Clerk JWT matches aether pattern | Need custom JWT parsing |

---

## 5. Dependencies

### New Workspace Dependencies

```toml
# Auth (Phase 9)
clerk-rs = "0.4.2"              # Clerk JWT validation, JWKS caching
open = "5"                       # Launch system browser
tiny_http = "0.12"               # Localhost callback server (sync, minimal)
keyring = { version = "3", features = ["apple-native", "windows-native"] }
```

### Spike File Dependencies

The spike runs in `zen-db` crate (has existing Turso infrastructure). Additional dev-dependencies:

```toml
[dev-dependencies]
clerk-rs = { workspace = true }
open = { workspace = true }
tiny_http = { workspace = true }
keyring = { workspace = true }
reqwest = { workspace = true }    # already in workspace
serde_json = { workspace = true }
tokio = { workspace = true }
tempfile = { workspace = true }
dotenvy = { workspace = true }
```

---

## 6. Spike Tests

**File**: `zenith/crates/zen-db/src/spike_clerk_auth.rs`

### Part A: clerk-rs JWT Validation (3 tests)

| # | Test | Validates |
|---|------|-----------|
| A1 | `spike_clerk_jwks_validator_creates` | Create `JwksValidator` from `ZENITH_CLERK__SECRET_KEY`. Verify it initializes without error. (H1) |
| A2 | `spike_clerk_validate_jwt_from_env` | Load a pre-generated Clerk JWT from `ZENITH_AUTH__TEST_TOKEN` env var (created via Clerk Dashboard or API). Validate it with `JwksValidator::validate()`. Extract claims: `sub`, `org_id`, `org_slug`, `org_role`. Verify all fields present. Skip if no test token. (H1, H8) |
| A3 | `spike_clerk_expired_token_rejected` | Construct or load an expired Clerk JWT. Validate it -- should return `AuthError::TokenExpired` or `AuthError::InvalidToken`. (H1) |

### Part B: Browser OAuth Flow (2 tests)

| # | Test | Validates |
|---|------|-----------|
| B1 | `spike_tiny_http_callback_captures_token` | Start `tiny_http` on `127.0.0.1:0`. Send a mock HTTP GET to `http://127.0.0.1:{port}/callback?token=test_jwt_123`. Verify the server captures `test_jwt_123` and responds with success HTML. No real browser. (H2) |
| B2 | `spike_browser_open_does_not_panic` | Call `open::that("https://example.com")` and verify it doesn't panic (may or may not open browser in CI). Verify it returns `Ok(())` or an error (both acceptable -- just shouldn't crash). (H2) |

### Part C: Token Storage (3 tests)

| # | Test | Validates |
|---|------|-----------|
| C1 | `spike_keyring_store_retrieve_delete` | Store a token via `keyring::Entry::new("zenith-spike", "test")`. Retrieve it. Delete it. Verify full lifecycle. Skip on CI if keyring unavailable. (H3) |
| C2 | `spike_file_storage_fallback` | Write token to `tempdir/.zenith/credentials` with 0600 permissions. Read it back. Verify permissions on Unix. (H3) |
| C3 | `spike_token_expiry_detection` | Decode a JWT's `exp` claim (base64 decode the payload, parse JSON). Check if expired. Check if near-expiry (within 60s buffer). No `clerk-rs` needed -- pure JWT parsing. (H6) |

### Part D: Turso JWKS Integration (4 tests)

**These tests require a live Turso database with JWKS registered for Clerk.**

**Prerequisites**:
```bash
# One-time setup (before running tests):
turso org jwks save clerk https://ruling-doe-21.clerk.accounts.dev/.well-known/jwks.json
# Create JWT template "zenith_cli" in Clerk Dashboard with Turso permissions
```

| # | Test | Validates |
|---|------|-----------|
| D1 | `spike_turso_jwks_remote_connection` | Get a fresh Clerk JWT (from test token env var or pre-generated). Connect to Turso via `Builder::new_remote(url, clerk_jwt)`. Execute `SELECT 1`. Verify connection works. Skip if credentials missing. (H4) |
| D2 | `spike_turso_jwks_embedded_replica` | Connect via `Builder::new_remote_replica(local_path, url, clerk_jwt)`. Create table, insert row, sync, query back. Verify embedded replica works with Clerk JWT. Skip if credentials missing. (H5) |
| D3 | `spike_turso_jwks_write_forward` | Through embedded replica with Clerk JWT: INSERT a row, sync, verify it's in cloud. Connect with a second replica, sync, verify the row is visible. (H5) |
| D4 | `spike_turso_jwks_expired_token_behavior` | Connect embedded replica with a valid Clerk JWT. Manually wait or use a short-lived token. Attempt sync after expiry. Document the error type (auth error? connection error?). Verify local reads still work. (H5, H6) |

### Part E: API Key Fallback (2 tests)

| # | Test | Validates |
|---|------|-----------|
| E1 | `spike_clerk_api_key_verify` | Use `clerk-rs` to call `POST /api_keys/verify` with a test API key. Extract claims. Skip if no API key configured. (H7) |
| E2 | `spike_clerk_api_key_to_turso` | If API key verification returns claims with org_id, generate a Clerk JWT (via Backend API `getToken`), use it to connect to Turso. Full API key → JWT → Turso flow. Skip if not configured. (H7, H4) |

**Total: 14 tests**

---

## 7. Evaluation Criteria

| Criterion | Weight | How We Measure |
|-----------|--------|---------------|
| clerk-rs standalone validation | **Critical** | Tests A1-A3: validator creates, validates tokens, rejects expired |
| Turso JWKS accepts Clerk JWT | **Critical** | Tests D1-D3: remote + replica connections work with Clerk JWT |
| Browser callback captures token | **High** | Test B1: tiny_http server receives and parses callback |
| Token storage works | **High** | Tests C1-C2: keyring and file fallback both work |
| Expiry detection | **High** | Tests C3, D4: JWT exp claim decoded, expiry detected |
| API key fallback | **Medium** | Tests E1-E2: Clerk API key verification works |
| Embedded replica expiry behavior | **Medium** | Test D4: document error type, verify local reads survive |
| Claims extraction | **Medium** | Test A2: org_id, org_slug, org_role all present |

---

## 8. What This Spike Does NOT Test

- **Full browser flow end-to-end** (real Clerk sign-in page + redirect) -- requires manual testing with a browser. The spike tests the callback server and browser-open separately.
- **Token refresh automation** -- the spike detects expiry but doesn't implement automatic re-auth. That's Phase 9 task 9.7.
- **org_id scoped queries** -- the spike connects to Turso with a Clerk JWT but doesn't test org-scoped SQL. That's Phase 9 task 9.10.
- **Multi-user isolation** -- testing that user A can't see user B's data. Requires two Clerk users and two JWT templates.
- **Clerk organization management** -- `znt team invite` / `znt team list`. That's Phase 9 task 9.22.
- **JWT template creation** -- must be done manually in Clerk Dashboard before running D-tests.

---

## 9. Success Criteria

- **clerk-rs validates Clerk JWTs without a web framework** (tests A1-A3 pass)
- **Turso accepts Clerk JWT via JWKS** for both remote and embedded replica connections (tests D1-D3 pass)
- **tiny_http captures callback tokens** (test B1 passes)
- **Token storage works on macOS** (test C1 passes, or C2 as fallback)
- **JWT expiry detection works** (test C3 passes)
- **Gotchas and error behaviors documented** (test D4 behavior documented)
- **At least 10/14 tests pass** (D-tests and E-tests may be skipped if cloud prereqs not met)

---

## 10. Post-Spike Actions

### If Spike Passes (Expected Path)

| Doc | Update |
|-----|--------|
| `07-implementation-plan.md` | Add spike 0.17 to Phase 0 table with results. Add Phase 9 section. |
| `05-crate-designs.md` | Add zen-auth crate design (Claims, AuthError, JwksValidator, BrowserFlow, TokenStore) |
| `Cargo.toml` | Add `clerk-rs`, `open`, `tiny_http`, `keyring` to workspace deps. Add `zen-auth` crate member. |
| `INDEX.md` | Add doc 15 to document map |

### If clerk-rs Doesn't Work (Fallback A)

- Use `jsonwebtoken` + `reqwest` for manual JWKS fetch and JWT validation
- Port aether's validation logic without `clerk-rs` dependency
- Still use Turso JWKS (the JWT format is the same)

### If Turso JWKS Doesn't Work (Fallback B)

- Fall back to Platform API token minting (already validated in spike 0.3)
- Add `turso_mint.rs` to zen-auth: mint database tokens via `POST /v1/organizations/{org}/databases/{db}/auth/tokens`
- Clerk JWT used only for identity, not for Turso auth

### If Browser Flow Too Complex (Fallback C)

- Start with API key only (tests E1-E2)
- User generates key in Clerk Dashboard, pastes into `.env`
- Add browser flow later when we have the hosted sign-in page

---

## Cross-References

- Turso sync spike: [spike_libsql_sync.rs](../../crates/zen-db/src/spike_libsql_sync.rs)
- Aether auth implementation: `/Users/wrath/projects/aether/crates/aether-auth/`
- Clerk config: [zen-config/src/clerk.rs](../../crates/zen-config/src/clerk.rs)
- Turso config: [zen-config/src/turso.rs](../../crates/zen-config/src/turso.rs)
- Implementation plan: [07-implementation-plan.md](./07-implementation-plan.md)
