//! Coherent, infrastructure-independent operational authority.
//!
//! No HTTP, storage, host clock or simulator queue participates in domain validity.
use std::collections::BTreeMap;
use std::ops::Deref;

use roadrunner_core::geo::{Meters, Seconds};
use roadrunner_core::graph::{EdgeId, NodeId};
use serde::{Deserialize, Serialize};

use crate::{
    AssignmentDecision, CommitError, DispatchInstant, DispatchTimeError, FleetContext,
    FleetDecision, InsertionDecision, Order, OrderId, OrderReadiness, PoolingContext,
    RecoveryContext, RecoveryDecision, RiderId, RiderState, RoutingAnchor, RoutingProvenance, Stop,
    World, validate_world,
};

/// Immutable namespace token. Live adapters must allocate independently unique tokens;
/// deterministic fixtures may explicitly supply their own token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OperationalWorldNamespace([u8; 16]);
impl OperationalWorldNamespace {
    /// Allocates a fresh volatile namespace from the operating system entropy source.
    ///
    /// # Errors
    /// Fails without establishing an authority if entropy is unavailable.
    pub fn fresh() -> Result<Self, getrandom::Error> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes)?;
        Ok(Self(bytes))
    }
    /// Establishes an explicit fixture/restoration namespace, not a live allocator.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    /// Lossless namespace-qualified public resource reference; integers are strings.
    #[must_use]
    pub fn reference(self, kind: &str, id: u64) -> String {
        let token: String = self.0.iter().fold(String::new(), |mut value, byte| {
            use std::fmt::Write as _;
            let _ = write!(value, "{byte:02x}");
            value
        });
        format!("{token}:{kind}:{id}")
    }
}

macro_rules! token {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
        )]
        pub struct $name(u64);
        impl $name {
            /// Explicit fixture/restoration value; allocation remains authority-controlled.
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }
            /// Numeric value within its declared scope.
            #[must_use]
            pub const fn value(self) -> u64 {
                self.0
            }
            fn next(self) -> Result<Self, CommitError> {
                self.0
                    .checked_add(1)
                    .map(Self)
                    .ok_or(CommitError::VersionOverflow)
            }
        }
    };
}
token!(
    PlanRevision,
    "Monotonic effective logical-plan revision per rider."
);
token!(
    ActionId,
    "Immutable started action identity within the operational namespace."
);
token!(
    ScheduleGeneration,
    "Scheduled-attempt generation, independent of logical action."
);
token!(
    AppliedExecutionEffectId,
    "Recognized transition-effect identity within the namespace."
);
token!(
    AdoptedContextRevision,
    "Monotonic authoritative context adoption generation."
);

/// Whole-operational revision; meaningful transitions advance it exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationalRevision {
    /// Owning immutable namespace.
    pub namespace: OperationalWorldNamespace,
    /// Monotonic state revision, not a timestamp.
    pub number: u64,
}

/// Immutable external semantic content identity, separate from adoption order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdoptedContextIdentity {
    /// Named category (graph/traffic/readiness/profile/policy).
    pub category: String,
    /// Immutable semantic identifier or verified digest.
    pub content: String,
}

/// Versioned adoption record; returning A after B still changes revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdoptedContext {
    /// Immutable content reference.
    pub identity: AdoptedContextIdentity,
    /// Operational adoption revision.
    pub revision: AdoptedContextRevision,
}

/// Pinned departed route, retained across suffix replacement and graph adoption.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrozenExecutionLeg {
    /// Snapshot-local origin under routing provenance.
    pub from: NodeId,
    /// Snapshot-local destination under routing provenance.
    pub to: NodeId,
    /// Actual departure instant.
    pub departed_at: DispatchInstant,
    /// Predicted road duration (not proof of observed completion).
    pub travel: Seconds,
    /// Road distance.
    pub distance: Meters,
    /// Pinned snapshot-local path.
    pub nodes: Vec<NodeId>,
    /// Pinned directed traversals.
    pub edges: Vec<EdgeId>,
    /// Originating graph/profile/traffic.
    pub routing: RoutingProvenance,
}

/// Authoritative active stage. Timers cannot directly imply physical effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ExecutionStage {
    /// Departed route must complete unchanged.
    Travelling,
    /// Authoritative arrival accepted; service has not started.
    Arrived,
    /// Active destination wait, not editable work.
    Waiting,
    /// Active deterministic service.
    Servicing,
}

