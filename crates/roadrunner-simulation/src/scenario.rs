use roadrunner_core::cost::TrafficMultiplier;
use roadrunner_core::geo::Seconds;
use serde::{Deserialize, Serialize};

/// Versioned immutable simulation inputs; numeric times are logical seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationScenario {
    /// Stable scenario/version identity, required for schema 2/3/4 pooled policies.
    #[serde(default)]
    pub scenario_id: Option<String>,
    /// 1 legacy; 2 insertion; 3 fleet batches; 4 dynamic committed recovery.
    pub schema_version: u32,
    /// Explicit seed for the versioned readiness generator, even for fixed scenarios.
    pub seed: u64,
    /// Beginning of the measurement window.
    pub start_seconds: Seconds,
    /// Inclusive end of execution; unfinished work is retained in results.
    pub end_seconds: Seconds,
    /// Dispatch instant corresponding to routing departure second zero.
    pub routing_epoch_seconds: Seconds,
    /// Optional exact graph binding, checked before any event is executed.
    #[serde(default)]
    pub graph_snapshot_digest: Option<String>,
    /// Independently callable single-order policy, including Phase 16 comparison objectives.
    pub dispatch: DispatchPolicy,
    /// Stable initial rider profiles and positions.
    pub riders: Vec<RiderInput>,
    /// External order creation and readiness inputs.
    pub orders: Vec<OrderInput>,
    /// Initial complete static traffic overlay; missing edges are normal.
    #[serde(default)]
    pub initial_traffic: Vec<TrafficOverride>,
    /// Each update replaces the complete overlay, without changing in-flight legs.
    #[serde(default)]
    pub traffic_changes: Vec<TrafficChange>,
    /// Schema 4 logical-time recovery triggers, ordered by input index at equal times.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dynamic_events: Vec<DynamicEvent>,
}

/// Dispatch choice for an independent deterministic run.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DispatchPolicy {
    /// Phase 19 committed recovery followed by Phase 18 admission (schema 4).
    Dynamic {
        /// Deterministic complete candidate submission budget per decision.
        work_budget: u64,
        /// Explicit forecast freshness duration.
        forecast_validity_seconds: Seconds,
        /// Admission algorithm; recovery uses its separately named neighborhood.
        algorithm: roadrunner_dispatch::FleetAlgorithm,
        /// Versioned explicit churn policy.
        recovery: roadrunner_dispatch::RecoveryPolicy,
    },
    /// Phase 18 joint batch allocation/resequencing (requires scenario schema 3).
    FleetBatch {
        /// Complete fleet submissions permitted per decision.
        work_budget: u64,
        /// Explicit forecast validity since creation.
        forecast_validity_seconds: Seconds,
        /// Deterministic construction/local-search variant.
        algorithm: roadrunner_dispatch::FleetAlgorithm,
    },
    /// Phase 17 exhaustive one-order insertion (requires scenario schema 2).
    MultiOrder {
        /// Complete candidate submissions permitted per admission attempt.
        work_budget: u64,
        /// Explicit forecast validity since order creation.
        forecast_validity_seconds: Seconds,
    },
    /// Pickup plus delivery road travel, ignoring preparation in predictions.
    Basic,
    /// Nearest straight-line pickup distance among feasible riders.
    NearestRider,
    /// Lowest traffic-aware pickup road travel.
    LowestPickupEta,
    /// Lowest readiness-aware completion time, without a waiting penalty.
    LowestCompletionTime,
    /// Readiness-aware completion plus weighted pickup waiting.
    PreparationAware {
        /// Finite non-negative score seconds per second of pickup waiting.
        idle_penalty_weight: f64,
    },
}

/// Initial rider capabilities and graph-backed authoritative coordinate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiderInput {
    /// Canonical dispatch rider identity.
    pub id: u64,
    /// Snapshot-local node defining the initial rider coordinate.
    pub node: u32,
    /// Normalized reference-parcel slots.
    pub capacity: u64,
    /// Initial new-work availability; schema 4 supports explicit updates.
    pub available: bool,
}

