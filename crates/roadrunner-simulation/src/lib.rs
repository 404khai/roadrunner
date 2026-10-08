//! Deterministic discrete-event execution over Roadrunner dispatch and world transitions.
//!
//! No wall clock, sleeps, or live data are consulted. Graph and exogenous scenario
//! inputs are fixed; static traffic changes affect only subsequently departing legs.

mod comparison;
mod engine;
mod event;
mod metrics;
mod scenario;

pub use comparison::{StrategyComparison, compare_strategies};
pub use engine::simulate;
pub use event::{ExecutedLeg, RecordedEvent, SimulationEvent};
pub use metrics::{Distribution, OrderOutcome, RiderMetrics, SimulationResult, SimulationSummary};
pub use metrics::{
    PredictionFailureCoverage, RealizedProtectionOutcome, SimulationFleetRecord,
    SimulationInsertionRecord, SimulationPredictionFailure, SimulationRecoveryRecord,
};
pub use scenario::{
    ActualReadiness, DispatchPolicy, DynamicChange, DynamicEvent, OrderInput, RiderInput,
    SimulationScenario, TrafficChange, TrafficOverride,
};

use roadrunner_core::cost::TrafficError;
use roadrunner_core::geo::UnitError;
use roadrunner_dispatch::{CommitError, DispatchEvaluationError, DispatchTimeError};
use thiserror::Error;

/// Scenario validation or deterministic execution failure; never a fabricated result.
#[derive(Debug, Error)]
pub enum SimulationError {
    /// Required Phase 17 planning input or whole-plan evaluation failed.
    #[error(transparent)]
    Pooling(#[from] roadrunner_dispatch::PoolingError),
    /// Invalid schema, identities, node references, configuration, or time bounds.
    #[error("invalid simulation scenario: {0}")]
    InvalidScenario(String),
    /// Shared immutable dispatch evaluation failed.
    #[error(transparent)]
    Dispatch(#[from] DispatchEvaluationError),
    /// A shared world transition rejected the requested execution.
    #[error(transparent)]
    Transition(#[from] CommitError),
    /// Clock advancement or epoch conversion failed.
    #[error(transparent)]
    Time(#[from] DispatchTimeError),
    /// Physical duration, distance, or metric overflow.
    #[error(transparent)]
    Measurement(#[from] UnitError),
    /// An invalid static traffic overlay was supplied.
    #[error(transparent)]
    Traffic(#[from] TrafficError),
    /// A committed delivery cannot execute a legal road leg.
    #[error("committed execution has no road route")]
    NoExecutionRoute,
    /// Monotonic event/decision counters exhausted their integer domain.
    #[error("simulation sequence counter exhausted")]
    SequenceExhausted,
}
