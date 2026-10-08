//! Phase 18 deterministic fleet construction/local search; committed owners are fixed.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::pooling::{impact, policy};
use crate::{
    AcceptedReadiness, AcceptedTerms, AdmissionDeadline, Availability, CommitError,
    CommittedAssignment, CompletionImpact, DispatchEvaluationError, DispatchInstant,
    DispatchSnapshot, FulfillmentState, InsertionRejection, OrderId, PoolingContext, PoolingError,
    PoolingInputs, ReadinessRule, RiderId, RiderPlan, RoutingAnchor, Stop, WholePlanEvaluation,
    World, WorldVersion, effective_readiness, evaluate_batch_plan, evaluate_whole_plan,
    validate_world,
};

/// Named deterministic algorithm variants for reproducible quality comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FleetAlgorithm {
    /// Canonical order greedy insertion only; comparison baseline.
    Greedy,
    /// Canonical greedy followed by best-improvement neighborhoods.
    LocalSearch,
    /// All cyclic canonical order starts, then best-improvement neighborhoods.
    MultiStartLocal,
}

/// Explicit execution identity retained when required prefix timing is unavailable.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnavailableExecution {
    /// Immutable active action identity, independent of plan revision.
    pub execution_id: u64,
    /// Required active pickup whose prediction is unavailable.
    pub order: OrderId,
    /// Pinned road arrival; no usable departure prediction is fabricated.
    pub arrival: DispatchInstant,
    /// Pinned destination routing anchor.
    pub anchor: RoutingAnchor,
}

/// Caller-owned inputs; unavailable active projections are explicitly identified.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetInputs {
    /// Shared prediction, service, routing, policies, projections and work budget.
    pub pooling: PoolingInputs,
    /// Active prefix cannot be predicted because this order's readiness is unavailable.
    pub unavailable_projections: BTreeMap<RiderId, UnavailableExecution>,
    /// Exact declared algorithm, pinned in publication context.
    pub algorithm: FleetAlgorithm,
}

/// Exact full fleet publication context; never inferred from host/simulator state.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetContext {
    /// Coherent clock, epoch, policies, forecasts, execution and routing anchors.
    pub pooling: PoolingContext,
    /// Explicit unavailable active projections.
    pub unavailable_projections: BTreeMap<RiderId, UnavailableExecution>,
    /// Algorithm identity/configuration.
    pub algorithm: FleetAlgorithm,
}
impl FleetContext {
    /// Bind current authoritative inputs to a coherent snapshot.
    ///
    /// # Errors
    /// Rejects unpinned contexts and contradictory projection/isolation inputs.
    pub fn new(snapshot: &DispatchSnapshot<'_>, inputs: FleetInputs) -> Result<Self, PoolingError> {
        for (rider, execution) in &inputs.unavailable_projections {
            let order = execution.order;
            if inputs.pooling.projections.contains_key(rider)
                || snapshot
                    .world
                    .data()
                    .plans
                    .get(rider)
                    .and_then(|p| p.stops.first())
                    != Some(&Stop::Pickup(order))
            {
                return Err(PoolingError::InvalidContext);
            }
            let data = snapshot.world.data();
            execution
                .anchor
                .validate(snapshot.provider.graph(), data.orders[&order].pickup)?;
            let unresolved_past_forecast = data.readiness[&order].observed_at.is_none()
                && data.readiness[&order]
                    .expected_at
                    .is_some_and(|at| at < snapshot.at);
            if effective_readiness(data, &inputs.pooling, order, snapshot.at)
                != Err(PoolingError::PredictionUnavailable(order))
                && !unresolved_past_forecast
            {
                return Err(PoolingError::InvalidContext);
            }
        }
        Ok(Self {
            pooling: PoolingContext::bind(snapshot, inputs.pooling, "fleet-greedy-local/v1")?,
            unavailable_projections: inputs.unavailable_projections,
            algorithm: inputs.algorithm,
        })
    }
}

/// Why a rider is preserved unchanged, independently of search quality.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum FleetIsolation {
    /// Operationally unavailable.
    Unavailable,
    /// The pinned provider does not support this routing profile.
    UnsupportedProfile,
    /// Required readiness is unknown; not healthy or infeasible.
    PredictionUnavailable(OrderId),
    /// Existing obligation cannot be routed under the pinned model.
    BaselineUnavailable,
    /// Existing accepted/current hard protection is already breached.
    BaselineBreach(Vec<InsertionRejection>),
}

