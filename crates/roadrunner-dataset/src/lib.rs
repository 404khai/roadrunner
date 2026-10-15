//! Phase 23 offline synthetic dataset construction. No fitting, metrics or inference.
mod bundle;
mod features;
mod labels;
mod source;
mod split;

pub use bundle::*;
pub use features::*;
pub use labels::*;
pub use source::*;
pub use split::*;

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Source, semantic or publication failure; never an imputed label.
#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    /// Unsupported or inconsistent retained source evidence.
    #[error("source contract: {0}")]
    Source(String),
    /// Bytes or derived content do not match their immutable evidence.
    #[error("integrity: {0}")]
    Integrity(String),
    /// Protected dependency crosses partitions or an assignment is missing.
    #[error("split contract: {0}")]
    Split(String),
    /// Invalid encoding, including nonfinite/invalid typed measurements.
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    /// File publication or source read failure.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
/// SHA-256 of exact recoverable bytes; not semantic verification by itself.
#[must_use]
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Deterministic JSON, preserving integers and roundtrip f64 (no JavaScript conversion).
/// # Errors
/// Rejects values which cannot be represented in JSON.
pub fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>, DatasetError> {
    // Value uses sorted map keys; arrays retain declared source/commit order.
    Ok(serde_json::to_vec(&serde_json::to_value(value)?)?)
}

/// Compiler and Cargo which actually built this extractor, not the current PATH default.
#[must_use]
pub fn build_toolchain() -> String {
    format!(
        "{}; {}",
        env!("ROADRUNNER_DATASET_RUSTC"),
        env!("ROADRUNNER_DATASET_CARGO")
    )
}
