//! Phase 17 exhaustive single-order insertion; no fleet resequencing or reassignment.
use std::collections::BTreeMap;

use roadrunner_core::geo::{Meters, Seconds};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    Availability, CapacityUnits, CommitError, CommittedAssignment, DispatchEvaluationError,
    DispatchInstant, DispatchSnapshot, DispatchTimeError, FulfillmentState, OrderId,
    PlanValidityError, RiderId, RiderPlan, RoutingAnchor, RoutingAnchors, RoutingEpoch,
    RoutingProvenance, Stop, StopTimeline, World, WorldData, WorldVersion, onboard_load,
    validate_plan, validate_world,
};

/// Per-order completed-dropoff deadline admission semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdmissionDeadline {
    /// Historical observation-only target.
    SoftObserved,
    /// Requires deadline data and rejects predicted completed-dropoff misses.
    Hard,
}

/// Required readiness semantics; fallback is explicitly named and versioned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadinessRule {
    /// Phase 13–16 expectation semantics, without invented forecast validity.
    LegacyV1,
    /// Requires observed readiness or a currently valid forecast.
    ValidForecastV1,
    /// Use order creation when prediction unavailable; explicit stock-ready assumption.
    CreatedAtFallbackV1,
}

/// Explicit versioned per-order admission and deterministic stop policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderPolicy {
    /// Stable policy identity.
    pub id: String,
    /// Supported semantic version, currently 1.
    pub version: u32,
    /// Deadline feasibility rule.
    pub deadline: AdmissionDeadline,
    /// Optional cumulative delay allowance, with no universal default.
    pub max_completion_delay: Option<Seconds>,
    /// Deterministic pickup service.
    pub pickup_service: Seconds,
    /// Deterministic dropoff service.
    pub dropoff_service: Seconds,
    /// Forecast validity or explicit fallback semantics.
    pub readiness: ReadinessRule,
}
impl OrderPolicy {
    /// Explicit Phase 13–16 compatibility; does not invent acceptance history.
    #[must_use]
    pub fn legacy() -> Self {
        Self {
            id: "legacy-phase13-16/v1".into(),
            version: 1,
            deadline: AdmissionDeadline::SoftObserved,
            max_completion_delay: None,
            pickup_service: Seconds::ZERO,
            dropoff_service: Seconds::ZERO,
            readiness: ReadinessRule::LegacyV1,
        }
    }
    /// Validates the explicitly supported policy identity and version.
    ///
    /// # Errors
    /// Rejects unknown versions and unsupported legacy protection migration.
    pub fn validate(&self) -> Result<(), PoolingError> {
        if self.version != 1 || self.id.is_empty() {
            return Err(PoolingError::UnsupportedPolicy);
        }
        if self.readiness == ReadinessRule::LegacyV1 && self.max_completion_delay.is_some() {
            return Err(PoolingError::UnsupportedMigration);
        }
        Ok(())
    }
}

/// Freshness metadata for the expectation stored in the coherent world.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReadinessForecast {
    /// Prediction identity/version supplied by the caller.
    pub id: String,
    /// Forecast creation instant, which cannot be in the future.
    pub generated_at: DispatchInstant,
    /// Inclusive validity endpoint at the logical evaluation instant.
    pub valid_until: DispatchInstant,
    /// Exact expectation to which validity belongs.
    pub expected_at: DispatchInstant,
}

/// Immutable acceptance-time service protections, established only by commit.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AcceptedTerms {
    /// Exact policy at acceptance, never reset on plan rewrite.
    pub policy: OrderPolicy,
    /// Successful assignment's evaluation instant.
    pub accepted_at: DispatchInstant,
    /// Authentic completed-dropoff prediction.
    pub completion_reference: DispatchInstant,
    /// Deadline data pinned at acceptance.
    pub deadline: Option<DispatchInstant>,
    /// Exact observed/forecast readiness inputs and source at acceptance.
    pub readiness: AcceptedReadiness,
    /// Prediction and execution context used at acceptance.
    pub provenance: PredictionIdentity,
}

/// Readiness prediction provenance pinned with authoritative acceptance terms.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AcceptedReadiness {
    /// Prediction value, separate from observation.
    pub expected_at: Option<DispatchInstant>,
    /// Actual ready fact available at acceptance.
    pub observed_at: Option<DispatchInstant>,
    /// Named/versioned effective readiness source.
    pub source: String,
    /// Exact validity metadata available at acceptance.
    pub forecast: Option<ReadinessForecast>,
}

/// Relevant non-world prediction policy identities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PredictionIdentity {
    /// Versioned prediction model/scenario identity.
    pub prediction: String,
    /// Versioned service configuration identity.
    pub service: String,
    /// Versioned optimizer configuration identity.
    pub optimizer: String,
    /// Exact routing graph/traffic/profile context.
    pub routing: RoutingProvenance,
}

/// Active execution plus destination wait/service, immutable during admission.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrozenPrefix {
    /// Stable active execution identity, independent of plan revisions.
    pub execution_id: u64,
    /// Complete predicted destination-stop timing including remaining road movement.
    pub timeline: StopTimeline,
    /// Explicit projected destination anchor.
    pub anchor: RoutingAnchor,
}

