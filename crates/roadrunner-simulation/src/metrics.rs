use roadrunner_core::geo::{Meters, Seconds};
use roadrunner_dispatch::{AssignmentDecision, DispatchInstant, OrderId, RiderId};
use serde::Serialize;

use crate::{DispatchPolicy, RecordedEvent, SimulationError};

/// Seconds distribution with explicit sample population and unavailable empty statistics.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Distribution {
    /// Number of included observations.
    pub samples: usize,
    /// Arithmetic mean; absent with zero observations.
    pub mean_seconds: Option<Seconds>,
    /// Median; even populations use the midpoint of their central pair.
    pub median_seconds: Option<Seconds>,
    /// Nearest-rank 95th percentile.
    pub p95_seconds: Option<Seconds>,
    /// Nearest-rank 99th percentile.
    pub p99_seconds: Option<Seconds>,
}

impl Distribution {
    pub(crate) fn from_values(mut values: Vec<Seconds>) -> Result<Self, SimulationError> {
        values.sort_by(|a, b| a.value().total_cmp(&b.value()));
        let n = values.len();
        if n == 0 {
            return Ok(Self {
                samples: 0,
                mean_seconds: None,
                median_seconds: None,
                p95_seconds: None,
                p99_seconds: None,
            });
        }
        let sum = values
            .iter()
            .try_fold(Seconds::ZERO, |sum, v| sum.checked_add(*v))?;
        let median = if n % 2 == 0 {
            let lower = values[n / 2 - 1].value();
            Seconds::new(lower + (values[n / 2].value() - lower) / 2.0)?
        } else {
            values[n / 2]
        };
        let percentile = |percent: usize| -> Result<Seconds, SimulationError> {
            let index = n
                .checked_mul(percent)
                .ok_or(SimulationError::SequenceExhausted)?
                .div_ceil(100)
                - 1;
            Ok(values[index])
        };
        Ok(Self {
            samples: n,
            mean_seconds: Some(Seconds::new(sum.value() / count(n)?)?),
            median_seconds: Some(median),
            p95_seconds: Some(percentile(95)?),
            p99_seconds: Some(percentile(99)?),
        })
    }
}

pub(crate) fn count(n: usize) -> Result<f64, SimulationError> {
    Ok(f64::from(u32::try_from(n).map_err(|_| {
        SimulationError::InvalidScenario("population exceeds u32 metric domain".into())
    })?))
}

/// Per-order inputs and executed facts; absent milestones represent unfinished work.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OrderOutcome {
    /// Scheduled request identity, including requests not created before the horizon.
    pub order: OrderId,
    /// Actual ready event materialized before execution, independent of dispatch policy.
    pub realized_ready_at: DispatchInstant,
    /// Creation processed within the run window.
    pub created_at: Option<DispatchInstant>,
    /// Actual readiness fact recorded by a processed event.
    pub observed_ready_at: Option<DispatchInstant>,
    /// Shared responsibility commit time.
    pub assigned_at: Option<DispatchInstant>,
    /// Assigned rider, retained after completion for analysis.
    pub rider: Option<RiderId>,
    /// Predicted evaluation-to-completion duration from the committed decision.
    pub predicted_eta: Option<Seconds>,
    /// Actual store/restaurant arrival.
    pub pickup_arrival: Option<DispatchInstant>,
    /// Actual shared pickup completion.
    pub picked_up_at: Option<DispatchInstant>,
    /// Actual shared dropoff completion.
    pub delivered_at: Option<DispatchInstant>,
    /// Soft deadline, separate from completion predictions.
    pub deadline: Option<DispatchInstant>,
    /// Observed waiting after arrival but before horizon for an unfinished pickup.
    pub partial_wait_seconds: Option<Seconds>,
}

/// Responsibility utilization and observed completed-leg distance for one rider.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RiderMetrics {
    /// Rider identity.
    pub rider: RiderId,
    /// Operational availability throughout the configured window.
    pub available: bool,
    /// Assignment-to-delivery responsibility intervals, clipped at the horizon.
    pub busy_seconds: Seconds,
    /// Available time without responsibility; absent for unavailable riders.
    pub idle_seconds: Option<Seconds>,
    /// Distance of road legs that arrived within the horizon.
    pub completed_distance_meters: Meters,
}

