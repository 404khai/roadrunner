use roadrunner_core::cost::RouteCost;
use roadrunner_core::geo::{KilometersPerHour, Meters, Seconds};
use roadrunner_core::routing::RoutingError;
use serde::Serialize;
use thiserror::Error;

use crate::spatial::{IndexedRiderLocator, RiderLocation, RiderLookup};
use crate::{
    AssignmentDecision, AssignmentProposal, CandidateEvidence, CandidatePolicy, CandidateRejection,
    CandidateResult, CommittedAssignment, DeadlinePolicy, DecisionEvidence, DecisionId,
    DispatchDecisionOutcome, DispatchInstant, DispatchTimeError, FulfillmentState, OrderId,
    RiderId, RiderPlan, RouteOutcome, RouteProvider, RoutingAnchor, RoutingAnchors, RoutingEpoch,
    RoutingProvenance, ScoreContributions, SelectionReason, Stop, UnassignedScope, World,
    basic_dispatch_eligible, validate_plan, validate_world,
};

/// Failed decision evaluation, distinct from a completed Unassigned decision.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DispatchEvaluationError {
    /// Authoritative assignment, plan, fulfillment, or keyed records contradict.
    #[error("invalid coherent world state")]
    InvalidWorldState,
    /// Basic Dispatch only accepts a created, unassigned, awaiting-pickup request.
    #[error("invalid Basic Dispatch request")]
    InvalidRequest,
    /// No routing projection exists for a required location.
    #[error("missing routing anchor")]
    MissingAnchor,
    /// Anchor coordinate, graph digest, or local node is invalid.
    #[error("invalid routing anchor")]
    InvalidAnchor,
    /// Provider context or returned route provenance changed or mismatched.
    #[error("routing provenance mismatch")]
    ProvenanceMismatch,
    /// Checked time conversion/propagation failed.
    #[error(transparent)]
    Time(#[from] DispatchTimeError),
    /// Road routing failed to evaluate rather than finding no legal path.
    #[error("routing evaluation failed: {0}")]
    Routing(#[source] RoutingError),
    /// Finite metric or score accumulation overflowed.
    #[error("non-finite plan metric or strategy score")]
    InvalidMetric,
    /// Candidate generation failed to build its coherent position projection.
    #[error("invalid candidate projection")]
    CandidateGeneration,
}

/// Coherent immutable evaluation view; no live store, DB handle, or clock.
pub struct DispatchSnapshot<'a> {
    pub(crate) world: &'a World,
    pub(crate) at: DispatchInstant,
    pub(crate) epoch: RoutingEpoch,
    pub(crate) provider: &'a dyn RouteProvider,
    pub(crate) anchors: &'a RoutingAnchors,
    pub(crate) provenance: RoutingProvenance,
}

impl<'a> DispatchSnapshot<'a> {
    /// Binds validated world, explicit now/epoch, routing context, and projections.
    ///
    /// # Errors
    /// Rejects incoherent/future facts, invalid time conversion, or provider identity.
    pub fn new(
        world: &'a World,
        at: DispatchInstant,
        epoch: RoutingEpoch,
        provider: &'a dyn RouteProvider,
        anchors: &'a RoutingAnchors,
    ) -> Result<Self, DispatchEvaluationError> {
        validate_world(world.data())?;
        epoch.departure_seconds(at)?;
        let provenance = provider.provenance();
        if provenance.graph_digest != provider.graph().metadata().snapshot_digest()
            || provenance.profile != provider.graph().metadata().routing_profile()
        {
            return Err(DispatchEvaluationError::ProvenanceMismatch);
        }
        let data = world.data();
        anchors.validate(provider.graph(), data)?;
        for (id, order) in &data.orders {
            let future_state = match data.fulfillment[id] {
                FulfillmentState::AwaitingPickup => false,
                FulfillmentState::PickedUp { at: t, .. }
                | FulfillmentState::Delivered { at: t, .. } => t > at,
            };
            if order.created_at > at
                || future_state
                || data.readiness[id].observed_at.is_some_and(|t| t > at)
            {
                return Err(DispatchEvaluationError::InvalidWorldState);
            }
        }
        Ok(Self {
            world,
            at,
            epoch,
            provider,
            anchors,
            provenance,
        })
    }
    /// Returns the fixed evaluation instant supplied by the caller.
    #[must_use]
    pub const fn evaluated_at(&self) -> DispatchInstant {
        self.at
    }
    /// Returns the coherent immutable world view.
    #[must_use]
    pub const fn world(&self) -> &World {
        self.world
    }
}