/// Started work independent of effective editable plan revision.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActiveExecution {
    /// Immutable logical action identity.
    pub action_id: ActionId,
    /// Current scheduled attempt (historical simulator event token representation).
    pub id: u64,
    /// Effective attempt generation.
    pub generation: ScheduleGeneration,
    /// Plan revision from which this action started; later revisions need not match.
    pub originating_plan: PlanRevision,
    /// Frozen destination/effect.
    pub stop: Stop,
    /// Current predicted arrival, not actual observation.
    pub arrival: DispatchInstant,
    /// Deterministic service completion when service has begun.
    pub service_end: Option<DispatchInstant>,
    /// Observed waiting duration for driver metrics.
    pub service_wait: Option<Seconds>,
    /// Frozen destination under originating snapshot.
    pub anchor: RoutingAnchor,
    /// Pinned original route, retained after arrival for interpretation.
    pub route: FrozenExecutionLeg,
    /// Uncompleted road leg; absent after accepted arrival.
    pub leg: Option<FrozenExecutionLeg>,
    /// Authoritative action stage.
    pub stage: ExecutionStage,
}

/// Distinct effects of one started action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum ExecutionEffect {
    /// Accepted endpoint arrival.
    Arrival,
    /// Accepted completion of pickup/dropoff service.
    Completion,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct AppliedEffect {
    id: AppliedExecutionEffectId,
    action: ActionId,
    rider: RiderId,
    generation: ScheduleGeneration,
    effect: ExecutionEffect,
    at: DispatchInstant,
}

/// Explicit effect result; duplicates do not change authority or revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectOutcome {
    /// Exactly one new effect atomically published.
    Applied,
    /// Same effect already recognized (including a distinct input identity).
    AlreadyApplied,
}

/// One successfully linearized transition. Attempts and duplicate ledger aliases
/// do not masquerade as new committed business history.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OperationalTransitionRecord {
    /// Expected authoritative predecessor.
    pub before: OperationalRevision,
    /// Exactly one succeeding operational revision.
    pub after: OperationalRevision,
    /// Domain version before publication (legacy compatibility scope).
    pub world_before: crate::WorldVersion,
    /// Domain version after publication.
    pub world_after: crate::WorldVersion,
    /// Changed effective plans.
    pub plans_changed: Vec<RiderId>,
    /// Changed active execution records.
    pub execution_changed: Vec<RiderId>,
    /// Context categories adopted by this authoritative transition.
    pub contexts_changed: Vec<String>,
    /// Recovery eligibility before the transition.
    pub recovery_before: Option<DispatchInstant>,
    /// Authentic recovery commitment instant after publication.
    pub recovery_after: Option<DispatchInstant>,
    /// Newly applied logical effects.
    pub applied_effects: Vec<AppliedExecutionEffectId>,
}

/// Detached immutable planning snapshot. Expensive evaluation need not hold the
/// authoritative writer; no mutable access or publication into this copy is exposed.
#[derive(Debug)]
pub struct OperationalPlanningSnapshot {
    state: OperationalState,
}
impl Deref for OperationalPlanningSnapshot {
    type Target = OperationalState;
    fn deref(&self) -> &OperationalState {
        &self.state
    }
}

/// Shared authority, not a simulator or generic runtime framework.
#[derive(Debug, PartialEq)]
pub struct OperationalState {
    pub(crate) world: World,
    compatibility: bool,
    revision: OperationalRevision,
    plans: BTreeMap<RiderId, PlanRevision>,
    execution: BTreeMap<RiderId, ActiveExecution>,
    contexts: BTreeMap<String, AdoptedContext>,
    effects: BTreeMap<AppliedExecutionEffectId, AppliedEffect>,
    next_action: ActionId,
    next_attempt: u64,
    next_effect: AppliedExecutionEffectId,
    next_order: u64,
    next_rider: u64,
    recovery_at: Option<DispatchInstant>,
    history: Vec<OperationalTransitionRecord>,
    pub(crate) publications: Vec<crate::PublicationProvenance>,
}
impl Deref for OperationalState {
    type Target = World;
    fn deref(&self) -> &World {
        &self.world
    }
}
impl OperationalState {
    pub(crate) fn staged_copy(&self) -> Self {
        Self {
            world: self.world.clone(),
            compatibility: self.compatibility,
            revision: self.revision,
            plans: self.plans.clone(),
            execution: self.execution.clone(),
            contexts: self.contexts.clone(),
            effects: self.effects.clone(),
            next_action: self.next_action,
            next_attempt: self.next_attempt,
            next_effect: self.next_effect,
            next_order: self.next_order,
            next_rider: self.next_rider,
            recovery_at: self.recovery_at,
            history: self.history.clone(),
            publications: self.publications.clone(),
        }
    }
    /// Explicit trusted simulation authority. Cannot convert an existing operational
    /// authority into compatibility mode. Simulation namespaces are fixture-local.
    ///
    /// # Errors
    /// Rejects invalid state/counters.
    pub fn for_simulation(
        namespace: OperationalWorldNamespace,
        world: World,
    ) -> Result<Self, CommitError> {
        let mut state = Self::new(namespace, world)?;
        state.compatibility = true;
        Ok(state)
    }