/// Explicit planning origin; no inference from stale rider positions during execution.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecutionProjection {
    /// Frozen action, absent only when no action has started.
    pub frozen: Option<FrozenPrefix>,
    /// Projected post-prefix departure (or evaluation instant when idle).
    pub at: DispatchInstant,
    /// Projected routing origin, not necessarily last completed endpoint.
    pub anchor: RoutingAnchor,
    /// Explicit post-prefix custody, checked against frozen effects.
    pub custody: BTreeMap<OrderId, DispatchInstant>,
    /// Explicit post-prefix onboard demand.
    pub load: CapacityUnits,
    /// Explicit editable suffix, checked against the effective committed plan.
    pub suffix: RiderPlan,
}

/// Complete caller-owned inputs, compared in full before publication.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PoolingInputs {
    /// Named prediction/service/optimizer/routing configuration.
    pub identity: PredictionIdentity,
    /// Current per-order policy, including explicit legacy compatibility entries.
    pub policies: BTreeMap<OrderId, OrderPolicy>,
    /// Validity metadata; absence does not imply ready.
    pub forecasts: BTreeMap<OrderId, ReadinessForecast>,
    /// Explicit state for every operationally eligible, supported rider.
    pub projections: BTreeMap<RiderId, ExecutionProjection>,
    /// Deterministic complete-candidate evaluation budget.
    pub work_budget: u64,
}

/// Exact context compared at publication; world version separately covers all domain facts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PoolingContext {
    /// Evaluation logical clock.
    pub at: DispatchInstant,
    /// Routing epoch.
    pub epoch: RoutingEpoch,
    /// Complete relevant input bundle.
    pub inputs: PoolingInputs,
    /// Exact graph projections.
    pub anchors: RoutingAnchors,
}
impl PoolingContext {
    /// Binds the caller's immutable inputs to a coherent dispatch snapshot.
    ///
    /// # Errors
    /// Rejects mismatched routing identity or unnamed policy contexts.
    pub fn new(
        snapshot: &DispatchSnapshot<'_>,
        inputs: PoolingInputs,
    ) -> Result<Self, PoolingError> {
        Self::bind(snapshot, inputs, "exhaustive-insertion/v1")
    }

    pub(crate) fn bind(
        snapshot: &DispatchSnapshot<'_>,
        inputs: PoolingInputs,
        optimizer: &str,
    ) -> Result<Self, PoolingError> {
        if inputs.identity.routing != snapshot.provenance
            || inputs.identity.prediction.is_empty()
            || inputs.identity.service.is_empty()
            || inputs.identity.optimizer != optimizer
        {
            return Err(PoolingError::InvalidContext);
        }
        Ok(Self {
            at: snapshot.at,
            epoch: snapshot.epoch,
            inputs,
            anchors: snapshot.anchors.clone(),
        })
    }
}

/// Evaluation failures are distinct from a completed infeasible or incomplete search.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum PoolingError {
    /// Shared world/routing/time validation failure.
    #[error(transparent)]
    Evaluation(#[from] DispatchEvaluationError),
    /// Checked logical time failure.
    #[error(transparent)]
    Time(#[from] DispatchTimeError),
    /// Required prediction is absent, stale, or invalid.
    #[error("unavailable required readiness forecast for {0:?}")]
    PredictionUnavailable(OrderId),
    /// Policy identity/version is unsupported or required policy data is absent.
    #[error("unsupported or missing admission policy")]
    UnsupportedPolicy,
    /// Existing responsibility lacks an authentic reference required by current policy.
    #[error("cumulative protection requires authentic acceptance terms")]
    UnsupportedMigration,
    /// Explicit projection/context disagrees with the coherent world.
    #[error("invalid execution projection or policy context")]
    InvalidContext,
    /// Required baseline cannot be routed; its health is unknown.
    #[error("baseline routing unavailable for {0:?}")]
    BaselineUnavailable(RiderId),
}

/// Typed infeasibility, never an objective penalty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum InsertionRejection {
    /// Capacity at some prefix is exceeded.
    Capacity,
    /// No legal road route.
    NoRoute,
    /// Completed dropoff violates an applicable hard deadline.
    HardDeadline(OrderId),
    /// Cumulative consumed delay exceeds immutable accepted allowance.
    CumulativeDelay(OrderId),
}

/// Per-stop whole-plan timing and onboard demand after completion.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvaluatedStop {
    /// Arrival, waiting, service and departure.
    pub timeline: StopTimeline,
    /// Road movement for this incoming editable leg.
    pub travel: Seconds,
    /// Incoming road distance.
    pub distance: Meters,
    /// Post-stop onboard demand.
    pub load: CapacityUnits,
    /// Effective readiness source, absent at dropoff.
    pub readiness_source: Option<String>,
}

/// Whole-plan physical evaluation, including all completed-dropoff predictions.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WholePlanEvaluation {
    /// Complete resulting plan including frozen first stop.
    pub plan: RiderPlan,
    /// Editable-horizon road seconds; frozen contribution excluded identically.
    pub travel: Seconds,
    /// Editable-horizon road distance.
    pub distance: Meters,
    /// Editable stop timelines, in execution order.
    pub stops: Vec<EvaluatedStop>,
    /// Completed-dropoff predictions, including a frozen dropoff if present.
    pub completions: BTreeMap<OrderId, DispatchInstant>,
    /// Hard protection breaches under both accepted and current policy.
    pub breaches: Vec<InsertionRejection>,
}

