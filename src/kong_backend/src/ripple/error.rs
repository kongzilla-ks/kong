use candid::Deserialize;
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum RippleError {
    #[error("Public key retrieval error: {0}")]
    PublicKeyRetrievalError(String),

    #[error("Invalid public key format: {0}")]
    InvalidPublicKeyFormat(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Network validation error: {0}")]
    NetworkValidationError(String),

    #[error("RPC error: {0}")]
    RpcError(String),
}