/// Rider health plus reproducible baseline and selected whole-plan timing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetRiderEvidence {
    /// Canonical rider identity.
    pub rider: RiderId,
    /// Editable stop population.
    pub suffix_length: usize,
    /// Unchanged-isolation reason, absent for an optimized rider.
    pub isolation: Option<FleetIsolation>,
    /// Current predicted timing, absent only when inputs cannot be evaluated.
    pub baseline: Option<WholePlanEvaluation>,
    /// Chosen timing; isolated evaluable riders retain the baseline.
    pub selected: Option<WholePlanEvaluation>,
    /// Signed accepted/current/candidate customer effects.
    pub impacts: Vec<CompletionImpact>,
}

/// Lexicographic fleet objective; demand is feasibility only.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetObjective {
    /// Newly admitted batch orders, distinct from executed delivered orders.
    pub admitted_orders: usize,
    /// Evaluable nonisolated remaining road seconds, frozen prefix excluded identically.
    pub road_seconds: f64,
    /// Same horizon road distance.
    pub road_meters: f64,
}

/// Declared algorithm termination; global VRP optimality is never asserted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FleetTermination {
    /// All declared constructions completed (greedy comparison algorithm).
    ConstructionComplete,
    /// Complete declared best-improvement neighborhood has no improving candidate.
    LocalOptimum,
    /// Budget expired; even an incumbent cannot publish.
    SearchIncomplete,
}

/// Deterministic candidate work separated by algorithm step.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct FleetWork {
    /// Greedy construction complete fleet submissions.
    pub construction: u64,
    /// Local improvement complete fleet submissions.
    pub neighborhood: u64,
    /// Feasible submissions, including duplicates in distinct declared starts.
    pub feasible: u64,
    /// Number of completed best-improvement rounds.
    pub completed_rounds: u64,
    /// Completed construction starts.
    pub completed_starts: usize,
    /// Stop relocations rejected before evaluation by pickup-before-dropoff proof.
    pub precedence_exclusions: u64,
    /// Counted infeasible submissions by typed reason (one submission may have many breaches).
    pub rejections: Vec<(InsertionRejection, u64)>,
}
impl FleetWork {
    /// Total complete fleet submissions, each including early infeasibility.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.construction + self.neighborhood
    }
}

/// Versioned fleet semantic artifact without machine-specific measurements.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetEvidence {
    /// Fleet evidence contract version.
    pub schema_version: u32,
    /// Exact source world identity.
    pub world_identity: u64,
    /// Exact source world version.
    pub world_version: WorldVersion,
    /// Complete publication context.
    pub context: FleetContext,
    /// Canonical input batch, independent of external collection construction.
    pub batch: Vec<OrderId>,
    /// Required unavailable forecasts for new work, isolated without fabricated feasibility.
    pub isolated_orders: Vec<OrderId>,
    /// Full original INPUT coverage; false when any required prediction/baseline is unavailable.
    pub input_complete: bool,
    /// Every rider classified; independent of input sufficiency.
    pub riders_complete: bool,
    /// Declared search completed over the explicit evaluable remainder.
    pub search_complete: bool,
    /// Original plan state, including isolated unchanged plans.
    pub expected_plans: BTreeMap<RiderId, RiderPlan>,
    /// Original committed ownership, never replaced by this algorithm.
    pub expected_assignments: BTreeMap<OrderId, CommittedAssignment>,
    /// Immutable original acceptance records; never reset by a plan rewrite.
    pub expected_terms: BTreeMap<OrderId, AcceptedTerms>,
    /// Per-rider health, timing and customer effects.
    pub riders: Vec<FleetRiderEvidence>,
    /// Evaluable-remainder objective before admission/resequencing.
    pub baseline_objective: FleetObjective,
    /// Chosen objective, absent on incomplete search.
    pub selected_objective: Option<FleetObjective>,
    /// Deterministic algorithm work counts.
    pub work: FleetWork,
    /// Exact scope of termination/quality claim.
    pub termination: FleetTermination,
    /// Objective boundary, isolation and optimality qualification.
    pub accounting: String,
}

