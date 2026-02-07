# Security Architecture Strategy

## Executive Summary

**Mission Control Security Philosophy**: Secure by design, authenticate simply, delegate safely.

This document outlines comprehensive security architecture for Mission Control, focusing on:
- **API key authentication** for simplified CLI access
- **User-delegated agent permissions** for secure agent operations
- **Clerk-rs integration** for robust user management
- **Axiom monitoring** for comprehensive security observability

---

## Security Model Overview

### Core Security Principles

1. **Authentication Simplicity**: API key authentication eliminates OAuth complexity for CLI users
2. **Permission Delegation**: Users authenticate once, agents inherit scoped permissions
3. **Least Privilege**: Agents and services only get permissions they absolutely need
4. **Audit Everything**: All security events logged to Axiom for monitoring
5. **Fail Secure**: Default to deny, explicit allow for permissions

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    MISSION CONTROL SECURITY ARCHITECTURE               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐│
│   │   CLI Client   │    │  Web Auth UI    │    │   Clerk API     ││
│   │                 │    │                 │    │                 ││
│   │ • Local key store│◄──►│ • API key page  │◄──►│ • JWT validation ││
│   │ • Bearer auth   │    │ • Token minting  │    │ • M2M tokens    ││
│   │ • Auto-refresh   │    │ • User management│    │ • Permission DB  ││
│   └─────────────────┘    └─────────────────┘    └─────────────────┘│
│             │                       │                       │             │
│             ▼                       ▼                       ▼             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │              SECURITY LAYER                               │   │
│   │                                                         │   │
│   │ • Permission validation                                     │   │
│   │ • Scope checking                                         │   │
│   │ • Token management                                      │   │
│   │ • Audit logging                                        │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│             │                                                       │
│             ▼                                                       │
│   ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐│
│   │ Research Agent  │    │   POC Agent     │    │ Supervisor      ││
│   │                 │    │                 │    │                 ││
│   │ • Inherited perms│    │ • Inherited perms│    │ • Agent mgmt     ││
│   │ • Scoped tokens  │    │ • Scoped tokens  │    │ • Config access  ││
│   │ • Auto-rotate   │    │ • Auto-rotate   │    │ • Orchestration  ││
│   └─────────────────┘    └─────────────────┘    └─────────────────┘│
│                                                                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Clerk Integration Strategy

### Authentication Architecture

#### 1. User Authentication (API Key Flow)

```rust
// src/security/authentication.rs
use clerk_rs::Clerk;
use anyhow::{Result, anyhow};

pub struct AuthManager {
    clerk_client: Clerk,
    keychain: OsKeychain,
}

impl AuthManager {
    /// Authenticate user with API key (no browser required)
    pub async fn authenticate_user(&self, api_key: &str) -> Result<UserSession, SecurityError> {
        // Validate API key with Clerk
        let user_info = self.clerk_client
            .verify_api_key(api_key)
            .await
            .map_err(|e| SecurityError::InvalidCredentials(format!("API key validation failed: {}", e)))?;
        
        // Store securely in OS keychain
        self.keychain
            .store("mission_control_api_key", api_key)
            .map_err(|e| SecurityError::KeychainError(e.to_string()))?;
        
        // Create user session
        let session = UserSession {
            user_id: user_info.id,
            email: user_info.email,
            api_key: api_key.to_string(),
            expires_at: Utc::now() + Duration::seconds(86400), // 24 hours
            permissions: user_info.permissions,
        };
        
        tracing::info!(
            user_id = %session.user_id,
            email = %session.email,
            "User authenticated successfully"
        );
        
        Ok(session)
    }
    
    /// Auto-refresh session on startup
    pub async fn ensure_authenticated(&self) -> Result<UserSession, SecurityError> {
        match self.load_stored_session()? {
            Some(session) if !session.is_expired() => Ok(session),
            Some(_) => {
                println!("⚠️  Session expired. Please re-authenticate.");
                self.prompt_reauth().await
            },
            None => {
                println!("🔐  First-time setup. Please authenticate.");
                self.prompt_reauth().await
            }
        }
    }
    
    async fn prompt_reauth(&self) -> Result<UserSession, SecurityError> {
        let api_key = rpassword::prompt_password("Enter your Mission Control API key: ")
            .map_err(|e| SecurityError::InputError(e.to_string()))?;
        
        self.authenticate_user(&api_key).await
    }
    
    fn load_stored_session(&self) -> Result<Option<UserSession>, SecurityError> {
        match self.keychain.get("mission_control_api_key") {
            Ok(Some(api_key)) => {
                // Validate stored key is still valid
                let user_info = futures::executor::block_on(
                    self.clerk_client.verify_api_key(&api_key)
                ).map_err(|_| SecurityError::InvalidSession("Stored token invalid"))?;
                
                Ok(Some(UserSession {
                    user_id: user_info.id,
                    email: user_info.email,
                    api_key,
                    expires_at: Utc::now() + Duration::seconds(86400),
                    permissions: user_info.permissions,
                }))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(SecurityError::KeychainError(e.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub user_id: String,
    pub email: String,
    pub api_key: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub permissions: Vec<String>,
}

impl UserSession {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}
```

