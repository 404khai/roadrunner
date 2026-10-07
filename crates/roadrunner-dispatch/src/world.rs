use roadrunner_core::geo::Coordinate;
use thiserror::Error;

use crate::{
    AssignmentDecision, AssignmentProposal, Availability, CapacityUnits, DispatchDecisionOutcome,
    DispatchEvaluationError, DispatchInstant, FulfillmentState, Order, OrderId, OrderReadiness,
    RiderId, RiderPlan, RiderState, Stop, WorldData, WorldVersion,
};

/// Invalid logical work or capacity/custody transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PlanValidityError {
    /// Pending stops disagree with assignment or fulfillment.
    #[error("invalid remaining plan")]
    InvalidPlan,
    /// Responsibility does not match the custody holder.
    #[error("custody violation")]
    CustodyViolation,
    /// Scalar demand exceeds supported rider capacity at some stop.
    #[error("scalar capacity exceeded")]
    CapacityExceeded,
}

/// Validates remaining logical work using execution state and responsibility.
///
/// # Errors
/// Rejects inconsistent stops, custody transfer, and load violations after any stop.
pub fn validate_plan(
    data: &WorldData,
    rider: RiderId,
    plan: &RiderPlan,
) -> Result<(), PlanValidityError> {
    let profile = data
        .profiles
        .get(&rider)
        .ok_or(PlanValidityError::InvalidPlan)?;
    for stop in &plan.stops {
        if data
            .assignments
            .get(&stop.order())
            .is_none_or(|a| a.rider != rider)
            || !data.orders.contains_key(&stop.order())
        {
            return Err(PlanValidityError::InvalidPlan);
        }
    }
    for (order, state) in &data.fulfillment {
        if matches!(state, FulfillmentState::PickedUp { rider: custody, .. } if *custody == rider)
            && data.assignments.get(order).is_none_or(|a| a.rider != rider)
        {
            return Err(PlanValidityError::CustodyViolation);
        }
    }
    let mut load = onboard_load(data, rider)?.value();
    if load > profile.max_capacity.value() {
        return Err(PlanValidityError::CapacityExceeded);
    }
    for assignment in data.assignments.values().filter(|a| a.rider == rider) {
        let order = assignment.order;
        let pickups: Vec<_> = plan
            .stops
            .iter()
            .enumerate()
            .filter_map(|(i, s)| (*s == Stop::Pickup(order)).then_some(i))
            .collect();
        let dropoffs: Vec<_> = plan
            .stops
            .iter()
            .enumerate()
            .filter_map(|(i, s)| (*s == Stop::Dropoff(order)).then_some(i))
            .collect();
        match data.fulfillment.get(&order) {
            Some(FulfillmentState::AwaitingPickup)
                if pickups.len() == 1 && dropoffs.len() == 1 && pickups[0] < dropoffs[0] => {}
            Some(FulfillmentState::PickedUp { rider: custody, .. }) => {
                if *custody != rider {
                    return Err(PlanValidityError::CustodyViolation);
                }
                if !pickups.is_empty() || dropoffs.len() != 1 {
                    return Err(PlanValidityError::InvalidPlan);
                }
            }
            _ => return Err(PlanValidityError::InvalidPlan),
        }
    }
    for stop in &plan.stops {
        let demand = data.orders[&stop.order()].demand.value();
        load = match stop {
            Stop::Pickup(_) => load.checked_add(demand),
            Stop::Dropoff(_) => load.checked_sub(demand),
        }
        .ok_or(PlanValidityError::CapacityExceeded)?;
        if load > profile.max_capacity.value() {
            return Err(PlanValidityError::CapacityExceeded);
        }
    }
    if load != 0 {
        return Err(PlanValidityError::InvalidPlan);
    }
    Ok(())
}

/// Derives onboard scalar demand solely from custody facts.
///
/// # Errors
/// Rejects absent requests or integer overflow; never infers load from order count.
pub fn onboard_load(data: &WorldData, rider: RiderId) -> Result<CapacityUnits, PlanValidityError> {
    let mut load = 0_u64;
    for (order, state) in &data.fulfillment {
        if matches!(state, FulfillmentState::PickedUp { rider: r, .. } if *r == rider) {
            load = load
                .checked_add(
                    data.orders
                        .get(order)
                        .ok_or(PlanValidityError::InvalidPlan)?
                        .demand
                        .value(),
                )
                .ok_or(PlanValidityError::CapacityExceeded)?;
        }
    }
    Ok(CapacityUnits::new(load))
}

