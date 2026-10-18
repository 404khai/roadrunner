//! Versioned all-started synthetic evidence, separate from completed-movement metrics.
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use roadrunner_dispatch::{
    ActionId, ActiveExecution, AppliedEffect, DispatchInstant, OperationalRevision,
    OperationalTransitionRecord, RiderId, ScheduleGeneration,
};
use serde::{Deserialize, Serialize};

/// Full original traffic evidence with shared immutable storage and copy-on-write edits.
///
/// Its wire representation remains the original ordered array, including normal
/// factors. Cloning shares storage; editing a cloned artifact cannot rewrite any
/// other action's original evidence. Deserialization does not infer shared identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ActionTraffic(Arc<Vec<crate::TrafficOverride>>);

impl From<Vec<crate::TrafficOverride>> for ActionTraffic {
    fn from(value: Vec<crate::TrafficOverride>) -> Self {
        Self(Arc::new(value))
    }
}
impl Deref for ActionTraffic {
    type Target = Vec<crate::TrafficOverride>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for ActionTraffic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}
impl Serialize for ActionTraffic {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_ref().serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ActionTraffic {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<crate::TrafficOverride>::deserialize(deserializer).map(Self::from)
    }
}

/// Original committed start and predictor-visible inputs only. No outcomes or scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartedAction {
    /// Committed owner; checked against every effect and final active record.
    pub rider: RiderId,
    /// Original frozen route, action, plan and generation zero.
    pub original: ActiveExecution,
    /// Revision immediately following the successful start transition.
    pub committed_revision: OperationalRevision,
    /// Original immutable static overlay, not later adopted traffic.
    pub traffic: ActionTraffic,
}

/// A schedule change is outcome lineage, never another logical road leg or predictor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleChange {
    /// Existing owner.
    pub rider: RiderId,
    /// Effective active record after intervention, preserving original route.
    pub execution: ActiveExecution,
    /// Authoritative order after successful delay.
    pub revision: OperationalRevision,
    /// Logical processing time of the synthetic intervention.
    pub at: DispatchInstant,
}

/// Exact unprocessed queue entry at the inclusive horizon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingEvent {
    /// Scheduled logical instant, not an observed endpoint.
    pub at: DispatchInstant,
    /// Original stable queue insertion identity.
    pub sequence: u64,
    /// Lossless tagged driver action, including stale attempts and future exogenous inputs.
    pub action: serde_json::Value,
}

/// Driver arrival-to-action join, independent of completed movement ordering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrivalEvidence {
    /// Original logical action, never inferred from movement ordering.
    pub action: ActionId,
    /// Actual committed owner at arrival.
    pub rider: RiderId,
    /// Accepted effective schedule generation; stale queue attempts are not observations.
    pub generation: ScheduleGeneration,
    /// Exact original processed movement event sequence.
    pub event_sequence: u64,
    /// Synthetic asserted occurrence reference.
    pub reported_at: DispatchInstant,
    /// Logical sample during driver processing, not physical time or HTTP ingress.
    pub processing_sample_at: DispatchInstant,
}

/// Immutable result trace from a fresh run; simulation logical clock has no civil epoch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyntheticTrace {
    /// Independently versioned evidence contract.
    pub semantics: String,
    /// Caller run token, or content-qualified replay token for `simulate`.
    pub run_identity: String,
    /// Every successfully committed road action, in commit order.
    pub starts: Vec<StartedAction>,
    /// All successful road-delay generations in commit order.
    pub schedules: Vec<ScheduleChange>,
    /// Authoritative recognized effects with original effect identities.
    pub effects: Vec<AppliedEffect>,
    /// Driver source arrival references joined to movement events and authority effects.
    pub arrivals: Vec<ArrivalEvidence>,
    /// Independent authority allocator denominator (fresh run begins at zero).
    pub committed_start_count: u64,
    /// Complete transition history for independent ordering reconciliation.
    pub history: Vec<OperationalTransitionRecord>,
    /// Complete active execution at the horizon; absent completed actions stay in starts.
    pub final_active: BTreeMap<RiderId, ActiveExecution>,
    /// Exact unprocessed actions, sorted by queue processing order.
    pub pending: Vec<PendingEvent>,
    /// Inclusive simulated observation boundary.
    pub horizon: DispatchInstant,
    /// Exact final operational boundary; no cross-run atomicity is claimed.
    pub final_revision: OperationalRevision,
}