#### 2. Agent Token Management

```rust
// src/security/agent_tokens.rs
use clerk_rs::models::{ApiKeyRequest, ApiKey};
use std::collections::HashMap;

pub struct AgentTokenManager {
    clerk_client: Clerk,
    user_session: UserSession,
    agent_tokens: Arc<RwLock<HashMap<String, AgentToken>>>,
}

impl AgentTokenManager {
    /// Create scoped token for specific agent
    pub async fn create_agent_token(&self, agent_id: &str, scopes: &[Scope]) -> Result<String, SecurityError> {
        let agent_scopes: Vec<String> = scopes.iter().map(|s| s.to_string()).collect();
        
        let api_key_request = ApiKeyRequest {
            name: format!("Mission Control Agent: {}", agent_id),
            subject: Some(self.user_session.user_id.clone()),
            scopes: Some(agent_scopes),
            seconds_until_expiration: Some(86400), // 24 hours
            metadata: Some(serde_json::json!({
                "agent_id": agent_id,
                "created_by": "mission_control",
                "purpose": "agent_delegation"
            })),
        };
        
        let created_key = self.clerk_client
            .create_api_key(api_key_request)
            .await
            .map_err(|e| SecurityError::TokenCreationFailed(format!("Failed to create agent token: {}", e)))?;
        
        let agent_token = AgentToken {
            agent_id: agent_id.to_string(),
            token: created_key.token.clone(),
            scopes: scopes.to_vec(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::seconds(86400),
        };
        
        // Cache the token
        let mut tokens = self.agent_tokens.write().unwrap();
        tokens.insert(agent_id.to_string(), agent_token.clone());
        
        tracing::info!(
            agent_id = %agent_id,
            scopes_count = %scopes.len(),
            "Agent token created successfully"
        );
        
        Ok(created_key.token)
    }
    
    /// Get valid agent token, refresh if needed
    pub async fn get_agent_token(&self, agent_id: &str) -> Result<String, SecurityError> {
        let tokens = self.agent_tokens.read().unwrap();
        
        match tokens.get(agent_id) {
            Some(token) if !token.is_expired() => Ok(token.token.clone()),
            Some(expired_token) => {
                drop(tokens);
                tracing::warn!(
                    agent_id = %agent_id,
                    "Agent token expired, refreshing"
                );
                self.refresh_agent_token(agent_id, &expired_token.scopes).await
            },
            None => {
                drop(tokens);
                let scopes = self.get_default_scopes_for_agent(agent_id)?;
                self.create_agent_token(agent_id, &scopes).await
            }
        }
    }
    
    async fn refresh_agent_token(&self, agent_id: &str, scopes: &[Scope]) -> Result<String, SecurityError> {
        // Revoke old token first
        self.revoke_agent_token(agent_id).await?;
        
        // Create new token
        self.create_agent_token(agent_id, scopes).await
    }
    
    async fn revoke_agent_token(&self, agent_id: &str) -> Result<(), SecurityError> {
        let tokens = self.agent_tokens.read().unwrap();
        
        if let Some(token) = tokens.get(agent_id) {
            // Revoke through Clerk API
            self.clerk_client
                .revoke_api_key(&token.token)
                .await
                .map_err(|e| SecurityError::TokenRevocationFailed(format!("Failed to revoke token: {}", e)))?;
            
            tracing::info!(
                agent_id = %agent_id,
                "Agent token revoked"
            );
        }
        
        Ok(())
    }
    
    fn get_default_scopes_for_agent(&self, agent_id: &str) -> Result<Vec<Scope>, SecurityError> {
        match agent_id {
            "research_agent" => Ok(vec![
                Scope::MissionsRead,
                Scope::StorageRead,
                Scope::MissionsExecute,
            ]),
            "poc_agent" => Ok(vec![
                Scope::MissionsWrite,
                Scope::StorageWrite,
                Scope::MissionsExecute,
            ]),
            "documentation_agent" => Ok(vec![
                Scope::MissionsRead,
                Scope::StorageWrite,
                Scope::ConfigRead,
            ]),
            "supervisor_agent" => Ok(vec![
                Scope::AgentsManage,
                Scope::ConfigRead,
                Scope::MissionsExecute,
                Scope::MissionsWrite,
            ]),
            _ => Err(SecurityError::UnknownAgent(agent_id.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentToken {
    pub agent_id: String,
    pub token: String,
    pub scopes: Vec<Scope>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl AgentToken {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}
```

