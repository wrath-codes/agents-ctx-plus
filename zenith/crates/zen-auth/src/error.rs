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