/// Per-order customer impact, separate from the road objective.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompletionImpact {
    /// Order identity.
    pub order: OrderId,
    /// Immutable authentic acceptance reference, absent for legacy work.
    pub accepted: Option<DispatchInstant>,
    /// Baseline completion, absent for newly admitted work.
    pub current: Option<DispatchInstant>,
    /// Candidate completed dropoff.
    pub candidate: DispatchInstant,
    /// Signed candidate-current seconds, informational.
    pub marginal_seconds: Option<f64>,
    /// Signed candidate-accepted seconds.
    pub cumulative_seconds: Option<f64>,
    /// Non-negative cumulative consumption; improvements do not bank allowance.
    pub consumed_seconds: Option<f64>,
    /// Signed candidate deadline slack, including soft targets.
    pub deadline_slack_seconds: Option<f64>,
    /// Slack against the independently retained accepted deadline.
    pub accepted_deadline_slack_seconds: Option<f64>,
    /// Remaining cumulative allowance under the stronger current/accepted bound.
    pub cumulative_allowance_remaining_seconds: Option<f64>,
}

/// Declared search accounting for one eligible rider.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RiderSearchEvidence {
    /// Canonical rider identity.
    pub rider: RiderId,
    /// Editable stop count.
    pub suffix_length: usize,
    /// Full insertion-pair count before policy exclusion.
    pub pairs_total: u64,
    /// Complete candidates submitted to evaluator.
    pub evaluated: u64,
    /// Correctness-preserving policy exclusions; only baseline-hard-breach exclusion.
    pub pruned: u64,
    /// Feasible candidate count.
    pub feasible: u64,
    /// Baseline timing and health.
    pub baseline: WholePlanEvaluation,
    /// Counted typed rejection reasons.
    pub rejections: Vec<(InsertionRejection, u64)>,
}

/// Search termination; incomplete searches cannot publish even a found candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum InsertionTermination {
    /// Best feasible placement over the complete declared search.
    BestInsertion,
    /// Complete eligible-fleet declared search contains no feasible insertion.
    NoFeasibleInsertion,
    /// Work budget exhausted; no proposal is publishable.
    SearchIncomplete,
}

/// Versioned semantic evidence; excludes timing/hardware measurements.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InsertionEvidence {
    /// Evidence contract version.
    pub schema_version: u32,
    /// Source world identity.
    pub world_identity: u64,
    /// Source coherent world version.
    pub world_version: WorldVersion,
    /// Input order identity.
    pub order: OrderId,
    /// Exact prediction, policy, projection and clock binding.
    pub context: PoolingContext,
    /// Input sufficiency; successful decisions always have complete required input.
    pub input_complete: bool,
    /// All eligible riders covered; independent of placement completion.
    pub riders_complete: bool,
    /// Every nonexcluded declared insertion evaluated.
    pub search_complete: bool,
    /// Typed structural exclusions (unavailable / unsupported profile).
    pub exclusions: Vec<(RiderId, String)>,
    /// Canonical rider search statistics, including blocked baseline health.
    pub riders: Vec<RiderSearchEvidence>,
    /// Complete evaluator submissions; baseline evaluations are outside this budget.
    pub work: u64,
    /// Scoped termination, with no arbitrary-resequencing optimum claim.
    pub termination: InsertionTermination,
    /// Road objective accounting boundary.
    pub accounting: String,
}

/// Immutable exact evaluated replacement, construction restricted to complete search.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InsertionProposal {
    pub(crate) rider: RiderId,
    pub(crate) expected_plan: RiderPlan,
    pub(crate) expected_assignment: Option<CommittedAssignment>,
    pub(crate) proposed_assignment: CommittedAssignment,
    pub(crate) existing_assignments: BTreeMap<OrderId, CommittedAssignment>,
    pub(crate) evaluation: WholePlanEvaluation,
    pub(crate) accepted: AcceptedTerms,
    pub(crate) incremental_travel: f64,
    pub(crate) incremental_distance: f64,
    pub(crate) impacts: Vec<CompletionImpact>,
}
impl InsertionProposal {
    /// Selected rider.
    #[must_use]
    pub const fn rider(&self) -> RiderId {
        self.rider
    }
    /// Exact evaluated replacement and timing.
    #[must_use]
    pub const fn evaluation(&self) -> &WholePlanEvaluation {
        &self.evaluation
    }
    /// Signed incremental remaining road seconds.
    #[must_use]
    pub const fn incremental_travel(&self) -> f64 {
        self.incremental_travel
    }
    /// Signed incremental road distance.
    #[must_use]
    pub const fn incremental_distance(&self) -> f64 {
        self.incremental_distance
    }
    /// Per-order accepted/current/candidate customer impacts.
    #[must_use]
    pub fn impacts(&self) -> &[CompletionImpact] {
        &self.impacts
    }
}

/// Complete or budget-incomplete read-only decision, containing no authoritative terms.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InsertionDecision {
    pub(crate) evidence: InsertionEvidence,
    pub(crate) proposal: Option<InsertionProposal>,
}
impl InsertionDecision {
    /// Coverage, exclusions, work and exact context evidence.
    #[must_use]
    pub const fn evidence(&self) -> &InsertionEvidence {
        &self.evidence
    }
    /// Publishable best insertion, present only after complete search.
    #[must_use]
    pub const fn proposal(&self) -> Option<&InsertionProposal> {
        self.proposal.as_ref()
    }
}

