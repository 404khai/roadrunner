use std::path::PathBuf;

use thiserror::Error;

/// Failure while extracting, validating, or compiling OpenStreetMap data.
#[derive(Debug, Error)]
pub enum OsmError {
    /// A source or artifact filesystem operation failed.
    #[error("{operation} failed for {path}: {source}")]
    Io {
        /// Operation being attempted.
        operation: &'static str,
        /// Affected path.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
    /// PBF decoding failed.
    #[error("PBF decoding failed: {source}")]
    Pbf {
        /// Underlying decoder error.
        #[source]
        source: osmpbf::Error,
    },
    /// JSON encoding or decoding failed.
    #[error("OSM artifact serialization failed: {source}")]
    Serialization {
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },
    /// Artifact framing is not recognized.
    #[error("invalid normalized OSM artifact magic")]
    InvalidMagic,
    /// Artifact schema is unsupported.
    #[error("unsupported normalized OSM artifact schema {version}")]
    UnsupportedSchema {
        /// Rejected schema version.
        version: u32,
    },
    /// Payload integrity does not match its envelope.
    #[error("normalized OSM artifact integrity hash does not match payload")]
    IntegrityMismatch,
    /// A source or normalized-data invariant failed.
    #[error("invalid OSM data: {message}")]
    InvalidData {
        /// Explainable validation failure.
        message: String,
    },
    /// Core graph construction rejected compiled data.
    #[error("graph compilation failed: {source}")]
    Graph {
        /// Core graph error.
        #[source]
        source: roadrunner_core::graph::GraphError,
    },
}

impl From<osmpbf::Error> for OsmError {
    fn from(source: osmpbf::Error) -> Self {
        Self::Pbf { source }
    }
}

impl From<serde_json::Error> for OsmError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialization { source }
    }
}

impl From<roadrunner_core::graph::GraphError> for OsmError {
    fn from(source: roadrunner_core::graph::GraphError) -> Self {
        Self::Graph { source }
    }
}

pub(crate) fn invalid(message: impl Into<String>) -> OsmError {
    OsmError::InvalidData {
        message: message.into(),
    }
}