/// Fixed external order facts; pickup/dropoff nodes represent store/customer locations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderInput {
    /// Required per-order service/protection policy for schema 2/3 pooled policies.
    #[serde(default)]
    pub admission: Option<roadrunner_dispatch::OrderPolicy>,
    /// Canonical dispatch order identity.
    pub id: u64,
    /// Store/restaurant node.
    pub pickup_node: u32,
    /// Customer node.
    pub dropoff_node: u32,
    /// Creation event time, no earlier than scenario start.
    pub created_at_seconds: Seconds,
    /// Forecast, separate from the actual ready event.
    #[serde(default)]
    pub expected_ready_at_seconds: Option<Seconds>,
    /// Fixed observation or seeded preparation delay, materialized before dispatch.
    pub actual_readiness: ActualReadiness,
    /// Optional soft completed-dropoff deadline.
    #[serde(default)]
    pub deadline_seconds: Option<Seconds>,
    /// Normalized reference-parcel demand.
    pub demand: u64,
}

/// Actual readiness generation, independent of strategy and execution outcomes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActualReadiness {
    /// Fixed ready instant; already-ready stock may precede creation.
    Fixed {
        /// Observed ready time in the declared dispatch domain.
        at_seconds: Seconds,
    },
    /// Uniform seeded preparation duration added to creation time.
    SeededDelay {
        /// Inclusive lower bound in seconds.
        min_seconds: Seconds,
        /// Upper bound; the discrete 53-bit uniform source is in [0, 1).
        max_seconds: Seconds,
    },
}

/// One directed-edge override in a complete static traffic overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrafficOverride {
    /// Snapshot-local directed edge identity.
    pub edge_id: u32,
    /// Congestion factor at least 1.0.
    pub multiplier: TrafficMultiplier,
}

/// External traffic replacement event; equal-time replacements preserve input order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrafficChange {
    /// Effective simulation instant.
    pub at_seconds: Seconds,
    /// Complete replacement overrides; all omitted edges become normal.
    pub overrides: Vec<TrafficOverride>,
}

/// Versioned `SplitMix64` with exact 53-bit conversion; used only for scenario generation.
pub(crate) struct ReadinessRng(u64);
impl ReadinessRng {
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub(crate) fn uniform(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^= value >> 31;
        // At most 53 bits: this conversion is exact in f64, unlike general u64 casts.
        #[allow(clippy::cast_precision_loss)]
        let exact = (value >> 11) as f64;
        exact / 9_007_199_254_740_992.0
    }
}

/// Explicit externally supplied logical-time dynamic input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynamicEvent {
    /// Event receipt time, never host time.
    pub at_seconds: Seconds,
    /// Domain change or execution observation.
    pub change: DynamicChange,
}
/// Supported recovery inputs; unsupported cancellation/delay is recorded as refusal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DynamicChange {
    /// New-work availability, not physical immobilization.
    Availability {
        /// Rider identity.
        rider: u64,
        /// Eligibility for new assignments.
        available: bool,
    },
    /// Replace readiness estimate; never overwrite an actual observation.
    Forecast {
        /// Order identity.
        order: u64,
        /// New forecast ready instant.
        expected_at_seconds: Seconds,
    },
    /// Cancel only unstarted awaiting-pickup work.
    Cancel {
        /// Order identity.
        order: u64,
    },
    /// Observed delay of a currently departed road leg; retain its path/destination.
    RoadDelay {
        /// Rider identity.
        rider: u64,
        /// Observed additional duration on the pinned leg.
        additional_seconds: Seconds,
    },
}

#[cfg(test)]
mod tests {
    use super::ReadinessRng;
    #[test]
    fn generator_v1_preserves_the_seed_zero_reference_vector() {
        let mut rng = ReadinessRng::new(0);
        // SplitMix64's first seed-zero word is 0xe220a8397b1dcdaf.
        assert_eq!(rng.uniform().to_bits(), 0x3fec_4415_072f_63b9);
    }
}