pub(crate) fn validate_accepted(data: &WorldData) -> Result<(), PoolingError> {
    for (order, accepted) in &data.accepted {
        accepted.policy.validate()?;
        if !data.orders.contains_key(order)
            || accepted.completion_reference < accepted.accepted_at
            || accepted.accepted_at < data.orders[order].created_at
            || (accepted.policy.deadline == AdmissionDeadline::Hard && accepted.deadline.is_none())
            || (matches!(
                data.fulfillment.get(order),
                Some(FulfillmentState::AwaitingPickup)
            ) && !data.assignments.contains_key(order))
            || accepted.provenance.prediction.is_empty()
            || accepted.provenance.service.is_empty()
            || accepted.provenance.optimizer.is_empty()
        {
            return Err(DispatchEvaluationError::InvalidWorldState.into());
        }
    }
    Ok(())
}

pub(crate) fn policy(inputs: &PoolingInputs, order: OrderId) -> Result<&OrderPolicy, PoolingError> {
    let p = inputs
        .policies
        .get(&order)
        .ok_or(PoolingError::UnsupportedPolicy)?;
    p.validate()?;
    Ok(p)
}

/// Resolves readiness without treating a past forecast as actual readiness.
///
/// # Errors
/// Missing/stale required forecasts are decision evaluation failures.
pub fn effective_readiness(
    data: &WorldData,
    inputs: &PoolingInputs,
    order: OrderId,
    at: DispatchInstant,
) -> Result<(DispatchInstant, String), PoolingError> {
    let p = policy(inputs, order)?;
    let ready = data
        .readiness
        .get(&order)
        .ok_or(DispatchEvaluationError::InvalidWorldState)?;
    if let Some(observed) = ready.observed_at {
        return Ok((observed, "observed/v1".into()));
    }
    if p.readiness == ReadinessRule::LegacyV1 {
        return ready
            .expected_at
            .map(|t| (t, "legacy-expectation/v1".into()))
            .ok_or(PoolingError::PredictionUnavailable(order));
    }
    if let Some(f) = inputs.forecasts.get(&order) {
        if !f.id.is_empty()
            && f.generated_at <= at
            && f.valid_until >= at
            && f.generated_at <= f.valid_until
            && ready.expected_at == Some(f.expected_at)
        {
            return Ok((f.expected_at, f.id.clone()));
        }
    }
    if p.readiness == ReadinessRule::CreatedAtFallbackV1 {
        return Ok((
            data.orders[&order].created_at,
            "created-at-fallback/v1".into(),
        ));
    }
    Err(PoolingError::PredictionUnavailable(order))
}

fn custody(data: &WorldData, rider: RiderId) -> BTreeMap<OrderId, DispatchInstant> {
    data.fulfillment
        .iter()
        .filter_map(|(o, s)| match s {
            FulfillmentState::PickedUp { rider: r, at } if *r == rider => Some((*o, *at)),
            _ => None,
        })
        .collect()
}

/// Builds and validates the explicit projected effects of the caller's frozen action.
///
/// # Errors
/// Rejects a changed frozen stop, wrong anchor/time, custody or load mismatch.
pub fn project_execution(
    data: &WorldData,
    rider: RiderId,
    at: DispatchInstant,
    origin: RoutingAnchor,
    frozen: Option<FrozenPrefix>,
) -> Result<ExecutionProjection, PoolingError> {
    validate_world(data)?;
    let mut projected = data.clone();
    let mut suffix = data
        .plans
        .get(&rider)
        .ok_or(PoolingError::InvalidContext)?
        .clone();
    let (after, anchor) = if let Some(f) = &frozen {
        let t = f.timeline;
        if suffix.stops.first() != Some(&t.stop)
            || t.departure < at
            || StopTimeline::new(t.stop, t.arrival, t.waiting, t.service)? != t
        {
            return Err(PoolingError::InvalidContext);
        }
        let order = &data.orders[&t.stop.order()];
        let coordinate = match t.stop {
            Stop::Pickup(_) => order.pickup,
            Stop::Dropoff(_) => order.dropoff,
        };
        if f.anchor.coordinate != coordinate {
            return Err(PoolingError::InvalidContext);
        }
        suffix.stops.remove(0);
        match t.stop {
            Stop::Pickup(o) => {
                projected.fulfillment.insert(
                    o,
                    FulfillmentState::PickedUp {
                        rider,
                        at: t.departure,
                    },
                );
            }
            Stop::Dropoff(o) => {
                let FulfillmentState::PickedUp {
                    rider: owner,
                    at: picked_up_at,
                } = data.fulfillment[&o]
                else {
                    return Err(PoolingError::InvalidContext);
                };
                if owner != rider {
                    return Err(PoolingError::InvalidContext);
                }
                projected.fulfillment.insert(
                    o,
                    FulfillmentState::Delivered {
                        rider,
                        picked_up_at,
                        at: t.departure,
                    },
                );
                projected.assignments.remove(&o);
            }
        }
        (t.departure, f.anchor.clone())
    } else {
        if origin.coordinate != data.riders[&rider].coordinate {
            return Err(PoolingError::InvalidContext);
        }
        (at, origin)
    };
    validate_plan(&projected, rider, &suffix).map_err(|_| PoolingError::InvalidContext)?;
    Ok(ExecutionProjection {
        frozen,
        at: after,
        anchor,
        custody: custody(&projected, rider),
        load: onboard_load(&projected, rider).map_err(|_| PoolingError::InvalidContext)?,
        suffix,
    })
}