---

## Permission System Design

### Scope-Based Permissions

```rust
// src/security/permissions.rs
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Scope {
    // Mission management
    MissionsRead,
    MissionsWrite,
    MissionsExecute,
    
    // Agent management
    AgentsManage,
    
    // Storage access
    StorageRead,
    StorageWrite,
    
    // Configuration access
    ConfigRead,
    ConfigWrite,
    
    // Research and data access
    ResearchAccess,
    ExternalAPIs,
    
    // Team collaboration
    TeamRead,
    TeamWrite,
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scope_str = match self {
            Scope::MissionsRead => "missions:read",
            Scope::MissionsWrite => "missions:write",
            Scope::MissionsExecute => "missions:execute",
            Scope::AgentsManage => "agents:manage",
            Scope::StorageRead => "storage:read",
            Scope::StorageWrite => "storage:write",
            Scope::ConfigRead => "config:read",
            Scope::ConfigWrite => "config:write",
            Scope::ResearchAccess => "research:access",
            Scope::ExternalAPIs => "external:apis",
            Scope::TeamRead => "team:read",
            Scope::TeamWrite => "team:write",
        };
        write!(f, "{}", scope_str)
    }
}

/// Permission checking engine
pub struct PermissionEngine {
    user_scopes: Arc<RwLock<HashMap<String, Vec<Scope>>>>,
}

impl PermissionEngine {
    pub fn new() -> Self {
        Self {
            user_scopes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Check if user has required scope
    pub fn check_scope(&self, user_id: &str, required_scope: &Scope) -> Result<bool, SecurityError> {
        let scopes = self.user_scopes.read().unwrap();
        
        match scopes.get(user_id) {
            Some(user_scopes) => Ok(user_scopes.contains(required_scope)),
            None => Err(SecurityError::UserNotFound(user_id.to_string())),
        }
    }
    
    /// Check if user has all required scopes
    pub fn check_scopes(&self, user_id: &str, required_scopes: &[Scope]) -> Result<bool, SecurityError> {
        let scopes = self.user_scopes.read().unwrap();
        
        match scopes.get(user_id) {
            Some(user_scopes) => {
                let has_all = required_scopes.iter().all(|s| user_scopes.contains(s));
                Ok(has_all)
            },
            None => Err(SecurityError::UserNotFound(user_id.to_string())),
        }
    }
    
    /// Update user's scopes (called after authentication)
    pub fn update_user_scopes(&self, user_id: &str, scopes: Vec<Scope>) {
        let mut scopes_map = self.user_scopes.write().unwrap();
        scopes_map.insert(user_id.to_string(), scopes);
        
        tracing::debug!(
            user_id = %user_id,
            scope_count = %scopes.len(),
            "Updated user scopes"
        );
    }
    
    /// Revoke user's permissions
    pub fn revoke_user_scopes(&self, user_id: &str) {
        let mut scopes_map = self.user_scopes.write().unwrap();
        scopes_map.remove(user_id);
        
        tracing::info!(
            user_id = %user_id,
            "Revoked user permissions"
        );
    }
}

/// Macro for easy permission checking
macro_rules! require_scope {
    ($user_scopes:expr, $required_scope:expr) => {
        if !$user_scopes.contains(&$required_scope) {
            return Err(SecurityError::InsufficientPermissions {
                required: $required_scope.to_string(),
                user_scopes: $user_scopes.iter().map(|s| s.to_string()).collect(),
            });
        }
    };
}

/// Macro for multiple scope checking
macro_rules! require_scopes {
    ($user_scopes:expr, [$($required_scope:expr),+]) => {
        $(
            require_scope!($user_scopes, $required_scope);
        )+
    };
}
```