/// Exact immutable fleet replacement, available only after complete declared search.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetProposal {
    plans: BTreeMap<RiderId, RiderPlan>,
    assignments: BTreeMap<OrderId, CommittedAssignment>,
    accepted: BTreeMap<OrderId, AcceptedTerms>,
    evaluations: BTreeMap<RiderId, WholePlanEvaluation>,
    objective: FleetObjective,
}
impl FleetProposal {
    /// Exact replacement plans for healthy evaluable riders.
    #[must_use]
    pub const fn plans(&self) -> &BTreeMap<RiderId, RiderPlan> {
        &self.plans
    }
    /// Newly admitted assignments only; existing ownership is unchanged.
    #[must_use]
    pub const fn assignments(&self) -> &BTreeMap<OrderId, CommittedAssignment> {
        &self.assignments
    }
    /// Acceptance terms established only if publication succeeds.
    #[must_use]
    pub const fn accepted(&self) -> &BTreeMap<OrderId, AcceptedTerms> {
        &self.accepted
    }
    /// Exact submitted physical evaluations.
    #[must_use]
    pub const fn evaluations(&self) -> &BTreeMap<RiderId, WholePlanEvaluation> {
        &self.evaluations
    }
    /// Chosen lexicographic objective.
    #[must_use]
    pub const fn objective(&self) -> &FleetObjective {
        &self.objective
    }
}

/// Read-only fleet search result; isolated input may coexist with a complete remainder.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetDecision {
    evidence: FleetEvidence,
    proposal: Option<FleetProposal>,
}
impl FleetDecision {
    /// Full coverage, health, work and context.
    #[must_use]
    pub const fn evidence(&self) -> &FleetEvidence {
        &self.evidence
    }
    /// Publishable exact replacement, absent if incomplete or semantically unchanged.
    #[must_use]
    pub const fn proposal(&self) -> Option<&FleetProposal> {
        self.proposal.as_ref()
    }
}

type Plans = BTreeMap<RiderId, RiderPlan>;
#[derive(Clone)]
struct Candidate {
    plans: Plans,
    assignments: BTreeMap<OrderId, CommittedAssignment>,
    evaluations: BTreeMap<RiderId, WholePlanEvaluation>,
    objective: FleetObjective,
}
fn better(a: &Candidate, b: &Candidate) -> bool {
    b.objective
        .admitted_orders
        .cmp(&a.objective.admitted_orders)
        .then_with(|| {
            a.objective
                .road_seconds
                .total_cmp(&b.objective.road_seconds)
        })
        .then_with(|| a.objective.road_meters.total_cmp(&b.objective.road_meters))
        .then_with(|| {
            a.plans
                .iter()
                .map(|(r, p)| (r, &p.stops))
                .cmp(b.plans.iter().map(|(r, p)| (r, &p.stops)))
        })
        .is_lt()
}
fn objective(
    evaluations: &BTreeMap<RiderId, WholePlanEvaluation>,
    admitted: usize,
) -> Result<FleetObjective, PoolingError> {
    let seconds: f64 = evaluations.values().map(|e| e.travel.value()).sum();
    let meters: f64 = evaluations.values().map(|e| e.distance.value()).sum();
    if !seconds.is_finite() || !meters.is_finite() {
        return Err(DispatchEvaluationError::InvalidMetric.into());
    }
    Ok(FleetObjective {
        admitted_orders: admitted,
        road_seconds: seconds,
        road_meters: meters,
    })
}
fn classify(
    snapshot: &DispatchSnapshot<'_>,
    context: &FleetContext,
) -> Result<(Vec<FleetRiderEvidence>, Candidate, bool), PoolingError> {
    let mut riders = Vec::new();
    let mut plans = BTreeMap::new();
    let mut evaluations = BTreeMap::new();
    let mut complete = true;
    let data = snapshot.world.data();
    for rider in data.profiles.keys().copied() {
        let mut baseline = None;
        let isolation = if data.riders[&rider].availability != Availability::Available {
            Some(FleetIsolation::Unavailable)
        } else if data.profiles[&rider].routing_profile != snapshot.provenance.profile {
            Some(FleetIsolation::UnsupportedProfile)
        } else if let Some(order) = context.unavailable_projections.get(&rider) {
            complete = false;
            Some(FleetIsolation::PredictionUnavailable(order.order))
        } else {
            match evaluate_whole_plan(
                snapshot,
                &context.pooling.inputs,
                rider,
                &data.plans[&rider],
                None,
            ) {
                Err(PoolingError::PredictionUnavailable(o)) => {
                    complete = false;
                    Some(FleetIsolation::PredictionUnavailable(o))
                }
                Ok(Err(_)) => {
                    complete = false;
                    Some(FleetIsolation::BaselineUnavailable)
                }
                Err(e) => return Err(e),
                Ok(Ok(e)) => {
                    let isolation = if e.breaches.is_empty() {
                        None
                    } else {
                        Some(FleetIsolation::BaselineBreach(e.breaches.clone()))
                    };
                    if isolation.is_none() {
                        plans.insert(rider, e.plan.clone());
                        evaluations.insert(rider, e.clone());
                    }
                    baseline = Some(e);
                    isolation
                }
            }
        };
        let suffix_length = context
            .pooling
            .inputs
            .projections
            .get(&rider)
            .map_or(data.plans[&rider].stops.len(), |p| p.suffix.stops.len());
        let impacts = baseline
            .as_ref()
            .map_or_else(Vec::new, |e| impact(data, e, e, &context.pooling.inputs));
        riders.push(FleetRiderEvidence {
            rider,
            suffix_length,
            isolation,
            selected: baseline.clone(),
            baseline,
            impacts,
        });
    }
    let objective = objective(&evaluations, 0)?;
    Ok((
        riders,
        Candidate {
            plans,
            assignments: BTreeMap::new(),
            evaluations,
            objective,
        },
        complete,
    ))
}