fn validate_projection(
    snapshot: &DispatchSnapshot<'_>,
    rider: RiderId,
    p: &ExecutionProjection,
) -> Result<(), PoolingError> {
    let origin = snapshot
        .anchors
        .riders
        .get(&rider)
        .ok_or(DispatchEvaluationError::MissingAnchor)?
        .clone();
    let expected = project_execution(
        snapshot.world.data(),
        rider,
        snapshot.at,
        origin,
        p.frozen.clone(),
    )?;
    if expected != *p {
        return Err(PoolingError::InvalidContext);
    }
    p.anchor
        .validate(snapshot.provider.graph(), p.anchor.coordinate)?;
    Ok(())
}

fn protections(
    data: &WorldData,
    inputs: &PoolingInputs,
    completions: &BTreeMap<OrderId, DispatchInstant>,
) -> Result<Vec<InsertionRejection>, PoolingError> {
    let mut breaches = Vec::new();
    for (o, completion) in completions {
        let current = policy(inputs, *o)?;
        let accepted = data.accepted.get(o);
        if current.deadline == AdmissionDeadline::Hard && data.orders[o].deadline.is_none() {
            return Err(PoolingError::UnsupportedPolicy);
        }
        if data.assignments.contains_key(o)
            && current.max_completion_delay.is_some()
            && accepted.is_none()
        {
            return Err(PoolingError::UnsupportedMigration);
        }
        let mut hard_deadlines = Vec::new();
        if current.deadline == AdmissionDeadline::Hard {
            hard_deadlines.push(data.orders[o].deadline);
        }
        if let Some(a) = accepted {
            a.policy.validate()?;
            if a.policy.deadline == AdmissionDeadline::Hard {
                hard_deadlines.push(a.deadline);
            }
            if hard_deadlines.iter().any(Option::is_none) {
                return Err(PoolingError::UnsupportedPolicy);
            }
            let consumed = (completion.value() - a.completion_reference.value()).max(0.0);
            if [a.policy.max_completion_delay, current.max_completion_delay]
                .into_iter()
                .flatten()
                .any(|limit| consumed > limit.value())
            {
                breaches.push(InsertionRejection::CumulativeDelay(*o));
            }
        }
        if hard_deadlines
            .into_iter()
            .flatten()
            .any(|d| *completion > d)
        {
            breaches.push(InsertionRejection::HardDeadline(*o));
        }
    }
    Ok(breaches)
}

fn stop_anchor<'a>(
    snapshot: &'a DispatchSnapshot<'_>,
    stop: Stop,
) -> Result<&'a RoutingAnchor, PoolingError> {
    match stop {
        Stop::Pickup(o) => snapshot.anchors.pickups.get(&o),
        Stop::Dropoff(o) => snapshot.anchors.dropoffs.get(&o),
    }
    .ok_or_else(|| DispatchEvaluationError::MissingAnchor.into())
}

fn validate_frozen(
    snapshot: &DispatchSnapshot<'_>,
    inputs: &PoolingInputs,
    p: &ExecutionProjection,
) -> Result<(), PoolingError> {
    if let Some(f) = &p.frozen {
        let order_policy = policy(inputs, f.timeline.stop.order())?;
        let service = match f.timeline.stop {
            Stop::Pickup(_) => order_policy.pickup_service,
            Stop::Dropoff(_) => order_policy.dropoff_service,
        };
        if f.timeline.service != service {
            return Err(PoolingError::InvalidContext);
        }
        if let Stop::Pickup(o) = f.timeline.stop {
            let (ready, _) = effective_readiness(snapshot.world.data(), inputs, o, snapshot.at)?;
            if f.timeline.arrival.checked_add(f.timeline.waiting)? < ready {
                return Err(PoolingError::InvalidContext);
            }
        }
    }
    Ok(())
}

fn validate_plan_inputs(
    snapshot: &DispatchSnapshot<'_>,
    inputs: &PoolingInputs,
    plan: &RiderPlan,
) -> Result<(), PoolingError> {
    let data = snapshot.world.data();
    for stop in &plan.stops {
        let o = stop.order();
        let order = data
            .orders
            .get(&o)
            .ok_or(DispatchEvaluationError::InvalidWorldState)?;
        let p = policy(inputs, o)?;
        if p.deadline == AdmissionDeadline::Hard && order.deadline.is_none() {
            return Err(PoolingError::UnsupportedPolicy);
        }
        if data.assignments.contains_key(&o)
            && p.max_completion_delay.is_some()
            && !data.accepted.contains_key(&o)
        {
            return Err(PoolingError::UnsupportedMigration);
        }
        if let Stop::Pickup(o) = stop {
            effective_readiness(data, inputs, *o, snapshot.at)?;
        }
    }
    Ok(())
}

fn provisional_plan(
    snapshot: &DispatchSnapshot<'_>,
    rider: RiderId,
    plan: &RiderPlan,
    new_orders: &[OrderId],
) -> Result<Result<WorldData, InsertionRejection>, PoolingError> {
    for order in new_orders {
        if snapshot.world.data().assignments.contains_key(order)
            || snapshot.world.data().fulfillment.get(order)
                != Some(&FulfillmentState::AwaitingPickup)
        {
            return Err(DispatchEvaluationError::InvalidRequest.into());
        }
    }
    let mut provisional = snapshot.world.data().clone();
    for o in new_orders {
        provisional
            .assignments
            .insert(*o, CommittedAssignment { order: *o, rider });
    }
    match validate_plan(&provisional, rider, plan) {
        Ok(()) => {}
        Err(PlanValidityError::CapacityExceeded) => return Ok(Err(InsertionRejection::Capacity)),
        Err(_) => return Err(DispatchEvaluationError::InvalidWorldState.into()),
    }
    Ok(Ok(provisional))
}