    /// Establishes a validated authority. A discarded live lifetime must receive a
    /// freshly allocated namespace; this constructor never invents durable history.
    ///
    /// # Errors
    /// Rejects exhausted initial counters or invalid authoritative state.
    pub fn new(namespace: OperationalWorldNamespace, world: World) -> Result<Self, CommitError> {
        let next_order = world
            .data()
            .orders
            .keys()
            .map(|o| o.value())
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(CommitError::VersionOverflow)?;
        let next_rider = world
            .data()
            .riders
            .keys()
            .map(|r| r.value())
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(CommitError::VersionOverflow)?;
        let plans = world
            .data()
            .plans
            .keys()
            .map(|r| (*r, PlanRevision::new(0)))
            .collect();
        let state = Self {
            world,
            compatibility: false,
            revision: OperationalRevision {
                namespace,
                number: 0,
            },
            plans,
            execution: BTreeMap::new(),
            contexts: BTreeMap::new(),
            effects: BTreeMap::new(),
            next_action: ActionId::new(0),
            next_attempt: 0,
            next_effect: AppliedExecutionEffectId::new(0),
            next_order,
            next_rider,
            recovery_at: None,
            history: Vec::new(),
            publications: Vec::new(),
        };
        state.validate()?;
        Ok(state)
    }
    /// Captures complete coherent state for evaluation outside the writer boundary.
    #[must_use]
    pub fn snapshot(&self) -> OperationalPlanningSnapshot {
        OperationalPlanningSnapshot {
            state: self.staged_copy(),
        }
    }
    /// Recorded successful temporal publications, immutable and actual-time stamped.
    #[must_use]
    pub fn publications(&self) -> &[crate::PublicationProvenance] {
        &self.publications
    }
    /// Committed transition history, not attempted operations.
    #[must_use]
    pub fn history(&self) -> &[OperationalTransitionRecord] {
        &self.history
    }
    /// Current coherent revision.
    #[must_use]
    pub const fn revision(&self) -> OperationalRevision {
        self.revision
    }
    /// Active authority, exposed read-only.
    #[must_use]
    pub const fn execution(&self) -> &BTreeMap<RiderId, ActiveExecution> {
        &self.execution
    }
    /// Effective plan revisions, exposed read-only.
    #[must_use]
    pub const fn plan_revisions(&self) -> &BTreeMap<RiderId, PlanRevision> {
        &self.plans
    }
    /// Adopted contexts, exposed read-only.
    #[must_use]
    pub const fn contexts(&self) -> &BTreeMap<String, AdoptedContext> {
        &self.contexts
    }
    /// Last successful recovery instant.
    #[must_use]
    pub const fn recovery_at(&self) -> Option<DispatchInstant> {
        self.recovery_at
    }
    /// Complete cross-domain/execution validation.
    ///
    /// # Errors
    /// Rejects owner, frozen destination, stage or snapshot inconsistencies.
    pub fn validate(&self) -> Result<(), CommitError> {
        validate_world(self.world.data()).map_err(|_| CommitError::InvalidTransition)?;
        for (r, a) in &self.execution {
            if self.world.data().plans.get(r).and_then(|p| p.stops.first()) != Some(&a.stop)
                || self
                    .world
                    .data()
                    .assignments
                    .get(&a.stop.order())
                    .is_none_or(|o| o.rider != *r)
                || a.anchor.graph_digest != a.route.routing.graph_digest
                || a.anchor.node != a.route.to
                || (a.stage == ExecutionStage::Travelling) != a.leg.is_some()
                || (a.stage == ExecutionStage::Servicing) != a.service_end.is_some()
            {
                return Err(CommitError::InvalidTransition);
            }
        }
        Ok(())
    }
    pub(crate) fn transaction<T>(
        &mut self,
        expected: OperationalRevision,
        apply: impl FnOnce(&mut Self) -> Result<T, CommitError>,
    ) -> Result<T, CommitError> {
        if expected != self.revision {
            return Err(CommitError::Stale);
        }
        let mut next = self.staged_copy();
        let result = apply(&mut next)?;
        next.validate()?;
        for (r, plan) in &next.world.data().plans {
            if self.world.data().plans.get(r) != Some(plan) {
                let revision = if let Some(previous) = self.plans.get(r) {
                    previous.next()?
                } else {
                    PlanRevision::new(0)
                };
                next.plans.insert(*r, revision);
            }
        }
        next.revision.number = self
            .revision
            .number
            .checked_add(1)
            .ok_or(CommitError::VersionOverflow)?;
        let record = OperationalTransitionRecord {
            before: self.revision,
            after: next.revision,
            world_before: self.world.version(),
            world_after: next.world.version(),
            plans_changed: next
                .plans
                .iter()
                .filter(|(r, p)| self.plans.get(r) != Some(p))
                .map(|(r, _)| *r)
                .collect(),
            execution_changed: self
                .execution
                .keys()
                .chain(next.execution.keys())
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .filter(|r| self.execution.get(r) != next.execution.get(r))
                .collect(),
            contexts_changed: next
                .contexts
                .iter()
                .filter(|(category, adopted)| self.contexts.get(*category) != Some(adopted))
                .map(|(category, _)| category.clone())
                .collect(),
            recovery_before: self.recovery_at,
            recovery_after: next.recovery_at,
            applied_effects: next
                .effects
                .keys()
                .filter(|id| !self.effects.contains_key(id))
                .copied()
                .collect(),
        };
        next.history.push(record);
        *self = next;
        Ok(result)
    }
    /// Adopts validated immutable input. Same-content re-adoption advances its revision.
    ///
    /// # Errors
    /// Rejects stale authority, empty identities or exhausted counters.
    pub fn adopt(
        &mut self,
        expected: OperationalRevision,
        identity: AdoptedContextIdentity,
    ) -> Result<(), CommitError> {
        self.transaction(expected, |s| {
            if identity.category.is_empty() || identity.content.is_empty() {
                return Err(CommitError::InvalidTransition);
            }
            let revision = s
                .contexts
                .get(&identity.category)
                .map_or(Ok(AdoptedContextRevision::new(1)), |c| c.revision.next())?;
            s.contexts.insert(
                identity.category.clone(),
                AdoptedContext { identity, revision },
            );
            Ok(())
        })
    }
    pub(crate) fn adopt_batch(
        &mut self,
        expected: OperationalRevision,
        content: BTreeMap<String, String>,
    ) -> Result<(), CommitError> {
        self.transaction(expected, |s| {
            for (category, value) in content {
                if category.is_empty() || value.is_empty() {
                    return Err(CommitError::InvalidTransition);
                }
                let revision = s
                    .contexts
                    .get(&category)
                    .map_or(Ok(AdoptedContextRevision::new(1)), |c| c.revision.next())?;
                s.contexts.insert(
                    category.clone(),
                    AdoptedContext {
                        identity: AdoptedContextIdentity {
                            category,
                            content: value,
                        },
                        revision,
                    },
                );
            }
            Ok(())
        })
    }
    /// Allocates and creates an order in one publication. No admission is implied.
    ///
    /// # Errors
    /// Rejects invalid state/request or counter exhaustion with no allocation leak.
    pub fn create_order(
        &mut self,
        expected: OperationalRevision,
        mut order: Order,
        readiness: OrderReadiness,
    ) -> Result<OrderId, CommitError> {
        self.transaction(expected, |s| {
            let id = OrderId::new(s.next_order);
            s.next_order = s
                .next_order
                .checked_add(1)
                .ok_or(CommitError::VersionOverflow)?;
            order.id = id;
            s.world.register_order(order, readiness)?;
            Ok(id)
        })
    }
    /// Registers a server-allocated rider atomically; supplied profile ID is replaced.
    ///
    /// # Errors
    /// Rejects stale/invalid state or allocation exhaustion.
    pub fn register_rider(
        &mut self,
        expected: OperationalRevision,
        mut profile: crate::RiderProfile,
        state: RiderState,
    ) -> Result<RiderId, CommitError> {
        self.transaction(expected, |s| {
            let id = RiderId::new(s.next_rider);
            s.next_rider = s
                .next_rider
                .checked_add(1)
                .ok_or(CommitError::VersionOverflow)?;
            profile.id = id;
            let mut data = s.world.data().clone();
            data.profiles.insert(id, profile);
            data.riders.insert(id, state);
            data.plans.insert(id, crate::RiderPlan::default());
            s.world.publish(data)?;
            s.plans.insert(id, PlanRevision::new(0));
            Ok(id)
        })
    }
    /// Checked available next rider identity. Actual registration is an atomic transition.
    #[must_use]
    pub const fn next_rider_id(&self) -> RiderId {
        RiderId::new(self.next_rider)
    }
    /// Selects next committed work; active frozen work must finish first.
    ///
    /// # Errors
    /// Rejects an absent rider or an already-started action.
    pub fn next_stop(&self, rider: RiderId) -> Result<Option<Stop>, CommitError> {
        if self.execution.contains_key(&rider) {
            return Err(CommitError::InvalidTransition);
        }
        Ok(self
            .world
            .data()
            .plans
            .get(&rider)
            .ok_or(CommitError::InvalidTransition)?
            .stops
            .first()
            .copied())
    }
    /// Starts the current effective first stop using a pinned route.
    ///
    /// # Errors
    /// Rejects stale authority, active work, wrong destination/owner or invalid route.
    pub fn start_action(
        &mut self,
        expected: OperationalRevision,
        rider: RiderId,
        expected_plan: PlanRevision,
        route: FrozenExecutionLeg,
        origin: &RoutingAnchor,
        anchor: RoutingAnchor,
    ) -> Result<u64, CommitError> {
        self.transaction(expected, |s| {
            for (category, content) in [
                (
                    "traffic",
                    crate::temporal::fingerprint(&route.routing.traffic),
                ),
                (
                    "profile",
                    crate::temporal::fingerprint(&route.routing.profile),
                ),
            ] {
                if s.contexts.get(category).is_some_and(|adopted| {
                    content
                        .as_ref()
                        .map_or(true, |identity| adopted.identity.content != *identity)
                }) {
                    return Err(CommitError::InvalidTransition);
                }
            }
            if s.effects
                .values()
                .any(|effect| effect.rider == rider && effect.at > route.departed_at)
                || s.contexts
                    .get("graph")
                    .is_some_and(|context| context.identity.content != route.routing.graph_digest)
                || s.world
                    .data()
                    .profiles
                    .get(&rider)
                    .is_none_or(|profile| profile.routing_profile != route.routing.profile)
                || s.plans.get(&rider) != Some(&expected_plan)
                || origin.node != route.from
                || origin.graph_digest != route.routing.graph_digest
                || s.world
                    .data()
                    .riders
                    .get(&rider)
                    .is_none_or(|r| r.coordinate != origin.coordinate)
                || s.execution.contains_key(&rider)
                || route.to != anchor.node
                || route.routing.graph_digest != anchor.graph_digest
                || route.nodes.first() != Some(&route.from)
                || route.nodes.last() != Some(&route.to)
            {
                return Err(CommitError::InvalidTransition);
            }
            let stop = *s
                .world
                .data()
                .plans
                .get(&rider)
                .and_then(|p| p.stops.first())
                .ok_or(CommitError::InvalidTransition)?;
            let order = &s.world.data().orders[&stop.order()];
            let destination = if matches!(stop, Stop::Pickup(_)) {
                order.pickup
            } else {
                order.dropoff
            };
            if anchor.coordinate != destination {
                return Err(CommitError::InvalidTransition);
            }
            let action_id = s.next_action;
            s.next_action = action_id.next()?;
            let id = s.allocate_attempt()?;
            let arrival = route
                .departed_at
                .checked_add(route.travel)
                .map_err(|_| CommitError::InvalidTransition)?;
            s.execution.insert(
                rider,
                ActiveExecution {
                    action_id,
                    id,
                    generation: ScheduleGeneration::new(0),
                    originating_plan: s.plans[&rider],
                    stop,
                    arrival,
                    service_end: None,
                    service_wait: None,
                    anchor,
                    route: route.clone(),
                    leg: Some(route),
                    stage: ExecutionStage::Travelling,
                },
            );
            Ok(id)
        })
    }
    fn allocate_attempt(&mut self) -> Result<u64, CommitError> {
        let id = self.next_attempt;
        self.next_attempt = id.checked_add(1).ok_or(CommitError::VersionOverflow)?;
        Ok(id)
    }
    /// Reschedules only the attempt; the logical action and originating route survive.
    ///
    /// # Errors
    /// Rejects a nontravelling action or invalid/exhausted arithmetic.
    pub fn delay_action(
        &mut self,
        expected: OperationalRevision,
        rider: RiderId,
        duration: Seconds,
    ) -> Result<ActiveExecution, CommitError> {
        self.transaction(expected, |s| {
            let id = s.allocate_attempt()?;
            let a = s
                .execution
                .get_mut(&rider)
                .ok_or(CommitError::InvalidTransition)?;
            if a.stage != ExecutionStage::Travelling {
                return Err(CommitError::InvalidTransition);
            }
            a.id = id;
            a.generation = a.generation.next()?;
            a.arrival = a
                .arrival
                .checked_add(duration)
                .map_err(|_| CommitError::InvalidTransition)?;
            let leg = a.leg.as_mut().ok_or(CommitError::InvalidTransition)?;
            leg.travel = leg
                .travel
                .checked_add(duration)
                .map_err(|_| CommitError::InvalidTransition)?;
            Ok(a.clone())
        })
    }
    fn duplicate(&self, effect: &AppliedEffect) -> Result<bool, CommitError> {
        if let Some(old) = self.effects.get(&effect.id) {
            if old != effect {
                return Err(CommitError::InvalidTransition);
            }
            return Ok(true);
        }
        if let Some(old) = self
            .effects
            .values()
            .find(|e| e.action == effect.action && e.effect == effect.effect)
        {
            if old.rider != effect.rider
                || old.generation != effect.generation
                || old.at != effect.at
            {
                return Err(CommitError::InvalidTransition);
            }
            return Ok(true);
        }
        Ok(false)
    }
    /// Applies an authorized arrival/service-completion observation exactly once.
    /// Caller authorization/authenticity is adapter-owned; validity is checked here.
    ///
    /// # Errors
    /// Rejects conflicting identity, stale state, wrong action/stage/generation or timing.
    #[allow(clippy::too_many_arguments)] // Distinct identity/precondition scopes are explicit at this boundary.
    pub fn apply_effect(
        &mut self,
        expected: OperationalRevision,
        id: AppliedExecutionEffectId,
        rider: RiderId,
        action: ActionId,
        generation: ScheduleGeneration,
        effect: ExecutionEffect,
        at: DispatchInstant,
    ) -> Result<EffectOutcome, CommitError> {
        let record = AppliedEffect {
            id,
            action,
            rider,
            generation,
            effect,
            at,
        };
        if self.duplicate(&record)? {
            if !self.effects.contains_key(&id) {
                let next = self.next_effect.max(id.next()?);
                self.effects.insert(id, record);
                self.next_effect = next;
            }
            return Ok(EffectOutcome::AlreadyApplied);
        }
        self.transaction(expected, |s| {
            let a = s
                .execution
                .get(&rider)
                .cloned()
                .ok_or(CommitError::InvalidTransition)?;
            if a.action_id != action || a.generation != generation {
                return Err(CommitError::InvalidTransition);
            }
            match effect {
                ExecutionEffect::Arrival => {
                    if a.stage != ExecutionStage::Travelling || at < a.route.departed_at {
                        return Err(CommitError::InvalidTransition);
                    }
                    let mut state = s.world.data().riders[&rider];
                    state.coordinate = a.anchor.coordinate;
                    s.world.update_rider_state(rider, state)?;
                    let active = s
                        .execution
                        .get_mut(&rider)
                        .ok_or(CommitError::InvalidTransition)?;
                    active.leg = None;
                    active.arrival = at;
                    active.stage = ExecutionStage::Arrived;
                }
                ExecutionEffect::Completion => {
                    if a.stage != ExecutionStage::Servicing
                        || a.service_end.is_none_or(|end| at < end)
                    {
                        return Err(CommitError::InvalidTransition);
                    }
                    s.execution.remove(&rider);
                    match a.stop {
                        Stop::Pickup(o) => s.world.pickup(rider, o, at)?,
                        Stop::Dropoff(o) => s.world.deliver(rider, o, at)?,
                    }
                }
            }
            s.next_effect = s.next_effect.max(id.next()?);
            s.effects.insert(id, record);
            Ok(EffectOutcome::Applied)
        })
    }
    /// Driver convenience: allocate effect identity and use the shared validity boundary.
    ///
    /// # Errors
    /// Rejects absent/invalid active stage with no state change.
    pub fn observe_execution(
        &mut self,
        rider: RiderId,
        effect: ExecutionEffect,
        at: DispatchInstant,
    ) -> Result<EffectOutcome, CommitError> {
        let a = self
            .execution
            .get(&rider)
            .cloned()
            .ok_or(CommitError::InvalidTransition)?;
        let id = self.next_effect;
        let outcome = self.apply_effect(
            self.revision,
            id,
            rider,
            a.action_id,
            a.generation,
            effect,
            at,
        )?;
        Ok(outcome)
    }
    /// Starts deterministic service or waiting at an already accepted arrival.
    ///
    /// # Errors
    /// Rejects invalid stage or missing readiness; no timer establishes completion.
    pub fn start_service(
        &mut self,
        expected: OperationalRevision,
        rider: RiderId,
        at: DispatchInstant,
        duration: Seconds,
    ) -> Result<(), CommitError> {
        self.transaction(expected, |s| {
            let a = s
                .execution
                .get_mut(&rider)
                .ok_or(CommitError::InvalidTransition)?;
            if !matches!(a.stage, ExecutionStage::Arrived | ExecutionStage::Waiting)
                || at < a.arrival
            {
                return Err(CommitError::InvalidTransition);
            }
            if matches!(a.stop, Stop::Pickup(_))
                && s.world.data().readiness[&a.stop.order()]
                    .observed_at
                    .is_none_or(|t| t > at)
            {
                return Err(CommitError::InvalidTransition);
            }
            let prescribed =
                s.world
                    .data()
                    .accepted
                    .get(&a.stop.order())
                    .map_or(Seconds::ZERO, |terms| {
                        if matches!(a.stop, Stop::Pickup(_)) {
                            terms.policy.pickup_service
                        } else {
                            terms.policy.dropoff_service
                        }
                    });
            if duration != prescribed {
                return Err(CommitError::InvalidTransition);
            }
            a.service_end = Some(
                at.checked_add(duration)
                    .map_err(|_| CommitError::InvalidTransition)?,
            );
            a.service_wait = Some(
                at.duration_since(a.arrival)
                    .map_err(|_| CommitError::InvalidTransition)?,
            );
            a.stage = ExecutionStage::Servicing;
            Ok(())
        })
    }
    /// Preserves active readiness waiting as authoritative frozen work.
    ///
    /// # Errors
    /// Rejects any stage other than a pickup arrival.
    pub fn wait_for_readiness(
        &mut self,
        expected: OperationalRevision,
        rider: RiderId,
    ) -> Result<(), CommitError> {
        self.transaction(expected, |s| {
            let a = s
                .execution
                .get_mut(&rider)
                .ok_or(CommitError::InvalidTransition)?;
            if a.stage != ExecutionStage::Arrived || !matches!(a.stop, Stop::Pickup(_)) {
                return Err(CommitError::InvalidTransition);
            }
            a.stage = ExecutionStage::Waiting;
            Ok(())
        })
    }
    /// Projects the authoritative active stop, including pinned service progress.
    /// Forecast data never becomes an observed pickup. Missing required readiness is
    /// a typed evaluation failure; the active obligation remains intact.
    ///
    /// # Errors
    /// Rejects missing/stale readiness, unsupported policy or an already-expired prefix.
    pub fn frozen_prefix(
        &self,
        rider: RiderId,
        inputs: &crate::PoolingInputs,
        at: DispatchInstant,
    ) -> Result<Option<crate::FrozenPrefix>, crate::PoolingError> {
        let Some(active) = self.execution.get(&rider) else {
            return Ok(None);
        };
        let policy = self
            .data()
            .accepted
            .get(&active.stop.order())
            .map(|t| &t.policy)
            .or_else(|| inputs.policies.get(&active.stop.order()))
            .ok_or(crate::PoolingError::UnsupportedPolicy)?;
        let (wait, service) = match active.stop {
            Stop::Pickup(order) => {
                let (ready, _) = crate::effective_readiness(self.data(), inputs, order, at)?;
                (
                    if ready > active.arrival {
                        ready.duration_since(active.arrival)?
                    } else {
                        Seconds::ZERO
                    },
                    policy.pickup_service,
                )
            }
            Stop::Dropoff(_) => (Seconds::ZERO, policy.dropoff_service),
        };
        let mut timeline = crate::StopTimeline::new(active.stop, active.arrival, wait, service)?;
        if let Some(end) = active.service_end {
            timeline = crate::StopTimeline::new(
                active.stop,
                active.arrival,
                active
                    .service_wait
                    .ok_or(crate::PoolingError::InvalidContext)?,
                service,
            )?;
            if timeline.departure != end {
                return Err(crate::PoolingError::InvalidContext);
            }
        }
        if timeline.departure < at {
            return Err(crate::PoolingError::PredictionUnavailable(
                active.stop.order(),
            ));
        }
        Ok(Some(crate::FrozenPrefix {
            execution_id: active.action_id.value(),
            timeline,
            anchor: active.anchor.clone(),
        }))
    }