---

## Security Middleware

### Request Authentication Middleware

```rust
// src/security/middleware.rs
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use tower::ServiceBuilder;

pub struct SecurityMiddleware {
    jwt_validator: JwtValidator,
    permission_engine: PermissionEngine,
    audit_logger: AuditLogger,
}

impl SecurityMiddleware {
    pub fn new() -> Self {
        Self {
            jwt_validator: JwtValidator::new(),
            permission_engine: PermissionEngine::new(),
            audit_logger: AuditLogger::new(),
        }
    }
    
    /// Tower middleware for request authentication
    pub fn layer() -> impl tower::Layer<SecurityService> {
        ServiceBuilder::new()
            .layer(axum::middleware::from_fn(auth_middleware))
    }
}

/// Authentication middleware for Axum
async fn auth_middleware(
    State(middleware): State<Arc<SecurityMiddleware>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Bearer token
    let token = match extract_bearer_token(request.headers()) {
        Some(token) => token,
        None => {
            middleware.audit_logger.log_security_event(&SecurityEvent {
                event_type: "auth_missing_token".to_string(),
                success: false,
                reason: Some("No bearer token provided".to_string()),
                ..Default::default()
            }).await;
            
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    // Validate JWT token
    let claims = match middleware.jwt_validator.validate_token(&token).await {
        Ok(claims) => claims,
        Err(e) => {
            middleware.audit_logger.log_security_event(&SecurityEvent {
                event_type: "auth_invalid_token".to_string(),
                success: false,
                reason: Some(format!("Token validation failed: {}", e)),
                ..Default::default()
            }).await;
            
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    
    // Check permission for requested operation
    let resource = extract_resource(&request);
    let action = extract_action(&request);
    
    if let Err(e) = middleware.permission_engine.check_scope(&claims.user_id, &action) {
        middleware.audit_logger.log_security_event(&SecurityEvent {
            event_type: "auth_permission_denied".to_string(),
            user_id: Some(claims.user_id.clone()),
            success: false,
            reason: Some(format!("Insufficient permissions: {}", e)),
            resource: Some(resource.to_string()),
            action: Some(action.to_string()),
            ..Default::default()
        }).await;
        
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Add user context to request extensions
    request.extensions_mut().insert(UserContext {
        user_id: claims.user_id,
        email: claims.email,
        scopes: claims.scopes,
    });
    
    // Log successful authentication
    middleware.audit_logger.log_security_event(&SecurityEvent {
        event_type: "auth_success".to_string(),
        user_id: Some(claims.user_id),
        success: true,
        resource: Some(resource.to_string()),
        action: Some(action.to_string()),
        ..Default::default()
    }).await;
    
    Ok(next.run(request).await)
}

fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|auth| {
            if auth.starts_with("Bearer ") {
                Some(auth[7..].to_string()) // Remove "Bearer " prefix
            } else {
                None
            }
        })
}

#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub email: String,
    pub scopes: Vec<Scope>,
}

/// Extract resource from request path
fn extract_resource(request: &Request) -> Resource {
    let path = request.uri().path();
    
    match path {
        p if p.starts_with("/api/missions") => Resource::Missions,
        p if p.starts_with("/api/agents") => Resource::Agents,
        p if p.starts_with("/api/storage") => Resource::Storage,
        p if p.starts_with("/api/config") => Resource::Config,
        _ => Resource::Unknown,
    }
}

/// Extract action from HTTP method
fn extract_action(request: &Request) -> Scope {
    match request.method() {
        &axum::http::Method::GET => Scope::MissionsRead,
        &axum::http::Method::POST => Scope::MissionsWrite,
        &axum::http::Method::PUT => Scope::MissionsWrite,
        &axum::http::Method::DELETE => Scope::MissionsWrite,
        _ => Scope::MissionsRead, // Default to read
    }
}

#[derive(Debug, Clone)]
pub enum Resource {
    Missions,
    Agents,
    Storage,
    Config,
    Unknown,
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let resource_str = match self {
            Resource::Missions => "missions",
            Resource::Agents => "agents",
            Resource::Storage => "storage",
            Resource::Config => "config",
            Resource::Unknown => "unknown",
        };
        write!(f, "{}", resource_str)
    }
}
```