/// Authoritative whole-plan evaluation over an explicit projected editable horizon.
/// Candidate assignment is provisional; this function never publishes terms or work.
///
/// # Errors
/// Invalid projections/structure, missing predictions, unsupported policies and routing
/// failures are typed errors. Physical infeasibility is the inner rejection result.
pub fn evaluate_whole_plan(
    snapshot: &DispatchSnapshot<'_>,
    inputs: &PoolingInputs,
    rider: RiderId,
    plan: &RiderPlan,
    new_order: Option<OrderId>,
) -> Result<Result<WholePlanEvaluation, InsertionRejection>, PoolingError> {
    evaluate_batch_plan(
        snapshot,
        inputs,
        rider,
        plan,
        &new_order.into_iter().collect::<Vec<_>>(),
    )
}

/// Shared physical evaluator for a plan containing several provisional new assignments.
/// Existing owners and accepted references remain authoritative in the source snapshot.
///
/// # Errors
/// Same structural/input errors as `evaluate_whole_plan`; infeasibility is the inner result.
pub fn evaluate_batch_plan(
    snapshot: &DispatchSnapshot<'_>,
    inputs: &PoolingInputs,
    rider: RiderId,
    plan: &RiderPlan,
    new_orders: &[OrderId],
) -> Result<Result<WholePlanEvaluation, InsertionRejection>, PoolingError> {
    let p = inputs
        .projections
        .get(&rider)
        .ok_or(PoolingError::InvalidContext)?;
    validate_projection(snapshot, rider, p)?;
    validate_plan_inputs(snapshot, inputs, plan)?;
    validate_frozen(snapshot, inputs, p)?;
    let provisional = match provisional_plan(snapshot, rider, plan, new_orders)? {
        Ok(data) => data,
        Err(reason) => return Ok(Err(reason)),
    };
    let mut editable = plan.stops.as_slice();
    let mut completions = BTreeMap::new();
    if let Some(f) = &p.frozen {
        if editable.first() != Some(&f.timeline.stop) {
            return Err(PoolingError::InvalidContext);
        }
        editable = &editable[1..];
        if let Stop::Dropoff(o) = f.timeline.stop {
            completions.insert(o, f.timeline.departure);
        }
    }
    let mut at = p.at;
    let mut anchor = &p.anchor;
    let mut travel = Seconds::ZERO;
    let mut distance = Meters::ZERO;
    let mut load = p.load.value();
    let mut stops = Vec::new();
    for stop in editable {
        let o = stop.order();
        let policy = policy(inputs, o)?;
        let target = stop_anchor(snapshot, *stop)?;
        let Some(leg) = crate::evaluation::checked_leg(snapshot, anchor, target, at)? else {
            return Ok(Err(InsertionRejection::NoRoute));
        };
        let road = leg.route.elapsed_travel_time();
        let meters = leg.route.total_distance();
        travel = travel
            .checked_add(road)
            .map_err(|_| DispatchEvaluationError::InvalidMetric)?;
        distance = distance
            .checked_add(meters)
            .map_err(|_| DispatchEvaluationError::InvalidMetric)?;
        let arrival = at.checked_add(road)?;
        let (waiting, service, source) = match stop {
            Stop::Pickup(_) => {
                let (ready, source) = effective_readiness(&provisional, inputs, o, snapshot.at)?;
                (
                    if ready > arrival {
                        ready.duration_since(arrival)?
                    } else {
                        Seconds::ZERO
                    },
                    policy.pickup_service,
                    Some(source),
                )
            }
            Stop::Dropoff(_) => (Seconds::ZERO, policy.dropoff_service, None),
        };
        let timeline = StopTimeline::new(*stop, arrival, waiting, service)?;
        let demand = provisional.orders[&o].demand.value();
        load = match stop {
            Stop::Pickup(_) => load.checked_add(demand),
            Stop::Dropoff(_) => load.checked_sub(demand),
        }
        .ok_or(DispatchEvaluationError::InvalidWorldState)?;
        if load > provisional.profiles[&rider].max_capacity.value() {
            return Ok(Err(InsertionRejection::Capacity));
        }
        if matches!(stop, Stop::Dropoff(_)) {
            completions.insert(o, timeline.departure);
        }
        stops.push(EvaluatedStop {
            timeline,
            travel: road,
            distance: meters,
            load: CapacityUnits::new(load),
            readiness_source: source,
        });
        at = timeline.departure;
        anchor = target;
    }
    let breaches = protections(snapshot.world.data(), inputs, &completions)?;
    Ok(Ok(WholePlanEvaluation {
        plan: plan.clone(),
        travel,
        distance,
        stops,
        completions,
        breaches,
    }))
}