/// Explicit arrival -> waiting -> service -> departure boundary.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct StopTimeline {
    /// Logical stop being completed.
    pub stop: Stop,
    /// Instant after incoming road travel.
    pub arrival: DispatchInstant,
    /// Stop wait, separate from road movement.
    pub waiting: Seconds,
    /// Stop service, separate from road movement.
    pub service: Seconds,
    /// Instant after waiting and service; next leg starts here.
    pub departure: DispatchInstant,
}

impl StopTimeline {
    /// Applies explicit stop durations between road legs.
    ///
    /// # Errors
    /// Rejects checked time overflow; no wall-clock reads.
    pub fn new(
        stop: Stop,
        arrival: DispatchInstant,
        waiting: Seconds,
        service: Seconds,
    ) -> Result<Self, DispatchTimeError> {
        let departure = arrival.checked_add(waiting)?.checked_add(service)?;
        Ok(Self {
            stop,
            arrival,
            waiting,
            service,
            departure,
        })
    }
}

/// Typed physical/timing outcomes, independent from strategy preference.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlanEvaluation {
    /// Proposed ordered logical work.
    pub plan: RiderPlan,
    /// Movement from rider location to pickup.
    pub pickup_travel: Seconds,
    /// Movement from pickup departure to dropoff arrival.
    pub delivery_travel: Seconds,
    /// Sum of road movement, excluding stops.
    pub travel: Seconds,
    /// Sum of road distance.
    pub distance: Meters,
    /// Timeline for every logical stop.
    pub stops: Vec<StopTimeline>,
    /// Predicted pickup arrival.
    pub pickup_arrival: DispatchInstant,
    /// Predicted pickup completion / onward departure.
    pub pickup_departure: DispatchInstant,
    /// Predicted customer arrival.
    pub dropoff_arrival: DispatchInstant,
    /// Predicted completed dropoff, the deadline endpoint.
    pub delivery_completed_at: DispatchInstant,
    /// Total stop waiting, zero in Phase 13.
    pub waiting: Seconds,
    /// Total stop service, zero in Phase 13.
    pub service: Seconds,
    /// Soft-observed completed-dropoff lateness.
    pub lateness: Seconds,
}

/// Feasible evaluated plan passed to a pure ranking policy.
#[derive(Debug, Clone, PartialEq)]
pub struct FeasibleCandidate {
    /// Candidate rider identity.
    pub rider: RiderId,
    /// Already-validated and measured plan.
    pub evaluation: PlanEvaluation,
}

/// Deterministic pure ranking output.
#[derive(Debug, Clone, PartialEq)]
pub struct BaselineRanking {
    /// Scores canonically ordered by `RiderId`.
    pub scores: Vec<(RiderId, ScoreContributions)>,
    /// Selected identity, absent only when no feasible plans were provided.
    pub selected: Option<RiderId>,
    /// Structured exact tie reason.
    pub reason: Option<SelectionReason>,
}

/// Phase 13 pure strategy: pickup road travel + delivery road travel only.
#[derive(Debug, Clone, Copy, Default)]
pub struct BaselineStrategy;