---

## Audit Logging & Monitoring

### Security Event Tracking

```rust
// src/security/audit.rs
use serde::{Serialize, Deserialize};
use axiom_rs::AxiomClient;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct AuditLogger {
    axiom_client: AxiomClient,
    event_counter: AtomicU64,
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            axiom_client: AxiomClient::new(),
            event_counter: AtomicU64::new(0),
        }
    }
    
    /// Log security event to Axiom
    pub async fn log_security_event(&self, event: &SecurityEvent) {
        let event_id = self.event_counter.fetch_add(1, Ordering::Relaxed);
        
        let log_entry = serde_json::json!({
            "event_id": event_id,
            "timestamp": Utc::now(),
            "event_type": event.event_type,
            "user_id": event.user_id,
            "agent_id": event.agent_id,
            "resource": event.resource,
            "action": event.action,
            "ip_address": event.ip_address,
            "user_agent": event.user_agent,
            "success": event.success,
            "reason": event.reason,
            "severity": self.determine_severity(&event),
            "session_id": event.session_id,
        });
        
        // Send to Axiom security dataset
        if let Err(e) = self.axiom_client.ingest("mission_control_security", &log_entry).await {
            tracing::error!(
                error = %e,
                event_id = %event_id,
                "Failed to log security event to Axiom"
            );
        }
        
        // Log locally for debugging
        tracing::info!(
            event_id = %event_id,
            event_type = %event.event_type,
            user_id = ?event.user_id,
            success = %event.success,
            "Security event logged"
        );
    }
    
    /// Log agent operation with full context
    pub async fn log_agent_operation(&self, operation: &AgentOperation) {
        let log_entry = serde_json::json!({
            "timestamp": Utc::now(),
            "event_type": "agent_operation",
            "agent_id": operation.agent_id,
            "user_id": operation.user_id,
            "operation_type": operation.operation_type,
            "operation": operation.operation,
            "resource": operation.resource,
            "input_size": operation.input_size,
            "output_size": operation.output_size,
            "duration_ms": operation.duration_ms,
            "success": operation.success,
            "error": operation.error,
            "scopes_used": operation.scopes_used,
            "session_id": operation.session_id,
        });
        
        if let Err(e) = self.axiom_client.ingest("mission_control_agents", &log_entry).await {
            tracing::error!(
                error = %e,
                agent_id = %operation.agent_id,
                "Failed to log agent operation to Axiom"
            );
        }
    }
    
    /// Log permission changes
    pub async fn log_permission_change(&self, change: &PermissionChange) {
        let log_entry = serde_json::json!({
            "timestamp": Utc::now(),
            "event_type": "permission_change",
            "user_id": change.user_id,
            "agent_id": change.agent_id,
            "change_type": change.change_type,
            "old_scopes": change.old_scopes,
            "new_scopes": change.new_scopes,
            "reason": change.reason,
            "changed_by": change.changed_by,
            "session_id": change.session_id,
        });
        
        if let Err(e) = self.axiom_client.ingest("mission_control_permissions", &log_entry).await {
            tracing::error!(
                error = %e,
                user_id = %change.user_id,
                "Failed to log permission change to Axiom"
            );
        }
    }
    
    fn determine_severity(&self, event: &SecurityEvent) -> String {
        if !event.success {
            return "high".to_string();
        }
        
        match event.event_type.as_str() {
            "auth_success" | "agent_operation" => "info".to_string(),
            "auth_permission_denied" => "medium".to_string(),
            "auth_invalid_token" | "auth_missing_token" => "high".to_string(),
            "token_revoked" | "permission_revoked" => "medium".to_string(),
            _ => "info".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub event_type: String,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub resource: Option<String>,
    pub action: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub reason: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOperation {
    pub agent_id: String,
    pub user_id: String,
    pub operation_type: String,
    pub operation: String,
    pub resource: String,
    pub input_size: Option<u64>,
    pub output_size: Option<u64>,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    pub scopes_used: Vec<String>,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionChange {
    pub user_id: String,
    pub agent_id: Option<String>,
    pub change_type: String,
    pub old_scopes: Option<Vec<String>>,
    pub new_scopes: Vec<String>,
    pub reason: String,
    pub changed_by: String,
    pub session_id: String,
}
```