pub(crate) fn impact(
    data: &WorldData,
    baseline: &WholePlanEvaluation,
    candidate: &WholePlanEvaluation,
    inputs: &PoolingInputs,
) -> Vec<CompletionImpact> {
    candidate
        .completions
        .iter()
        .map(|(o, c)| {
            let accepted = data.accepted.get(o).map(|a| a.completion_reference);
            let current = baseline.completions.get(o).copied();
            let cumulative = accepted.map(|a| c.value() - a.value());
            CompletionImpact {
                order: *o,
                accepted,
                current,
                candidate: *c,
                marginal_seconds: current.map(|t| c.value() - t.value()),
                cumulative_seconds: cumulative,
                consumed_seconds: cumulative.map(|d| d.max(0.0)),
                deadline_slack_seconds: data.orders[o].deadline.map(|d| d.value() - c.value()),
                accepted_deadline_slack_seconds: data
                    .accepted
                    .get(o)
                    .and_then(|a| a.deadline)
                    .map(|d| d.value() - c.value()),
                cumulative_allowance_remaining_seconds: data
                    .accepted
                    .get(o)
                    .and_then(|a| {
                        [
                            a.policy.max_completion_delay,
                            inputs.policies[o].max_completion_delay,
                        ]
                        .into_iter()
                        .flatten()
                        .map(Seconds::value)
                        .min_by(f64::total_cmp)
                    })
                    .zip(cumulative)
                    .map(|(limit, delay)| limit - delay.max(0.0)),
            }
        })
        .collect()
}

fn better(a: &InsertionProposal, b: &InsertionProposal) -> bool {
    a.incremental_travel
        .total_cmp(&b.incremental_travel)
        .then_with(|| a.incremental_distance.total_cmp(&b.incremental_distance))
        .then_with(|| a.rider.cmp(&b.rider))
        .then_with(|| a.evaluation.plan.stops.cmp(&b.evaluation.plan.stops))
        .is_lt()
}

type RiderCoverage = (Vec<RiderSearchEvidence>, Vec<(RiderId, String)>);

fn baseline_coverage(
    snapshot: &DispatchSnapshot<'_>,
    inputs: &PoolingInputs,
) -> Result<RiderCoverage, PoolingError> {
    let data = snapshot.world.data();
    let mut riders = Vec::new();
    let mut exclusions = Vec::new();
    // Complete baseline pass establishes input/health coverage even with budget zero.
    for rider in data.profiles.keys().copied() {
        if data.riders[&rider].availability != Availability::Available {
            exclusions.push((rider, "operationally-unavailable/v1".into()));
            continue;
        }
        if data.profiles[&rider].routing_profile != snapshot.provenance.profile {
            exclusions.push((rider, "unsupported-routing-profile/v1".into()));
            continue;
        }
        let projection = inputs
            .projections
            .get(&rider)
            .ok_or(PoolingError::InvalidContext)?;
        let baseline = evaluate_whole_plan(snapshot, inputs, rider, &data.plans[&rider], None)?
            .map_err(|_| PoolingError::BaselineUnavailable(rider))?;
        let n = projection.suffix.stops.len();
        let n = u64::try_from(n).map_err(|_| DispatchEvaluationError::InvalidMetric)?;
        let pairs = n
            .checked_add(1)
            .and_then(|a| n.checked_add(2).and_then(|b| a.checked_mul(b)))
            .ok_or(DispatchEvaluationError::InvalidMetric)?
            / 2;
        riders.push(RiderSearchEvidence {
            rider,
            suffix_length: projection.suffix.stops.len(),
            pairs_total: pairs,
            evaluated: 0,
            pruned: if baseline.breaches.is_empty() {
                0
            } else {
                pairs
            },
            feasible: 0,
            baseline,
            rejections: Vec::new(),
        });
    }
    Ok((riders, exclusions))
}

fn search_placements(
    snapshot: &DispatchSnapshot<'_>,
    inputs: &PoolingInputs,
    order: OrderId,
    new_policy: &OrderPolicy,
    riders: &mut [RiderSearchEvidence],
) -> Result<(Option<InsertionProposal>, u64, bool), PoolingError> {
    let data = snapshot.world.data();
    let mut best: Option<InsertionProposal> = None;
    let mut work = 0;
    let mut complete = true;
    'fleet: for r in riders {
        if r.pruned > 0 {
            continue;
        }
        let projection = &inputs.projections[&r.rider];
        let n = projection.suffix.stops.len();
        for pickup in 0..=n {
            for dropoff in pickup + 1..=n + 1 {
                if work == inputs.work_budget {
                    complete = false;
                    break 'fleet;
                }
                let mut stops = projection.suffix.stops.clone();
                stops.insert(pickup, Stop::Pickup(order));
                stops.insert(dropoff, Stop::Dropoff(order));
                if let Some(f) = &projection.frozen {
                    stops.insert(0, f.timeline.stop);
                }
                let plan = RiderPlan { stops };
                work += 1;
                r.evaluated += 1;
                let evaluation =
                    evaluate_whole_plan(snapshot, inputs, r.rider, &plan, Some(order))?;
                let evaluation = match evaluation {
                    Ok(e) if e.breaches.is_empty() => e,
                    other => {
                        let reasons = match other {
                            Ok(e) => e.breaches,
                            Err(reason) => vec![reason],
                        };
                        for reason in reasons {
                            if let Some((_, count)) = r
                                .rejections
                                .iter_mut()
                                .find(|(existing, _)| *existing == reason)
                            {
                                *count += 1;
                            } else {
                                r.rejections.push((reason, 1));
                            }
                        }
                        continue;
                    }
                };
                r.feasible += 1;
                let accepted = AcceptedTerms {
                    policy: new_policy.clone(),
                    accepted_at: snapshot.at,
                    completion_reference: evaluation.completions[&order],
                    deadline: data.orders[&order].deadline,
                    readiness: AcceptedReadiness {
                        expected_at: data.readiness[&order].expected_at,
                        observed_at: data.readiness[&order].observed_at,
                        source: effective_readiness(data, inputs, order, snapshot.at)?.1,
                        forecast: inputs.forecasts.get(&order).cloned(),
                    },
                    provenance: inputs.identity.clone(),
                };
                let proposal = InsertionProposal {
                    rider: r.rider,
                    expected_plan: data.plans[&r.rider].clone(),
                    expected_assignment: None,
                    proposed_assignment: CommittedAssignment {
                        order,
                        rider: r.rider,
                    },
                    existing_assignments: data
                        .assignments
                        .iter()
                        .filter(|(_, a)| a.rider == r.rider)
                        .map(|(o, a)| (*o, *a))
                        .collect(),
                    incremental_travel: evaluation.travel.value() - r.baseline.travel.value(),
                    incremental_distance: evaluation.distance.value() - r.baseline.distance.value(),
                    impacts: impact(data, &r.baseline, &evaluation, inputs),
                    evaluation,
                    accepted,
                };
                if best.as_ref().is_none_or(|b| better(&proposal, b)) {
                    best = Some(proposal);
                }
            }
        }
    }
    Ok((best, work, complete))
}