/// Versioned aggregate metrics with explicit unfinished and uncreated populations.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulationSummary {
    /// All scenario requests, even those scheduled after the horizon.
    pub scheduled_orders: usize,
    /// Requests whose creation event was processed.
    pub created_orders: usize,
    /// Requests scheduled beyond the horizon.
    pub uncreated_orders: usize,
    /// Requests assigned at least once; this phase permits one responsibility.
    pub assigned_orders: usize,
    /// Completed dropoffs.
    pub delivered_orders: usize,
    /// Created requests without an assignment at the horizon.
    pub unassigned_orders: usize,
    /// Assigned requests not delivered by the horizon.
    pub assigned_unfinished_orders: usize,
    /// All created requests not delivered, including never-assigned work.
    pub outstanding_orders: usize,
    /// Delivered requests whose observed completed dropoff missed the deadline.
    pub late_deliveries: usize,
    /// Outstanding requests already past their deadline at the horizon.
    pub outstanding_past_deadline: usize,
    /// Predicted ETA population: committed assignments, including unfinished work.
    pub predicted_eta: Distribution,
    /// Observed delivery duration population: completed dropoffs, measured since creation.
    pub delivered_duration: Distribution,
    /// Observed pickup waiting population: completed pickups, including unfinished deliveries.
    pub pickup_waiting: Distribution,
    /// Completed-leg distance; in-flight partial distance is deliberately not fabricated.
    pub completed_distance_meters: Meters,
    /// Busy fraction of all initially available rider-seconds; absent for a zero denominator.
    pub rider_utilization: Option<f64>,
    /// Mean available rider time without committed responsibility.
    pub mean_rider_idle_seconds: Option<Seconds>,
}

/// Full deterministic artifact; contains no wall-clock timing or machine-specific values.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulationResult {
    /// Named schema 2 scenario identity; absent for historical schema 1 fixtures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario_id: Option<String>,
    /// Output schema version.
    pub schema_version: u32,
    /// Stable seeded generator identifier.
    pub randomness: String,
    /// Explicit input seed.
    pub seed: u64,
    /// Immutable road graph identity.
    pub graph_snapshot_digest: String,
    /// Graph population for reproducibility and external benchmarks.
    pub graph_nodes: usize,
    /// Directed-edge population.
    pub graph_edges: usize,
    /// Policy used for all assignments.
    pub dispatch: DispatchPolicy,
    /// Configured measurement start.
    pub started_at: DispatchInstant,
    /// Inclusive configured horizon, even when the queue becomes empty earlier.
    pub ended_at: DispatchInstant,
    /// Remaining queued actions beyond the horizon.
    pub future_events: usize,
    /// Processed event trace in exact queue order.
    pub events: Vec<RecordedEvent>,
    /// All completed evaluations, including valid Unassigned attempts.
    pub decisions: Vec<AssignmentDecision>,
    /// Phase 17 typed unavailable-input attempts; no proposal or domain mutation.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub prediction_failures: Vec<SimulationPredictionFailure>,
    /// Phase 17 evaluated attempts and atomic publication outcome.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub insertions: Vec<SimulationInsertionRecord>,
    /// Realized outcomes against admission terms, distinct from predicted feasibility.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub realized_protections: Vec<RealizedProtectionOutcome>,
    /// Final domain state for schema 2, including authentic accepted protections.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_state: Option<roadrunner_dispatch::WorldData>,
    /// Canonical order outcomes, sorted by `OrderId`.
    pub orders: Vec<OrderOutcome>,
    /// Canonical rider metrics, sorted by `RiderId`.
    pub riders: Vec<RiderMetrics>,
    /// Aggregates derived from recorded facts and intervals.
    pub summary: SimulationSummary,
}

pub(crate) fn summarize(
    orders: &[OrderOutcome],
    riders: &[RiderMetrics],
    start: DispatchInstant,
    end: DispatchInstant,
) -> Result<SimulationSummary, SimulationError> {
    let created = orders.iter().filter(|o| o.created_at.is_some()).count();
    let assigned = orders.iter().filter(|o| o.assigned_at.is_some()).count();
    let delivered = orders.iter().filter(|o| o.delivered_at.is_some()).count();
    let durations = orders
        .iter()
        .filter_map(|o| o.delivered_at.zip(o.created_at))
        .map(|(at, created)| at.duration_since(created))
        .collect::<Result<Vec<_>, _>>()?;
    let waiting = orders
        .iter()
        .filter_map(|o| o.picked_up_at.zip(o.pickup_arrival))
        .map(|(at, arrival)| at.duration_since(arrival))
        .collect::<Result<Vec<_>, _>>()?;
    let mut distance = Meters::ZERO;
    let mut busy = Seconds::ZERO;
    let mut idle = Seconds::ZERO;
    let mut available = 0;
    for rider in riders {
        distance = distance.checked_add(rider.completed_distance_meters)?;
        if rider.available {
            available += 1;
            busy = busy.checked_add(rider.busy_seconds)?;
            idle = idle.checked_add(rider.idle_seconds.ok_or_else(|| {
                SimulationError::InvalidScenario("missing available rider interval".into())
            })?)?;
        }
    }
    let denominator = Seconds::new(end.duration_since(start)?.value() * count(available)?)?.value();
    let utilization = if denominator > 0.0 {
        let value = busy.value() / denominator;
        if !value.is_finite() || value > 1.0 {
            return Err(SimulationError::InvalidScenario(
                "invalid utilization aggregate".into(),
            ));
        }
        Some(value)
    } else {
        None
    };
    Ok(SimulationSummary {
        scheduled_orders: orders.len(),
        created_orders: created,
        uncreated_orders: orders.len() - created,
        assigned_orders: assigned,
        delivered_orders: delivered,
        unassigned_orders: created - assigned,
        assigned_unfinished_orders: assigned - delivered,
        outstanding_orders: created - delivered,
        late_deliveries: orders
            .iter()
            .filter(|o| o.delivered_at.zip(o.deadline).is_some_and(|(at, d)| at > d))
            .count(),
        outstanding_past_deadline: orders
            .iter()
            .filter(|o| {
                o.created_at.is_some()
                    && o.delivered_at.is_none()
                    && o.deadline.is_some_and(|d| end > d)
            })
            .count(),
        predicted_eta: Distribution::from_values(
            orders.iter().filter_map(|o| o.predicted_eta).collect(),
        )?,
        delivered_duration: Distribution::from_values(durations)?,
        pickup_waiting: Distribution::from_values(waiting)?,
        completed_distance_meters: distance,
        rider_utilization: utilization,
        mean_rider_idle_seconds: if available > 0 {
            Some(Seconds::new(idle.value() / count(available)?)?)
        } else {
            None
        },
    })
}