---

## Security Configuration

### Environment Variables

```bash
# Clerk Configuration
export CLERK_API_KEY="<your-clerk-api-key>"
export CLERK_JWKS_URL="https://api.clerk.com/v1/jwks"
export CLERK_API_URL="https://api.clerk.com/v1"
export CLERK_WEBHOOK_SECRET="whsec_xxxxxxxxxxxxxxxxxxxxxxxx"

# Security Configuration
export TOKEN_STORAGE_PATH="~/.mission_control/credentials"
export SESSION_TIMEOUT=86400  # 24 hours in seconds
export AGENT_TOKEN_LIFETIME=86400  # 24 hours in seconds
export MAX_FAILED_ATTEMPTS=5
export LOCKOUT_DURATION=900  # 15 minutes in seconds

# Axiom Configuration
export AXIOM_DATASET="mission_control_security"
export AXIOM_API_TOKEN="xoat_xxxxxxxxxxxxxxxxxxxxxxxx"

# JWT Configuration
export JWT_ALGORITHM="RS256"
export JWT_ISSUER="https://clerk.com"
export JWT_AUDIENCE="mission-control"

# Rate Limiting
export RATE_LIMIT_REQUESTS=100
export RATE_LIMIT_WINDOW=60000  # 1 minute in milliseconds
export AGENT_RATE_LIMIT_REQUESTS=1000
export AGENT_RATE_LIMIT_WINDOW=60000
```

### Configuration File Format