    pub(crate) fn validate_context_authority(
        &self,
        context: &PoolingContext,
    ) -> Result<(), crate::PoolingError> {
        for (r, a) in &self.execution {
            let frozen = context
                .inputs
                .projections
                .get(r)
                .and_then(|p| p.frozen.as_ref())
                .ok_or(crate::PoolingError::InvalidContext)?;
            if frozen.execution_id != a.action_id.value()
                || frozen.anchor != a.anchor
                || frozen.timeline.stop != a.stop
                || self
                    .frozen_prefix(*r, &context.inputs, context.at)?
                    .as_ref()
                    != Some(frozen)
            {
                return Err(crate::PoolingError::InvalidContext);
            }
        }
        for (r, p) in &context.inputs.projections {
            if p.frozen.is_some() && !self.execution.contains_key(r) {
                return Err(crate::PoolingError::InvalidContext);
            }
        }
        Ok(())
    }
    // Explicit trusted compatibility adapters. They preserve legacy simulation planning
    // semantics; operational proposal publication is provided by the temporal boundary.
    /// Registers an explicitly identified trusted fixture order.
    ///
    /// # Errors
    /// Rejects invalid domain replacement.
    pub fn register_order(
        &mut self,
        order: Order,
        readiness: OrderReadiness,
    ) -> Result<(), CommitError> {
        if !self.compatibility {
            return Err(CommitError::InvalidTransition);
        }
        self.transaction(self.revision, |s| s.world.register_order(order, readiness))
    }
    /// Adopts an authoritative readiness observation once.
    ///
    /// # Errors
    /// Rejects conflicting repeated observation; identical redelivery is a no-effect.
    pub fn observe_ready(
        &mut self,
        order: OrderId,
        at: DispatchInstant,
    ) -> Result<(), CommitError> {
        if self
            .world
            .data()
            .readiness
            .get(&order)
            .and_then(|r| r.observed_at)
            == Some(at)
        {
            return Ok(());
        }
        self.transaction(self.revision, |s| s.world.observe_ready(order, at))
    }
    /// Updates readiness prediction under trusted legacy fixture semantics.
    ///
    /// # Errors
    /// Rejects invalid domain input.
    pub fn estimate_readiness(
        &mut self,
        order: OrderId,
        at: DispatchInstant,
    ) -> Result<(), CommitError> {
        self.transaction(self.revision, |s| s.world.estimate_readiness(order, at))
    }
    /// Updates authoritative rider facts; cannot move the endpoint of active travel.
    ///
    /// # Errors
    /// Rejects active-location replacement or invalid state.
    pub fn update_rider_state(
        &mut self,
        rider: RiderId,
        state: RiderState,
    ) -> Result<(), CommitError> {
        self.transaction(self.revision, |s| {
            if s.execution.contains_key(&rider)
                && s.world.data().riders[&rider].coordinate != state.coordinate
            {
                return Err(CommitError::InvalidTransition);
            }
            s.world.update_rider_state(rider, state)
        })
    }
    /// Trusted Phase 13–16 simulation publication, not operational temporal admission.
    ///
    /// # Errors
    /// Rejects stale/invalid proposal or frozen inconsistency.
    pub fn commit(&mut self, decision: &AssignmentDecision) -> Result<(), CommitError> {
        if !self.compatibility {
            return Err(CommitError::InvalidTransition);
        }
        self.transaction(self.revision, |s| s.world.commit(decision))
    }
    /// Trusted Phase 17 simulation publication at its controlled logical instant.
    ///
    /// # Errors
    /// Rejects invalid/stale context or frozen replacement.
    pub(crate) fn certified_commit_insertion(
        &mut self,
        decision: &InsertionDecision,
        current: &PoolingContext,
    ) -> Result<(), CommitError> {
        self.transaction(self.revision, |s| {
            s.world.commit_insertion(decision, current)
        })
    }
    /// Trusted simulator-only publication; operational authorities require a sealed temporal certificate.
    ///
    /// # Errors
    /// Rejects operational use, stale context, or invalid frozen/domain state.
    pub fn commit_insertion(
        &mut self,
        decision: &InsertionDecision,
        current: &PoolingContext,
    ) -> Result<(), CommitError> {
        if !self.compatibility {
            return Err(CommitError::InvalidTransition);
        }
        self.certified_commit_insertion(decision, current)
    }
    /// Trusted Phase 18 simulation publication at its controlled logical instant.
    ///
    /// # Errors
    /// Rejects invalid/stale context or frozen replacement.
    pub(crate) fn certified_commit_fleet(
        &mut self,
        decision: &FleetDecision,
        current: &FleetContext,
    ) -> Result<(), CommitError> {
        self.transaction(self.revision, |s| s.world.commit_fleet(decision, current))
    }
    /// Trusted simulator-only publication; operational authorities require a sealed temporal certificate.
    ///
    /// # Errors
    /// Rejects operational use, stale context, or invalid frozen/domain state.
    pub fn commit_fleet(
        &mut self,
        decision: &FleetDecision,
        current: &FleetContext,
    ) -> Result<(), CommitError> {
        if !self.compatibility {
            return Err(CommitError::InvalidTransition);
        }
        self.certified_commit_fleet(decision, current)
    }
    /// Trusted Phase 19 simulation publication, including cooldown atomically.
    ///
    /// # Errors
    /// Rejects stale/invalid proposal or frozen ownership replacement.
    pub(crate) fn certified_commit_recovery(
        &mut self,
        decision: &RecoveryDecision,
        current: &RecoveryContext,
    ) -> Result<(), CommitError> {
        self.transaction(self.revision, |s| {
            s.world.commit_recovery(decision, current)?;
            s.recovery_at = Some(current.pooling.at);
            Ok(())
        })
    }
    pub(crate) fn stamp_recovery_commit(&mut self, at: DispatchInstant) {
        self.recovery_at = Some(at);
        if let Some(record) = self.history.last_mut() {
            record.recovery_after = Some(at);
        }
    }
    /// Trusted simulator-only publication; operational authorities require a sealed temporal certificate.
    ///
    /// # Errors
    /// Rejects operational use, stale context, or invalid frozen/domain state.
    pub fn commit_recovery(
        &mut self,
        decision: &RecoveryDecision,
        current: &RecoveryContext,
    ) -> Result<(), CommitError> {
        if !self.compatibility {
            return Err(CommitError::InvalidTransition);
        }
        self.certified_commit_recovery(decision, current)
    }
    /// Cancellation derives frozen protection from authoritative execution, not caller sets.
    ///
    /// # Errors
    /// Rejects frozen/custody work or invalid cancellation instant.
    pub fn cancel_order(
        &mut self,
        order: OrderId,
        at: DispatchInstant,
    ) -> Result<Result<(), crate::CancellationRefusal>, CommitError> {
        let active = self.execution.values().map(|a| a.stop.order()).collect();
        let mut next = self.staged_copy();
        if let Err(reason) = next.world.cancel_order(order, at, &active)? {
            Ok(Err(reason))
        } else {
            self.transaction(self.revision, |s| {
                s.world = next.world;
                Ok(())
            })?;
            Ok(Ok(()))
        }
    }
}

/// Caller-supplied authoritative monotonic time source; domain code never reads host time.
pub trait OperationalClock {
    /// Named time domain and origin/conversion semantics.
    fn time_domain(&self) -> &str;
    /// Authoritative current publication instant.
    ///
    /// # Errors
    /// Returns invalid/unavailable time rather than substituting a host clock.
    fn now(&self) -> Result<DispatchInstant, DispatchTimeError>;
}
