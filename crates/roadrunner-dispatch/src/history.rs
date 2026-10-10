//! Versioned inspection contracts. No persistence or event sourcing is implemented.
use crate::{DispatchInstant, OperationalRevision, PredictionIdentity};
use serde::Serialize;

/// Historical record category; these are records, not inbound operational commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HistoricalRecordKind {
    /// Command request/result envelope; registry is deferred to Phase 20.
    Command,
    /// Evaluated decision and declared coverage.
    Decision,
    /// Successfully committed assignment/plan/terms.
    Commitment,
    /// Accepted execution effect.
    Execution,
}
/// Independent version dimensions prevent silent historical reinterpretation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct HistoricalRecord<T> {
    /// Envelope storage/inspection schema, not a policy version.
    pub schema_version: u32,
    /// Explicit historical meaning.
    pub kind: HistoricalRecordKind,
    /// Namespace and source state revision.
    pub operational_revision: OperationalRevision,
    /// Supported original policy semantics identifier.
    pub policy_semantics: String,
    /// Canonical configuration digest (not merely a policy name).
    pub policy_configuration: String,
    /// Original optimizer semantics, not current implementation fallback.
    pub algorithm_semantics: String,
    /// Exact graph/profile/traffic/prediction/service contexts.
    pub prediction: PredictionIdentity,
    /// Original evaluated instant.
    pub evaluated_at: DispatchInstant,
    /// Actual commitment instant only when one occurred.
    pub committed_at: Option<DispatchInstant>,
    /// Recorded facts/results; inspection does not rerun an optimizer.
    pub payload: T,
}
/// Explicit inability to recompute history; callers must not substitute current inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HistoricalReplayUnavailable {
    /// Required original graph/prediction/source artifact not retained.
    MissingArtifact,
    /// Original transition/optimizer/policy implementation unsupported.
    UnsupportedSemantics,
    /// Record predates an authentic acceptance reference; no migration may invent it.
    MissingAcceptanceReference,
}