```toml
# config/security.toml

[clerk]
api_key_env = "CLERK_API_KEY"
jwks_url = "https://api.clerk.com/v1/jwks"
api_url = "https://api.clerk.com/v1"
webhook_secret_env = "CLERK_WEBHOOK_SECRET"

[authentication]
session_timeout_seconds = 86400
agent_token_lifetime_seconds = 86400
max_failed_attempts = 5
lockout_duration_seconds = 900
token_storage_path = "~/.mission_control/credentials"

[permissions]
default_user_scopes = [
    "missions:read",
    "missions:write",
    "missions:execute",
    "storage:read",
    "storage:write",
    "config:read"
]

[rate_limiting]
requests_per_window = 100
window_ms = 60000
agent_requests_per_window = 1000
agent_window_ms = 60000

[monitoring]
axiom_dataset = "mission_control_security"
security_log_level = "info"
alert_threshold_failures = 5
alert_threshold_duration = 300  # 5 minutes

[security_headers]
strict_transport_security = "max-age=31536000; includeSubDomains"
content_security_policy = "default-src 'self'"
x_frame_options = "DENY"
x_content_type_options = "nosniff"
```

---

## Security Best Practices

### Token Management

1. **Secure Storage**
   - Use OS keychain (not filesystem)
   - Never log or print API keys
   - Implement automatic cleanup on logout

2. **Token Lifecycle**
   - 24-hour expiration for user sessions
   - Automatic rotation for agent tokens
   - Immediate revocation capability
   - Graceful degradation on expiration

3. **Scope Limitation**
   - Minimum privilege principle
   - Agent-specific scope sets
   - Regular scope audits
   - Context-aware permission checks

### Network Security

1. **Transport Security**
   - TLS 1.3 for all API calls
   - Certificate pinning for Clerk endpoints
   - Secure DNS resolution
   - HTTP/2 where possible

2. **API Security**
   - Request signing for critical operations
   - Rate limiting per user and agent
   - Request size limits
   - Input validation and sanitization

### Monitoring & Alerting

1. **Real-time Monitoring**
   - All security events to Axiom
   - Immediate alerts on suspicious patterns
   - Dashboard for security metrics
   - Integration with error handling system

2. **Automated Responses**
   - Account lockout on repeated failures
   - Automatic token revocation on anomalies
   - Circuit breaker integration
   - Escalation procedures

---

## Dependencies

### Required Dependencies (Cargo.toml)

```toml
[dependencies]
# Security & Authentication
clerk-rs = "0.4"
jsonwebtoken = "0.17"
ring = "0.16"

# Keychain Storage
keyring = "2.0"
rpassword = "7.0"

# Permissions & Validation
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Web Framework & Middleware
axum = "0.7"
tower = { version = "0.5", features = ["full"] }
tower-http = "0.5"

# Monitoring & Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
axiom-rs = "0.3"

# Time & Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }

# Async Runtime
tokio = { version = "1.0", features = ["full"] }

# HTTP Client
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }

# Config Management
config = "0.13"
toml = "0.8"

# Error Handling
thiserror = "1.0"
anyhow = "1.0"
```

---

## Integration with Implementation Roadmap

### Phase 1: Foundation (Weeks 1-2)

**Security Tasks**:
- [ ] TASK-SEC-001: Set up Clerk integration and authentication module
- [ ] TASK-SEC-002: Implement API key authentication in CLI
- [ ] TASK-SEC-003: Add OS keychain storage for tokens
- [ ] TASK-SEC-004: Create basic permission engine

### Phase 2: Agent System (Weeks 3-4)

**Security Tasks**:
- [ ] TASK-SEC-005: Implement agent token management
- [ ] TASK-SEC-006: Add permission middleware to API endpoints
- [ ] TASK-SEC-007: Create audit logging system
- [ ] TASK-SEC-008: Integrate security with agent coordination

### Phase 3: Data Management (Weeks 5-6)