impl BaselineStrategy {
    /// Scores already-feasible evaluated plans without routing or world access.
    ///
    /// # Errors
    /// Rejects non-finite accumulation or duplicate candidate identities.
    pub fn rank(
        self,
        candidates: &[FeasibleCandidate],
    ) -> Result<BaselineRanking, DispatchEvaluationError> {
        let mut scores = Vec::with_capacity(candidates.len());
        for c in candidates {
            scores.push((
                c.rider,
                ScoreContributions {
                    pickup_travel: c.evaluation.pickup_travel,
                    delivery_travel: c.evaluation.delivery_travel,
                    total: c
                        .evaluation
                        .pickup_travel
                        .checked_add(c.evaluation.delivery_travel)
                        .map_err(|_| DispatchEvaluationError::InvalidMetric)?,
                },
            ));
        }
        scores.sort_by_key(|(rider, _)| *rider);
        if scores.windows(2).any(|p| p[0].0 == p[1].0) {
            return Err(DispatchEvaluationError::InvalidMetric);
        }
        let mut ranked = scores.clone();
        ranked.sort_by(|(a, sa), (b, sb)| {
            sa.total
                .value()
                .total_cmp(&sb.total.value())
                .then_with(|| a.cmp(b))
        });
        let selected = ranked.first().map(|(rider, _)| *rider);
        let reason = ranked.first().map(|(_, score)| {
            let mut tied_riders: Vec<_> = ranked
                .iter()
                .filter(|(_, s)| s.total == score.total)
                .map(|(r, _)| *r)
                .collect();
            tied_riders.sort();
            if tied_riders.len() > 1 {
                SelectionReason::ExactScoreThenRiderId { tied_riders }
            } else {
                SelectionReason::LowestScore
            }
        });
        Ok(BaselineRanking {
            scores,
            selected,
            reason,
        })
    }
}

/// Separate candidate-generation policy over Basic Dispatch eligible riders.
///
/// # Errors
/// Rejects inconsistent worlds/projections; radius/limit never implies complete coverage.
pub fn generate_candidates(
    snapshot: &DispatchSnapshot<'_>,
    order: OrderId,
    policy: CandidatePolicy,
) -> Result<Vec<RiderId>, DispatchEvaluationError> {
    let data = snapshot.world.data();
    let request = data
        .orders
        .get(&order)
        .ok_or(DispatchEvaluationError::InvalidRequest)?;
    let mut eligible = Vec::new();
    for rider in data.profiles.keys().copied() {
        if basic_dispatch_eligible(data, rider)? {
            eligible.push(rider);
        }
    }
    if let CandidatePolicy::Spatial { radius, limit } = policy {
        // Screening speed is fixed and never used by ranking; only positions matter.
        let speed = KilometersPerHour::new(30.0)
            .map_err(|_| DispatchEvaluationError::CandidateGeneration)?;
        let index = IndexedRiderLocator::new(
            eligible.iter().map(|r| RiderLocation {
                id: *r,
                coordinate: data.riders[r].coordinate,
            }),
            speed,
        )
        .map_err(|_| DispatchEvaluationError::CandidateGeneration)?;
        eligible = index
            .nearest_riders(request.pickup, radius, limit)
            .into_iter()
            .map(|c| c.rider_id)
            .collect();
    }
    eligible.sort();
    Ok(eligible)
}

fn checked_leg(
    snapshot: &DispatchSnapshot<'_>,
    from: &RoutingAnchor,
    to: &RoutingAnchor,
    at: DispatchInstant,
) -> Result<Option<crate::RoutedLeg>, DispatchEvaluationError> {
    let departure = snapshot.epoch.departure_seconds(at)?;
    if snapshot.provider.provenance() != snapshot.provenance {
        return Err(DispatchEvaluationError::ProvenanceMismatch);
    }
    let result = snapshot.provider.route(from.node, to.node, departure)?;
    if snapshot.provider.provenance() != snapshot.provenance {
        return Err(DispatchEvaluationError::ProvenanceMismatch);
    }
    match result {
        RouteOutcome::NoRoute => Ok(None),
        RouteOutcome::RouteFound(leg) => {
            if leg.provenance != snapshot.provenance
                || leg.departure != departure
                || leg.route.graph_snapshot_digest() != snapshot.provenance.graph_digest
                || leg.route.path().first() != Some(&from.node)
                || leg.route.path().last() != Some(&to.node)
                || leg.route.total_cost()
                    != RouteCost::from_travel_time(leg.route.elapsed_travel_time())
            {
                return Err(DispatchEvaluationError::ProvenanceMismatch);
            }
            Ok(Some(*leg))
        }
    }
}