/// Checks that a coherent world's authoritative records agree.
///
/// # Errors
/// Every contradiction is `InvalidWorldState`, not ordinary rider ineligibility.
pub fn validate_world(data: &WorldData) -> Result<(), DispatchEvaluationError> {
    let invalid = || DispatchEvaluationError::InvalidWorldState;
    crate::pooling::validate_accepted(data).map_err(|_| invalid())?;
    if !data.orders.keys().eq(data.fulfillment.keys())
        || !data.orders.keys().eq(data.readiness.keys())
        || !data.profiles.keys().eq(data.riders.keys())
        || !data.profiles.keys().eq(data.plans.keys())
    {
        return Err(invalid());
    }
    for (id, profile) in &data.profiles {
        if *id != profile.id || profile.routing_profile.is_empty() {
            return Err(invalid());
        }
        validate_plan(data, *id, &data.plans[id]).map_err(|_| invalid())?;
    }
    for (id, order) in &data.orders {
        let ready = data.readiness[id];
        if *id != order.id {
            return Err(invalid());
        }
        match data.fulfillment[id] {
            FulfillmentState::AwaitingPickup => {}
            FulfillmentState::PickedUp { rider, at } => {
                if at < order.created_at
                    || data.assignments.get(id).is_none_or(|a| a.rider != rider)
                    || ready.observed_at.is_none_or(|ready_at| ready_at > at)
                {
                    return Err(invalid());
                }
            }
            FulfillmentState::Delivered {
                rider,
                picked_up_at,
                at,
            } => {
                if picked_up_at < order.created_at
                    || at < picked_up_at
                    || !data.profiles.contains_key(&rider)
                    || data.assignments.contains_key(id)
                    || ready
                        .observed_at
                        .is_none_or(|ready_at| ready_at > picked_up_at)
                {
                    return Err(invalid());
                }
            }
        }
    }
    for (id, assignment) in &data.assignments {
        if *id != assignment.order
            || !data.orders.contains_key(id)
            || !data.profiles.contains_key(&assignment.rider)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Explicit optimistic commit or shared domain-transition rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum CommitError {
    /// Proposal's source world/version no longer matches.
    #[error("stale decision or world version")]
    Stale,
    /// Expected prior state or resulting transition violates the domain.
    #[error("invalid world transition")]
    InvalidTransition,
    /// Whole-world version cannot advance.
    #[error("world version exhausted")]
    VersionOverflow,
}

/// Mutable domain owner; evaluation sees only immutable borrows of its data.
#[derive(Debug)]
pub struct World {
    identity: u64,
    version: WorldVersion,
    data: WorldData,
}

impl World {
    /// Creates a validated world at version zero with caller-supplied identity.
    ///
    /// # Errors
    /// Rejects incoherent domain input.
    pub fn new(identity: u64, data: WorldData) -> Result<Self, DispatchEvaluationError> {
        validate_world(&data)?;
        Ok(Self {
            identity,
            version: WorldVersion::new(0),
            data,
        })
    }
    /// Returns the stable caller-supplied world identity.
    #[must_use]
    pub const fn identity(&self) -> u64 {
        self.identity
    }
    /// Returns the current whole-world optimistic version.
    #[must_use]
    pub const fn version(&self) -> WorldVersion {
        self.version
    }
    /// Exposes immutable authoritative data.
    #[must_use]
    pub const fn data(&self) -> &WorldData {
        &self.data
    }

    pub(crate) fn publish(&mut self, data: WorldData) -> Result<(), CommitError> {
        validate_world(&data).map_err(|_| CommitError::InvalidTransition)?;
        let version = self
            .version
            .value()
            .checked_add(1)
            .ok_or(CommitError::VersionOverflow)?;
        self.data = data;
        self.version = WorldVersion::new(version);
        Ok(())
    }

    /// Applies the exact selected responsibility and plan together.
    ///
    /// # Errors
    /// Rejects stale or invalid proposals without mutation or reevaluation.
    pub fn commit(&mut self, decision: &AssignmentDecision) -> Result<(), CommitError> {
        if decision.evidence().world_identity != self.identity
            || decision.evidence().world_version != self.version
        {
            return Err(CommitError::Stale);
        }
        let DispatchDecisionOutcome::Assigned(proposal) = decision.outcome() else {
            return Err(CommitError::InvalidTransition);
        };
        if self
            .data
            .profiles
            .get(&proposal.rider)
            .is_none_or(|profile| profile.routing_profile != decision.evidence().routing.profile)
        {
            return Err(CommitError::InvalidTransition);
        }
        self.validate_proposal(proposal)?;
        let mut next = self.data.clone();
        next.assignments
            .insert(proposal.order, proposal.proposed_assignment);
        next.plans
            .insert(proposal.rider, proposal.proposed_plan.clone());
        self.publish(next)
    }

    fn validate_proposal(&self, p: &AssignmentProposal) -> Result<(), CommitError> {
        if self.data.assignments.get(&p.order).copied() != p.expected_assignment
            || self.data.plans.get(&p.rider) != Some(&p.expected_plan)
            || self.data.fulfillment.get(&p.order) != Some(&FulfillmentState::AwaitingPickup)
            || !basic_dispatch_eligible(&self.data, p.rider)
                .map_err(|_| CommitError::InvalidTransition)?
            || p.expected_assignment.is_some()
            || !p.expected_plan.stops.is_empty()
            || p.proposed_assignment.order != p.order
            || p.proposed_assignment.rider != p.rider
            || p.proposed_plan.stops != [Stop::Pickup(p.order), Stop::Dropoff(p.order)]
        {
            return Err(CommitError::InvalidTransition);
        }
        Ok(())
    }

    /// Registers a new request, readiness, and awaiting-pickup state atomically.
    ///
    /// # Errors
    /// Rejects duplicate identities or an invalid resulting world without mutation.
    pub fn register_order(
        &mut self,
        order: Order,
        readiness: OrderReadiness,
    ) -> Result<(), CommitError> {
        if self.data.orders.contains_key(&order.id) {
            return Err(CommitError::InvalidTransition);
        }
        let mut next = self.data.clone();
        next.readiness.insert(order.id, readiness);
        next.fulfillment
            .insert(order.id, FulfillmentState::AwaitingPickup);
        next.orders.insert(order.id, order);
        self.publish(next)
    }

    /// Updates rider position/availability through the shared versioned boundary.
    ///
    /// # Errors
    /// Rejects absent rider or invalid resulting world without mutation.
    pub fn update_rider_state(
        &mut self,
        rider: RiderId,
        state: RiderState,
    ) -> Result<(), CommitError> {
        if !self.data.riders.contains_key(&rider) {
            return Err(CommitError::InvalidTransition);
        }
        let mut next = self.data.clone();
        next.riders.insert(rider, state);
        self.publish(next)
    }

    /// Updates only expected readiness; observations remain historical facts.
    ///
    /// # Errors
    /// Rejects absent order or invalid resulting world.
    pub fn estimate_readiness(
        &mut self,
        order: OrderId,
        at: DispatchInstant,
    ) -> Result<(), CommitError> {
        let mut next = self.data.clone();
        next.readiness
            .get_mut(&order)
            .ok_or(CommitError::InvalidTransition)?
            .expected_at = Some(at);
        self.publish(next)
    }

    /// Records actual readiness exactly once through the shared transition boundary.
    ///
    /// # Errors
    /// Rejects duplicate observation, absent order, or invalid resulting world.
    pub fn observe_ready(
        &mut self,
        order: OrderId,
        at: DispatchInstant,
    ) -> Result<(), CommitError> {
        let mut next = self.data.clone();
        let ready = next
            .readiness
            .get_mut(&order)
            .ok_or(CommitError::InvalidTransition)?;
        if ready.observed_at.is_some() {
            return Err(CommitError::InvalidTransition);
        }
        ready.observed_at = Some(at);
        self.publish(next)
    }

    /// Completes the next pickup, establishes custody, and advances the same plan.
    ///
    /// # Errors
    /// Rejects wrong rider, unready order, or out-of-order pickup without mutation.
    pub fn pickup(
        &mut self,
        rider: RiderId,
        order: OrderId,
        at: DispatchInstant,
    ) -> Result<(), CommitError> {
        self.complete_stop(rider, order, at, true)
    }

    /// Completes the next dropoff, clears active responsibility, and advances the plan.
    ///
    /// # Errors
    /// Rejects custody mismatch, reversed time, or out-of-order delivery.
    pub fn deliver(
        &mut self,
        rider: RiderId,
        order: OrderId,
        at: DispatchInstant,
    ) -> Result<(), CommitError> {
        self.complete_stop(rider, order, at, false)
    }

    fn complete_stop(
        &mut self,
        rider: RiderId,
        order: OrderId,
        at: DispatchInstant,
        pickup: bool,
    ) -> Result<(), CommitError> {
        let expected = if pickup {
            Stop::Pickup(order)
        } else {
            Stop::Dropoff(order)
        };
        let state = self
            .data
            .fulfillment
            .get(&order)
            .ok_or(CommitError::InvalidTransition)?;
        let state_ok = if pickup {
            *state == FulfillmentState::AwaitingPickup
                && self.data.readiness[&order]
                    .observed_at
                    .is_some_and(|t| t <= at)
        } else {
            matches!(state, FulfillmentState::PickedUp { rider: r, at: t } if *r == rider && *t <= at)
        };
        if !state_ok
            || self
                .data
                .assignments
                .get(&order)
                .is_none_or(|a| a.rider != rider)
            || self
                .data
                .plans
                .get(&rider)
                .and_then(|p| p.stops.first())
                .copied()
                != Some(expected)
        {
            return Err(CommitError::InvalidTransition);
        }
        let mut next = self.data.clone();
        let location: Coordinate = if pickup {
            next.orders[&order].pickup
        } else {
            next.orders[&order].dropoff
        };
        next.riders
            .get_mut(&rider)
            .ok_or(CommitError::InvalidTransition)?
            .coordinate = location;
        next.plans
            .get_mut(&rider)
            .ok_or(CommitError::InvalidTransition)?
            .stops
            .remove(0);
        next.fulfillment.insert(
            order,
            if pickup {
                FulfillmentState::PickedUp { rider, at }
            } else {
                FulfillmentState::Delivered {
                    rider,
                    picked_up_at: match *state {
                        FulfillmentState::PickedUp { at, .. } => at,
                        _ => return Err(CommitError::InvalidTransition),
                    },
                    at,
                }
            },
        );
        if !pickup {
            next.assignments.remove(&order);
        }
        self.publish(next)
    }
}

/// Phase 13 policy only: operationally available and coherently idle.
///
/// # Errors
/// Inconsistent responsibility, custody, or plan is an invalid world.
pub fn basic_dispatch_eligible(
    data: &WorldData,
    rider: RiderId,
) -> Result<bool, DispatchEvaluationError> {
    validate_world(data)?;
    let state = data
        .riders
        .get(&rider)
        .ok_or(DispatchEvaluationError::InvalidWorldState)?;
    Ok(state.availability == Availability::Available
        && !data.assignments.values().any(|a| a.rider == rider)
        && onboard_load(data, rider)
            .map_err(|_| DispatchEvaluationError::InvalidWorldState)?
            .value()
            == 0
        && data.plans[&rider].stops.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CandidateCoverage, CandidatePolicy, CommittedAssignment, DeadlinePolicy, DecisionEvidence,
        DecisionId, Order, OrderReadiness, RiderProfile, RoutingEpoch, RoutingProvenance,
        TrafficIdentity,
    };

    fn world_and_decision() -> (World, AssignmentDecision) {
        let coordinate = Coordinate::new(0.0, 0.0).unwrap_or_else(|e| panic!("coordinate: {e}"));
        let at = DispatchInstant::new(0.0).unwrap_or_else(|e| panic!("time: {e}"));
        let order = OrderId::new(1);
        let rider = RiderId::new(1);
        let mut data = WorldData::default();
        data.orders.insert(
            order,
            Order {
                id: order,
                pickup: coordinate,
                dropoff: coordinate,
                created_at: at,
                deadline: None,
                demand: CapacityUnits::new(2),
            },
        );
        data.readiness.insert(order, OrderReadiness::default());
        data.fulfillment
            .insert(order, FulfillmentState::AwaitingPickup);
        data.profiles.insert(
            rider,
            RiderProfile {
                id: rider,
                routing_profile: "test".into(),
                max_capacity: CapacityUnits::new(2),
            },
        );
        data.riders.insert(
            rider,
            RiderState {
                coordinate,
                availability: Availability::Available,
            },
        );
        data.plans.insert(rider, RiderPlan::default());
        let world = World::new(7, data).unwrap_or_else(|e| panic!("world: {e}"));
        let decision = AssignmentDecision::new(
            DecisionEvidence {
                decision_id: DecisionId::new(1),
                world_identity: 7,
                world_version: WorldVersion::new(0),
                evaluated_at: at,
                routing_epoch: RoutingEpoch(at),
                routing: RoutingProvenance {
                    graph_digest: "test".into(),
                    traffic: TrafficIdentity::FreeFlow,
                    profile: "test".into(),
                },
                strategy: "basic-road-travel/v1".into(),
                preparation: None,
                feasibility_policy: "test".into(),
                eligibility_policy: "basic-idle/v1".into(),
                deadline_policy: DeadlinePolicy::SoftObserved,
                candidate_policy: CandidatePolicy::Exhaustive,
                coverage: CandidateCoverage::Complete,
                candidates: Vec::new(),
                selection_reason: None,
            },
            DispatchDecisionOutcome::Assigned(AssignmentProposal {
                order,
                rider,
                expected_assignment: None,
                expected_plan: RiderPlan::default(),
                proposed_assignment: CommittedAssignment { order, rider },
                proposed_plan: RiderPlan {
                    stops: vec![Stop::Pickup(order), Stop::Dropoff(order)],
                },
            }),
        );
        (world, decision)
    }

    #[test]
    fn expected_state_and_transition_invariants_reject_without_partial_mutation() {
        let (mut world, decision) = world_and_decision();
        let DispatchDecisionOutcome::Assigned(proposal) = decision.outcome() else {
            panic!("assigned")
        };
        let before = world.data.clone();
        let mut wrong_prior = proposal.clone();
        wrong_prior.expected_plan = wrong_prior.proposed_plan.clone();
        let invalid = AssignmentDecision::new(
            decision.evidence().clone(),
            DispatchDecisionOutcome::Assigned(wrong_prior),
        );
        assert_eq!(world.commit(&invalid), Err(CommitError::InvalidTransition));
        assert_eq!(world.data, before);
        let mut invalid_plan = proposal.clone();
        invalid_plan.proposed_plan.stops.reverse();
        let invalid = AssignmentDecision::new(
            decision.evidence().clone(),
            DispatchDecisionOutcome::Assigned(invalid_plan),
        );
        assert_eq!(world.commit(&invalid), Err(CommitError::InvalidTransition));
        assert_eq!(world.data, before);
        // Defense in depth: a malformed selected proposal cannot bypass capacity
        // even if it reaches commit with the matching version and expected plan.
        world
            .data
            .profiles
            .get_mut(&RiderId::new(1))
            .unwrap_or_else(|| panic!("rider"))
            .max_capacity = CapacityUnits::new(1);
        let before = world.data.clone();
        assert_eq!(world.commit(&decision), Err(CommitError::InvalidTransition));
        assert_eq!(world.data, before);
        assert_eq!(world.version, WorldVersion::new(0));
    }

    #[test]
    fn version_overflow_does_not_publish_half_a_transition() {
        let (mut world, decision) = world_and_decision();
        world.version = WorldVersion::new(u64::MAX);
        let mut evidence = decision.evidence().clone();
        evidence.world_version = world.version;
        let decision = AssignmentDecision::new(evidence, decision.outcome().clone());
        let before = world.data.clone();
        assert_eq!(world.commit(&decision), Err(CommitError::VersionOverflow));
        assert_eq!(world.data, before);
        assert_eq!(world.version, WorldVersion::new(u64::MAX));
    }
    #[test]
    fn order_registration_is_atomic_and_rejects_duplicates_and_version_overflow() {
        let (mut world, _) = world_and_decision();
        let mut order = world.data.orders[&OrderId::new(1)].clone();
        order.id = OrderId::new(2);
        assert!(
            world
                .register_order(order.clone(), OrderReadiness::default())
                .is_ok()
        );
        assert_eq!(world.version, WorldVersion::new(1));
        assert_eq!(
            world.data.fulfillment[&order.id],
            FulfillmentState::AwaitingPickup
        );
        assert!(world.data.readiness.contains_key(&order.id));
        assert!(validate_world(&world.data).is_ok());
        let before = world.data.clone();
        assert_eq!(
            world.register_order(order.clone(), OrderReadiness::default()),
            Err(CommitError::InvalidTransition)
        );
        assert_eq!(world.data, before);
        assert_eq!(world.version, WorldVersion::new(1));
        world.version = WorldVersion::new(u64::MAX);
        order.id = OrderId::new(3);
        assert_eq!(
            world.register_order(order, OrderReadiness::default()),
            Err(CommitError::VersionOverflow)
        );
        assert_eq!(world.data, before);
        assert_eq!(world.version, WorldVersion::new(u64::MAX));
    }
}