struct Search<'a, 'b> {
    snapshot: &'a DispatchSnapshot<'b>,
    context: &'a FleetContext,
    batch: BTreeSet<OrderId>,
    work: FleetWork,
    exhausted: bool,
}
impl Search<'_, '_> {
    fn submit(
        &mut self,
        plans: Plans,
        neighborhood: bool,
    ) -> Result<Option<Candidate>, PoolingError> {
        if self.work.total() >= self.context.pooling.inputs.work_budget {
            self.exhausted = true;
            return Ok(None);
        }
        if neighborhood {
            self.work.neighborhood += 1;
        } else {
            self.work.construction += 1;
        }
        let data = self.snapshot.world.data();
        let mut assignments = BTreeMap::new();
        for (r, p) in &plans {
            for stop in &p.stops {
                if let Stop::Dropoff(o) = stop {
                    if self.batch.contains(o)
                        && assignments
                            .insert(
                                *o,
                                CommittedAssignment {
                                    order: *o,
                                    rider: *r,
                                },
                            )
                            .is_some()
                    {
                        return Err(DispatchEvaluationError::InvalidWorldState.into());
                    }
                }
            }
        }
        let mut evaluations = BTreeMap::new();
        for (r, p) in &plans {
            let new: Vec<_> = assignments
                .values()
                .filter(|a| a.rider == *r)
                .map(|a| a.order)
                .collect();
            let evaluation =
                evaluate_batch_plan(self.snapshot, &self.context.pooling.inputs, *r, p, &new)?;
            let reasons = match evaluation {
                Ok(e) if e.breaches.is_empty() => {
                    evaluations.insert(*r, e);
                    continue;
                }
                Ok(e) => e.breaches,
                Err(reason) => vec![reason],
            };
            for reason in reasons {
                if let Some((_, count)) =
                    self.work.rejections.iter_mut().find(|(r, _)| *r == reason)
                {
                    *count += 1;
                } else {
                    self.work.rejections.push((reason, 1));
                }
            }
            return Ok(None);
        }
        // Provisional assignment derivation must never include an existing commitment.
        if assignments.keys().any(|o| data.assignments.contains_key(o)) {
            return Err(DispatchEvaluationError::InvalidRequest.into());
        }
        self.work.feasible += 1;
        let objective = objective(&evaluations, assignments.len())?;
        Ok(Some(Candidate {
            plans,
            assignments,
            evaluations,
            objective,
        }))
    }
    fn frozen_len(&self, r: RiderId) -> usize {
        usize::from(self.context.pooling.inputs.projections[&r].frozen.is_some())
    }
    fn insertions(
        &mut self,
        base: &Plans,
        order: OrderId,
        neighborhood: bool,
        best: &mut Option<Candidate>,
    ) -> Result<(), PoolingError> {
        for (r, p) in base {
            let first = self.frozen_len(*r);
            for pickup in first..=p.stops.len() {
                for dropoff in pickup + 1..=p.stops.len() + 1 {
                    let mut plans = base.clone();
                    let stops = &mut plans.get_mut(r).ok_or(PoolingError::InvalidContext)?.stops;
                    stops.insert(pickup, Stop::Pickup(order));
                    stops.insert(dropoff, Stop::Dropoff(order));
                    self.consider(plans, neighborhood, best)?;
                    if self.exhausted {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }
    fn consider(
        &mut self,
        plans: Plans,
        neighborhood: bool,
        best: &mut Option<Candidate>,
    ) -> Result<(), PoolingError> {
        if let Some(c) = self.submit(plans, neighborhood)? {
            if best.as_ref().is_none_or(|b| better(&c, b)) {
                *best = Some(c);
            }
        }
        Ok(())
    }
    fn construct(
        &mut self,
        baseline: &Candidate,
        orders: &[OrderId],
    ) -> Result<Candidate, PoolingError> {
        let mut current = baseline.clone();
        for order in orders {
            let mut best = None;
            self.insertions(&current.plans, *order, false, &mut best)?;
            if self.exhausted {
                return Ok(current);
            }
            if let Some(c) = best {
                current = c;
            }
        }
        self.work.completed_starts += 1;
        Ok(current)
    }
    fn improve(
        &mut self,
        mut current: Candidate,
        orders: &[OrderId],
    ) -> Result<Candidate, PoolingError> {
        loop {
            let mut best = Some(current.clone());
            // Unplaced requests may enter any healthy plan.
            for o in orders
                .iter()
                .filter(|o| !current.assignments.contains_key(o))
            {
                self.insertions(&current.plans, *o, true, &mut best)?;
                if self.exhausted {
                    return Ok(current);
                }
            }
            // Move one editable stop in its owner plan; precedence is a proven structural exclusion.
            for (r, p) in &current.plans {
                let first = self.frozen_len(*r);
                for from in first..p.stops.len() {
                    for to in first..p.stops.len() {
                        if from == to {
                            continue;
                        }
                        let mut plans = current.plans.clone();
                        let stops =
                            &mut plans.get_mut(r).ok_or(PoolingError::InvalidContext)?.stops;
                        let stop = stops.remove(from);
                        stops.insert(to, stop);
                        if !precedence(stops) {
                            self.work.precedence_exclusions += 1;
                            continue;
                        }
                        self.consider(plans, true, &mut best)?;
                        if self.exhausted {
                            return Ok(current);
                        }
                    }
                }
            }
            // Only proposed new ownership may change. Existing committed owners never move.
            for o in current.assignments.keys() {
                let removed = remove_order(&current.plans, *o);
                self.insertions(&removed, *o, true, &mut best)?;
                if self.exhausted {
                    return Ok(current);
                }
                // One-request ejection: admit unplaced work, then reinsert the displaced request.
                for unplaced in orders
                    .iter()
                    .filter(|o| !current.assignments.contains_key(o))
                {
                    self.ejections(&removed, *unplaced, *o, &mut best)?;
                    if self.exhausted {
                        return Ok(current);
                    }
                }
            }
            self.work.completed_rounds += 1;
            let selected = best.ok_or(PoolingError::InvalidContext)?;
            if !better(&selected, &current) {
                return Ok(current);
            }
            current = selected;
        }
    }
    fn ejections(
        &mut self,
        base: &Plans,
        unplaced: OrderId,
        displaced: OrderId,
        best: &mut Option<Candidate>,
    ) -> Result<(), PoolingError> {
        for (r, p) in base {
            for pickup in self.frozen_len(*r)..=p.stops.len() {
                for dropoff in pickup + 1..=p.stops.len() + 1 {
                    let mut plans = base.clone();
                    let stops = &mut plans.get_mut(r).ok_or(PoolingError::InvalidContext)?.stops;
                    stops.insert(pickup, Stop::Pickup(unplaced));
                    stops.insert(dropoff, Stop::Dropoff(unplaced));
                    self.consider(plans.clone(), true, best)?;
                    if self.exhausted {
                        return Ok(());
                    }
                    // Do not require the intermediate plan feasible: final departures/readiness may change it.
                    self.insertions(&plans, displaced, true, best)?;
                    if self.exhausted {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }
}
fn precedence(stops: &[Stop]) -> bool {
    for (i, s) in stops.iter().enumerate() {
        if let Stop::Pickup(o) = s {
            if stops[..i].contains(&Stop::Dropoff(*o)) {
                return false;
            }
        }
    }
    true
}
fn remove_order(plans: &Plans, order: OrderId) -> Plans {
    plans
        .iter()
        .map(|(r, p)| {
            (
                *r,
                RiderPlan {
                    stops: p
                        .stops
                        .iter()
                        .copied()
                        .filter(|s| s.order() != order)
                        .collect(),
                },
            )
        })
        .collect()
}
fn acceptance(
    snapshot: &DispatchSnapshot<'_>,
    context: &FleetContext,
    candidate: &Candidate,
) -> Result<BTreeMap<OrderId, AcceptedTerms>, PoolingError> {
    let data = snapshot.world.data();
    let inputs = &context.pooling.inputs;
    candidate
        .assignments
        .iter()
        .map(|(o, a)| {
            Ok((
                *o,
                AcceptedTerms {
                    policy: policy(inputs, *o)?.clone(),
                    accepted_at: snapshot.at,
                    completion_reference: candidate.evaluations[&a.rider].completions[o],
                    deadline: data.orders[o].deadline,
                    readiness: AcceptedReadiness {
                        expected_at: data.readiness[o].expected_at,
                        observed_at: data.readiness[o].observed_at,
                        source: effective_readiness(data, inputs, *o, snapshot.at)?.1,
                        forecast: inputs.forecasts.get(o).cloned(),
                    },
                    provenance: inputs.identity.clone(),
                },
            ))
        })
        .collect()
}

/// Jointly allocate current new work and resequence healthy editable suffixes.
///
/// # Errors
/// Structural/policy/projection corruption is fatal. Required unavailable predictions
/// are explicitly isolated. Budget exhaustion returns a nonpublishable decision.
#[allow(clippy::too_many_lines)]
pub fn optimize_fleet(
    snapshot: &DispatchSnapshot<'_>,
    batch: &[OrderId],
    inputs: FleetInputs,
) -> Result<FleetDecision, PoolingError> {
    let data = snapshot.world.data();
    validate_world(data)?;
    let context = FleetContext::new(snapshot, inputs)?;
    // Policy errors are not unknown readiness and cannot be hidden by rider isolation.
    for o in data.assignments.keys() {
        let p = policy(&context.pooling.inputs, *o)?;
        if p.deadline == AdmissionDeadline::Hard && data.orders[o].deadline.is_none() {
            return Err(PoolingError::UnsupportedPolicy);
        }
        if p.max_completion_delay.is_some() && !data.accepted.contains_key(o) {
            return Err(PoolingError::UnsupportedMigration);
        }
    }
    let canonical: BTreeSet<_> = batch.iter().copied().collect();
    if canonical.len() != batch.len() {
        return Err(DispatchEvaluationError::InvalidRequest.into());
    }
    let mut orders = Vec::new();
    let mut isolated_orders = Vec::new();
    for o in &canonical {
        if !data.orders.contains_key(o)
            || data.assignments.contains_key(o)
            || data.fulfillment[o] != FulfillmentState::AwaitingPickup
            || data.orders[o].created_at > snapshot.at
        {
            return Err(DispatchEvaluationError::InvalidRequest.into());
        }
        let p = policy(&context.pooling.inputs, *o)?;
        if p.readiness == ReadinessRule::LegacyV1
            || (p.deadline == AdmissionDeadline::Hard && data.orders[o].deadline.is_none())
        {
            return Err(PoolingError::UnsupportedPolicy);
        }
        match effective_readiness(data, &context.pooling.inputs, *o, snapshot.at) {
            Ok(_) => orders.push(*o),
            Err(PoolingError::PredictionUnavailable(_)) => isolated_orders.push(*o),
            Err(e) => return Err(e),
        }
    }
    let (mut riders, baseline, input_complete) = classify(snapshot, &context)?;
    let mut search = Search {
        snapshot,
        context: &context,
        batch: orders.iter().copied().collect(),
        work: FleetWork::default(),
        exhausted: false,
    };
    let mut selected = baseline.clone();
    let starts = if context.algorithm == FleetAlgorithm::MultiStartLocal {
        orders.len().max(1)
    } else {
        1
    };
    for offset in 0..starts {
        let mut sequence = orders.clone();
        if !sequence.is_empty() {
            sequence.rotate_left(offset);
        }
        let candidate = search.construct(&baseline, &sequence)?;
        if search.exhausted {
            break;
        }
        if better(&candidate, &selected) {
            selected = candidate;
        }
    }
    if !search.exhausted && context.algorithm != FleetAlgorithm::Greedy {
        selected = search.improve(selected, &orders)?;
    }
    let complete = !search.exhausted;
    let termination = if !complete {
        FleetTermination::SearchIncomplete
    } else if context.algorithm == FleetAlgorithm::Greedy {
        FleetTermination::ConstructionComplete
    } else {
        FleetTermination::LocalOptimum
    };
    let work = search.work;
    let mut proposal = None;
    if complete {
        for r in &mut riders {
            if let Some(e) = selected.evaluations.get(&r.rider) {
                r.selected = Some(e.clone());
                r.impacts = impact(
                    data,
                    r.baseline.as_ref().ok_or(PoolingError::InvalidContext)?,
                    e,
                    &context.pooling.inputs,
                );
            }
        }
        if !selected.assignments.is_empty()
            || selected.plans.iter().any(|(r, p)| data.plans[r] != *p)
        {
            let accepted = acceptance(snapshot, &context, &selected)?;
            proposal = Some(FleetProposal {
                plans: selected.plans.clone(),
                assignments: selected.assignments.clone(),
                accepted,
                evaluations: selected.evaluations.clone(),
                objective: selected.objective.clone(),
            });
        }
    }
    let accounting = concat!(
        "evaluable nonisolated fleet; projected post-frozen-prefix road horizon; ",
        "isolated constant contributions excluded identically; ",
        "declared heuristic/local optimum, not global VRP optimum/v1"
    )
    .into();
    Ok(FleetDecision {
        evidence: FleetEvidence {
            schema_version: 1,
            world_identity: snapshot.world.identity(),
            world_version: snapshot.world.version(),
            batch: canonical.into_iter().collect(),
            input_complete: input_complete && isolated_orders.is_empty(),
            isolated_orders,
            riders_complete: true,
            search_complete: complete,
            expected_plans: data.plans.clone(),
            expected_assignments: data.assignments.clone(),
            expected_terms: data.accepted.clone(),
            riders,
            baseline_objective: baseline.objective,
            selected_objective: complete.then_some(selected.objective),
            work,
            termination,
            context,
            accounting,
        },
        proposal,
    })
}

impl World {
    /// Publish all new ownership, plans and acceptance terms in one validated transition.
    ///
    /// # Errors
    /// Any world/input/clock/execution mismatch rejects the entire proposal stale.
    /// Incomplete or unchanged decisions cannot commit. Rejection leaves state untouched.
    pub fn commit_fleet(
        &mut self,
        decision: &FleetDecision,
        current: &FleetContext,
    ) -> Result<(), CommitError> {
        let e = &decision.evidence;
        if self.identity() != e.world_identity
            || self.version() != e.world_version
            || *current != e.context
            || self.data().plans != e.expected_plans
            || self.data().assignments != e.expected_assignments
            || self.data().accepted != e.expected_terms
        {
            return Err(CommitError::Stale);
        }
        let p = decision
            .proposal
            .as_ref()
            .ok_or(CommitError::InvalidTransition)?;
        if !e.search_complete || !e.riders_complete {
            return Err(CommitError::InvalidTransition);
        }
        let mut next = self.data().clone();
        for (o, a) in &p.assignments {
            if next.assignments.contains_key(o)
                || next.accepted.contains_key(o)
                || next.fulfillment.get(o) != Some(&FulfillmentState::AwaitingPickup)
            {
                return Err(CommitError::InvalidTransition);
            }
            next.assignments.insert(*o, *a);
            next.accepted.insert(*o, p.accepted[o].clone());
        }
        for (r, plan) in &p.plans {
            next.plans.insert(*r, plan.clone());
        }
        self.publish(next)
    }
}
