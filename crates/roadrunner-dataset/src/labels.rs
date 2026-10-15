//! Evidence-qualified endpoint subtraction, independent of fulfillment success.
use roadrunner_core::geo::Seconds;
use roadrunner_dispatch::DispatchInstant;
use serde::{Deserialize, Serialize};

/// Producer identity does not imply measurement validity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProducerOrigin {
    /// Deterministic simulation driver.
    Simulator,
    /// External report source.
    ExternalReporter,
    /// Explicit controlled test.
    ControlledTest,
    /// Manual adapter.
    ManualIntegration,
    /// Historical origin not established.
    Unknown,
}
/// Scientific support independent of operational acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeasurementSupport {
    /// Deterministic synthetic clock and event coverage.
    SyntheticControlled,
    /// Clock-supported operational reports, not physical departure.
    OperationalReportInterval,
    /// Independently supported physical endpoints (not produced by this extractor).
    PhysicalMeasurementSupported,
    /// Missing historical measurement evidence.
    InsufficientEvidence,
}
/// Meaning of a value independently of its producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordMeaning {
    /// Source assertion.
    Report,
    /// Authoritative accepted domain transition.
    CommittedDomainFact,
    /// Deterministic route output.
    RoutingEstimate,
    /// Derived value with versioned parent references.
    DerivedValue,
}
/// Four independent provenance dimensions, attached per relevant value category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Origin, never inferred from a source's name.
    pub producer_origin: ProducerOrigin,
    /// Record semantics.
    pub record_meaning: RecordMeaning,
    /// Evidence-qualified scientific cohort.
    pub measurement_support: MeasurementSupport,
    /// Recoverable source references and transform version.
    pub derivation_lineage: Vec<String>,
}
/// Exactly one primary state per committed start; reasons remain separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Lifecycle {
    /// An accepted endpoint exists; exact-label eligibility is separately assessed.
    Observed,
    /// Source interval is still open; no supported non-arrival coverage.
    Pending,
    /// Explicit compatible observation coverage proves non-arrival through a bound.
    RightCensored,
    /// Closed source lacks sufficient endpoint/non-arrival evidence.
    MissingArrival,
    /// Conflicting evidence, invalid interval or credible dispute.
    DisputedOrInvalid,
}
/// Endpoint subtraction witness. No operational/physical clock conversion is implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClockWitness {
    /// Same deterministic run-relative f64 seconds; no epoch/timezone, drift or restart.
    SyntheticLogicalV1,
    /// Relationship, uncertainty or physical departure not established.
    Unsupported,
}
/// Inputs to lifecycle assessment; not accessible to feature construction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelEvidence {
    /// Logical start sampled at successful transition, not physical departure/COMMIT.
    pub start_effective_at: DispatchInstant,
    /// Original accepted source occurrence, never a rejected correction.
    pub arrival_reported_at: Option<DispatchInstant>,
    /// Source processing sample, not HTTP ingress.
    pub arrival_processing_sample_at: Option<DispatchInstant>,
    /// Inclusive extraction boundary.
    pub boundary: DispatchInstant,
    /// Explicit compatible non-arrival coverage bound; silence alone is insufficient.
    pub non_arrival_through: Option<DispatchInstant>,
    /// Whether observation collection is closed at this boundary.
    pub source_closed: bool,
    /// Credible evidence dispute under the immutable-as-of/v1 policy.
    pub disputed: bool,
    /// Endpoint/clock relationship evidence.
    pub clock: ClockWitness,
    /// Scientific support independent of lifecycle state.
    pub measurement: MeasurementSupport,
}
/// Immutable as-of label, with exact value and censor bound kept separate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelAssessment {
    /// One mutually exclusive primary partition.
    pub outcome: Lifecycle,
    /// Exact whole-leg duration, never an edge decomposition or missingness sentinel.
    pub duration_seconds: Option<Seconds>,
    /// Supported lower bound only; never enters exact metrics.
    pub lower_bound_seconds: Option<Seconds>,
    /// Overlapping reasons why exact evaluation is unavailable.
    pub reasons: Vec<String>,
    /// Original endpoint, boundary and clock evidence.
    pub evidence: LabelEvidence,
}
/// Assesses retained endpoint evidence without inventing clock compatibility.
#[must_use]
pub fn assess_label(e: LabelEvidence) -> LabelAssessment {
    let compatible = e.clock == ClockWitness::SyntheticLogicalV1
        && e.measurement == MeasurementSupport::SyntheticControlled;
    let mut reasons = Vec::new();
    if !compatible {
        reasons.push("UnsupportedClockOrMeasurement".into());
    }
    let arrival = e
        .arrival_reported_at
        .zip(e.arrival_processing_sample_at)
        .filter(|(_, processed)| *processed <= e.boundary);
    let mut exact = None;
    let mut bound = None;
    let outcome = if e.disputed {
        reasons.push("CredibleDispute".into());
        Lifecycle::DisputedOrInvalid
    } else if let Some((at, processed)) = arrival {
        if at < e.start_effective_at || at > processed || e.start_effective_at > e.boundary {
            reasons.push("InvalidEndpointOrdering".into());
            Lifecycle::DisputedOrInvalid
        } else {
            if compatible {
                exact = at.duration_since(e.start_effective_at).ok();
            }
            Lifecycle::Observed
        }
    } else if compatible
        && e.source_closed
        && e.non_arrival_through
            .is_some_and(|at| at >= e.start_effective_at && at <= e.boundary)
    {
        bound = e
            .non_arrival_through
            .and_then(|at| at.duration_since(e.start_effective_at).ok());
        reasons.push("NoExactArrival".into());
        Lifecycle::RightCensored
    } else if e.source_closed {
        reasons.push("NoAdmissibleArrivalOrCoverage".into());
        Lifecycle::MissingArrival
    } else {
        reasons.push("OpenObservationInterval".into());
        Lifecycle::Pending
    };
    LabelAssessment {
        outcome,
        duration_seconds: exact,
        lower_bound_seconds: bound,
        reasons,
        evidence: e,
    }
}