fn evaluate_basic_plan(
    snapshot: &DispatchSnapshot<'_>,
    order: OrderId,
    plan: RiderPlan,
    pickup: &RoutingAnchor,
    dropoff: &RoutingAnchor,
    origin: &RoutingAnchor,
) -> Result<Option<PlanEvaluation>, DispatchEvaluationError> {
    let Some(first) = checked_leg(snapshot, origin, pickup, snapshot.at)? else {
        return Ok(None);
    };
    let pickup_timeline = StopTimeline::new(
        Stop::Pickup(order),
        snapshot.at.checked_add(first.route.elapsed_travel_time())?,
        Seconds::ZERO,
        Seconds::ZERO,
    )?;
    let Some(second) = checked_leg(snapshot, pickup, dropoff, pickup_timeline.departure)? else {
        return Ok(None);
    };
    let dropoff_timeline = StopTimeline::new(
        Stop::Dropoff(order),
        pickup_timeline
            .departure
            .checked_add(second.route.elapsed_travel_time())?,
        Seconds::ZERO,
        Seconds::ZERO,
    )?;
    let deadline = snapshot.world.data().orders[&order].deadline;
    let completed = dropoff_timeline.departure;
    let lateness = if let Some(d) = deadline.filter(|d| *d < completed) {
        completed.duration_since(d)?
    } else {
        Seconds::ZERO
    };
    Ok(Some(PlanEvaluation {
        plan,
        pickup_travel: first.route.elapsed_travel_time(),
        delivery_travel: second.route.elapsed_travel_time(),
        travel: first
            .route
            .elapsed_travel_time()
            .checked_add(second.route.elapsed_travel_time())
            .map_err(|_| DispatchEvaluationError::InvalidMetric)?,
        distance: first
            .route
            .total_distance()
            .checked_add(second.route.total_distance())
            .map_err(|_| DispatchEvaluationError::InvalidMetric)?,
        stops: vec![pickup_timeline, dropoff_timeline],
        pickup_arrival: pickup_timeline.arrival,
        pickup_departure: pickup_timeline.departure,
        dropoff_arrival: dropoff_timeline.arrival,
        delivery_completed_at: completed,
        waiting: Seconds::ZERO,
        service: Seconds::ZERO,
        lateness,
    }))
}

fn evaluate_candidates(
    snapshot: &DispatchSnapshot<'_>,
    order: OrderId,
    plan: &RiderPlan,
    pickup: &RoutingAnchor,
    dropoff: &RoutingAnchor,
    riders: Vec<RiderId>,
) -> Result<(Vec<CandidateEvidence>, Vec<FeasibleCandidate>), DispatchEvaluationError> {
    let mut evidence = Vec::new();
    let mut feasible = Vec::new();
    let data = snapshot.world.data();
    for rider in riders {
        let rejection = if data.profiles[&rider].routing_profile == snapshot.provenance.profile {
            let mut proposed = data.clone();
            proposed
                .assignments
                .insert(order, CommittedAssignment { order, rider });
            validate_plan(&proposed, rider, plan)
                .err()
                .map(CandidateRejection::from)
        } else {
            Some(CandidateRejection::UnsupportedCandidateProfile)
        };
        if let Some(reason) = rejection {
            evidence.push(CandidateEvidence {
                rider,
                result: CandidateResult::Rejected(reason),
            });
            continue;
        }
        if let Some(evaluation) = evaluate_basic_plan(
            snapshot,
            order,
            plan.clone(),
            pickup,
            dropoff,
            &snapshot.anchors.riders[&rider],
        )? {
            feasible.push(FeasibleCandidate { rider, evaluation });
        } else {
            evidence.push(CandidateEvidence {
                rider,
                result: CandidateResult::Rejected(CandidateRejection::NoRoute),
            });
        }
    }
    Ok((evidence, feasible))
}