**Security Tasks**:
- [ ] TASK-SEC-009: Add security to storage operations
- [ ] TASK-SEC-010: Implement privacy boundary enforcement
- [ ] TASK-SEC-011: Add team collaboration security
- [ ] TASK-SEC-012: Configure Axiom security monitoring

### Phase 4: Integration & Optimization (Weeks 7-8)

**Security Tasks**:
- [ ] TASK-SEC-013: Add security headers to all endpoints
- [ ] TASK-SEC-014: Implement rate limiting and throttling
- [ ] TASK-SEC-015: Create security dashboard integration
- [ ] TASK-SEC-016: Add security testing and validation

### Phase 5: Production Readiness (Weeks 9-10)

**Security Tasks**:
- [ ] TASK-SEC-017: Conduct security audit and penetration testing
- [ ] TASK-SEC-018: Document security procedures and runbooks
- [ ] TASK-SEC-019: Set up production security monitoring
- [ ] TASK-SEC-020: Create security incident response procedures

---

## Success Metrics

### Security KPIs

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Authentication Success Rate** | > 99% | Successful authentications / total attempts |
| **Permission Enforcement Rate** | 100% | All API calls properly permission-checked |
| **Security Event Coverage** | > 95% | Percentage of security events logged |
| **Token Rotation Compliance** | 100% | All tokens rotated before expiration |
| **Audit Log Completeness** | > 99% | All security operations properly logged |

### Security Health Metrics

- **Mean Time To Detection (MTTD)**: Target < 5 minutes
- **False Positive Rate**: Target < 1%
- **Account Lockout Accuracy**: Target > 95%
- **Security Dashboard Latency**: Target < 2 seconds
- **Incident Response Time**: Target < 15 minutes

---

## File Structure

```
src/
├── security/
│   ├── mod.rs              # Security module exports
│   ├── authentication.rs    # User authentication with Clerk
│   ├── agent_tokens.rs      # Agent token management
│   ├── permissions.rs       # Permission engine and scopes
│   ├── middleware.rs        # Security middleware for APIs
│   ├── audit.rs           # Security event logging
│   ├── jwt_validator.rs    # JWT token validation
│   └── keychain.rs        # OS keychain integration
├── config/
│   └── security.rs        # Security configuration management
└── telemetry/
    └── security.rs        # Security metrics and monitoring
```

---

## Cross-References

- Related to: [Error Handling Strategy](./08-error-handling-strategy.md)
- Related to: [Agent System Design](./02-agent-system-design.md)
- Related to: [Data Management Strategy](./03-data-management-strategy.md#privacy-security-architecture)
- Related to: [Implementation Roadmap](./06-implementation-roadmap.md)

---

## Implementation Notes

### Security Integration Benefits

1. **Simplified User Experience**: API key authentication eliminates OAuth browser complexity
2. **Secure Agent Delegation**: Scoped tokens ensure agents only get necessary permissions
3. **Comprehensive Audit Trail**: All security events tracked in Axiom
4. **Automatic Security**: Token rotation, permission checks, and monitoring handled automatically
5. **Production Ready**: Enterprise-grade security with rate limiting and monitoring

### Clerk Integration Advantages

- **Mature Authentication**: Proven authentication provider with excellent Rust SDK
- **Flexible Permissions**: Support for custom scopes and API keys
- **Built-in Security**: Rate limiting, abuse detection, and compliance features
- **Developer Friendly**: Excellent documentation and community support
- **Scalable**: Handles enterprise workloads with high reliability

### Security Trade-offs

- **Complexity vs. Simplicity**: Chose API key auth for simplicity over OAuth flow
- **Security vs. Convenience**: 24-hour token lifetime balances security and usability
- **Granularity vs. Management**: Scoped permissions provide security without overwhelming complexity

---

**Last Updated**: February 2026
**Next Review**: End of Phase 2 (Week 4)
**Document Owner**: Security Team