/// Planning evidence and actual atomic publication result, distinct from delivery outcomes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulationInsertionRecord {
    /// Read-only complete/incomplete search decision.
    pub decision: roadrunner_dispatch::InsertionDecision,
    /// Whether this exact proposed insertion was committed.
    pub committed: bool,
    /// Authoritative world version after this attempt.
    pub world_version_after: roadrunner_dispatch::WorldVersion,
}

/// Actual completed-dropoff comparison with immutable terms; admission is not a guarantee.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RealizedProtectionOutcome {
    /// Accepted order identity.
    pub order: OrderId,
    /// Authentic acceptance-time completed-dropoff prediction.
    pub accepted_completion: DispatchInstant,
    /// Actual completed dropoff, unavailable for unfinished work.
    pub actual_completion: Option<DispatchInstant>,
    /// Signed realized completion delay relative to acceptance.
    pub signed_delay_seconds: Option<f64>,
    /// Realized miss of the accepted hard deadline, unavailable if unfinished or soft.
    pub hard_deadline_missed: Option<bool>,
    /// Realized consumption exceeding accepted cumulative allowance, when configured.
    pub cumulative_allowance_exceeded: Option<bool>,
}

pub(crate) fn realized_protections(
    data: &roadrunner_dispatch::WorldData,
) -> Vec<RealizedProtectionOutcome> {
    data.accepted
        .iter()
        .map(|(order, terms)| {
            let actual = match data.fulfillment[order] {
                roadrunner_dispatch::FulfillmentState::Delivered { at, .. } => Some(at),
                _ => None,
            };
            let signed_delay = actual.map(|at| at.value() - terms.completion_reference.value());
            RealizedProtectionOutcome {
                order: *order,
                accepted_completion: terms.completion_reference,
                actual_completion: actual,
                signed_delay_seconds: signed_delay,
                hard_deadline_missed: if terms.policy.deadline
                    == roadrunner_dispatch::AdmissionDeadline::Hard
                {
                    actual
                        .zip(terms.deadline)
                        .map(|(at, deadline)| at > deadline)
                } else {
                    None
                },
                cumulative_allowance_exceeded: signed_delay
                    .zip(terms.policy.max_completion_delay)
                    .map(|(delay, allowance)| delay.max(0.0) > allowance.value()),
            }
        })
        .collect()
}

/// A typed failed admission with incomplete inputs; not completed infeasibility.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulationPredictionFailure {
    /// New order blocked from admission.
    pub order: OrderId,
    /// Order whose required prediction is unavailable.
    pub unavailable_order: OrderId,
    /// Exact logical evaluation instant.
    pub at: DispatchInstant,
    /// Coherent world identity.
    pub world_identity: u64,
    /// Unchanged authoritative world version.
    pub world_version: roadrunner_dispatch::WorldVersion,
    /// Independent input, rider and placement coverage for the failed attempt.
    pub coverage: PredictionFailureCoverage,
    /// Baseline/input failures occur before any placement submission.
    pub work: u64,
    /// Explicit failure kind/version.
    pub reason: String,
    /// Atomic publication never occurred.
    pub committed: bool,
}

/// Independent coverage dimensions for a failed admission, never feasibility.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PredictionFailureCoverage {
    /// Required input sufficiency was not established.
    pub input_complete: bool,
    /// Complete eligible-rider health was not established.
    pub riders_complete: bool,
    /// Complete placement search was not established.
    pub search_complete: bool,
}