/// Exhaustively inserts one new order into every eligible rider's editable suffix.
///
/// # Errors
/// Any required unavailable eligible-rider input fails the whole decision. No partial
/// input publication, spatial shortlisting or host-clock interruption is supported.
pub fn insert_order(
    snapshot: &DispatchSnapshot<'_>,
    order: OrderId,
    inputs: PoolingInputs,
) -> Result<InsertionDecision, PoolingError> {
    let data = snapshot.world.data();
    validate_world(data)?;
    if !data.orders.contains_key(&order)
        || data.assignments.contains_key(&order)
        || data.fulfillment[&order] != FulfillmentState::AwaitingPickup
    {
        return Err(DispatchEvaluationError::InvalidRequest.into());
    }
    let context = PoolingContext::new(snapshot, inputs)?;
    let inputs = &context.inputs;
    let new_policy = policy(inputs, order)?.clone();
    if new_policy.readiness == ReadinessRule::LegacyV1 {
        return Err(PoolingError::UnsupportedPolicy);
    }
    if new_policy.deadline == AdmissionDeadline::Hard && data.orders[&order].deadline.is_none() {
        return Err(PoolingError::UnsupportedPolicy);
    }
    effective_readiness(data, inputs, order, snapshot.at)?;
    let (mut riders, exclusions) = baseline_coverage(snapshot, inputs)?;
    let (mut best, work, complete) =
        search_placements(snapshot, inputs, order, &new_policy, &mut riders)?;
    let termination = if !complete {
        best = None;
        InsertionTermination::SearchIncomplete
    } else if best.is_some() {
        InsertionTermination::BestInsertion
    } else {
        InsertionTermination::NoFeasibleInsertion
    };
    Ok(InsertionDecision {
        evidence: InsertionEvidence {
            schema_version: 2,
            world_identity: snapshot.world.identity(),
            world_version: snapshot.world.version(),
            order,
            context,
            input_complete: true,
            riders_complete: true,
            search_complete: complete,
            exclusions,
            riders,
            work,
            termination,
            accounting:
                "projected-post-frozen-prefix; frozen road contribution excluded identically/v1"
                    .into(),
        },
        proposal: best,
    })
}

impl World {
    /// Atomically publishes the exact complete-search assignment, plan and accepted terms.
    /// The caller supplies current coherent prediction/execution/clock context in full.
    ///
    /// # Errors
    /// Any relevant mismatch is stale; incomplete/unassigned decisions cannot commit.
    /// No rejected proposal changes any authoritative record or version.
    pub fn commit_insertion(
        &mut self,
        decision: &InsertionDecision,
        current: &PoolingContext,
    ) -> Result<(), CommitError> {
        let e = &decision.evidence;
        if e.world_identity != self.identity()
            || e.world_version != self.version()
            || e.context != *current
        {
            return Err(CommitError::Stale);
        }
        let p = decision
            .proposal
            .as_ref()
            .ok_or(CommitError::InvalidTransition)?;
        if e.termination != InsertionTermination::BestInsertion
            || !e.search_complete
            || !e.input_complete
            || !e.riders_complete
            || p.expected_assignment.is_some()
            || p.proposed_assignment
                != (CommittedAssignment {
                    order: e.order,
                    rider: p.rider,
                })
            || self.data().assignments.contains_key(&e.order)
            || self.data().accepted.contains_key(&e.order)
            || self.data().plans.get(&p.rider) != Some(&p.expected_plan)
            || self.data().fulfillment.get(&e.order) != Some(&FulfillmentState::AwaitingPickup)
        {
            return Err(CommitError::InvalidTransition);
        }
        let mut next = self.data().clone();
        next.assignments.insert(e.order, p.proposed_assignment);
        next.plans.insert(p.rider, p.evaluation.plan.clone());
        next.accepted.insert(e.order, p.accepted.clone());
        self.publish(next)
    }
}