/// Evaluates the narrow Phase 13 workflow without mutating its source world.
///
/// # Errors
/// Invalid inputs, anchors, numeric failures, and provider failures are decision
/// errors. Candidate NoRoute/profile/capacity rejection produces a completed decision.
pub fn basic_dispatch(
    snapshot: &DispatchSnapshot<'_>,
    order: OrderId,
    id: DecisionId,
    policy: CandidatePolicy,
) -> Result<AssignmentDecision, DispatchEvaluationError> {
    let data = snapshot.world.data();
    validate_world(data)?;
    let request = data
        .orders
        .get(&order)
        .ok_or(DispatchEvaluationError::InvalidRequest)?;
    if data.fulfillment[&order] != FulfillmentState::AwaitingPickup
        || data.assignments.contains_key(&order)
        || request.created_at > snapshot.at
    {
        return Err(DispatchEvaluationError::InvalidRequest);
    }
    let pickup = snapshot
        .anchors
        .pickups
        .get(&order)
        .ok_or(DispatchEvaluationError::MissingAnchor)?;
    let dropoff = snapshot
        .anchors
        .dropoffs
        .get(&order)
        .ok_or(DispatchEvaluationError::MissingAnchor)?;
    pickup.validate(snapshot.provider.graph(), request.pickup)?;
    dropoff.validate(snapshot.provider.graph(), request.dropoff)?;
    let riders = generate_candidates(snapshot, order, policy)?;
    // Validate all candidate endpoint inputs before interpreting structural rejections.
    for rider in &riders {
        snapshot
            .anchors
            .riders
            .get(rider)
            .ok_or(DispatchEvaluationError::MissingAnchor)?
            .validate(snapshot.provider.graph(), data.riders[rider].coordinate)?;
    }
    let plan = RiderPlan {
        stops: vec![Stop::Pickup(order), Stop::Dropoff(order)],
    };
    let (mut evidence, feasible) =
        evaluate_candidates(snapshot, order, &plan, pickup, dropoff, riders)?;
    let ranking = BaselineStrategy.rank(&feasible)?;
    for candidate in feasible {
        let score = ranking
            .scores
            .iter()
            .find(|(r, _)| *r == candidate.rider)
            .ok_or(DispatchEvaluationError::InvalidMetric)?
            .1;
        evidence.push(CandidateEvidence {
            rider: candidate.rider,
            result: CandidateResult::Feasible {
                evaluation: candidate.evaluation,
                score,
            },
        });
    }
    evidence.sort_by_key(|c| c.rider);
    let outcome = if let Some(rider) = ranking.selected {
        DispatchDecisionOutcome::Assigned(AssignmentProposal {
            order,
            rider,
            expected_assignment: data.assignments.get(&order).copied(),
            expected_plan: data.plans[&rider].clone(),
            proposed_assignment: CommittedAssignment { order, rider },
            proposed_plan: plan,
        })
    } else {
        DispatchDecisionOutcome::Unassigned {
            scope: match policy {
                CandidatePolicy::Exhaustive => UnassignedScope::EligibleFleet,
                CandidatePolicy::Spatial { .. } => UnassignedScope::EvaluatedCandidates,
            },
        }
    };
    Ok(AssignmentDecision::new(
        DecisionEvidence {
            decision_id: id,
            world_identity: snapshot.world.identity(),
            world_version: snapshot.world.version(),
            evaluated_at: snapshot.at,
            routing_epoch: snapshot.epoch,
            routing: snapshot.provenance.clone(),
            strategy: "basic-road-travel/v1".into(),
            feasibility_policy: "scalar-capacity-plan-custody-profile/v1".into(),
            eligibility_policy: "basic-idle/v1".into(),
            deadline_policy: DeadlinePolicy::SoftObserved,
            candidate_policy: policy,
            coverage: policy.coverage(),
            candidates: evidence,
            selection_reason: ranking.reason,
        },
        outcome,
    ))
}
