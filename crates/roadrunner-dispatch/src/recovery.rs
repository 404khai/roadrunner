//! Phase 19 bounded deterministic recovery of existing commitments.
use std::collections::BTreeMap;

use roadrunner_core::geo::Seconds;
use serde::{Deserialize, Serialize};

use crate::{
    Availability, CommitError, CommittedAssignment, CompletionImpact, DispatchInstant,
    DispatchSnapshot, FulfillmentState, PoolingContext, PoolingError, PoolingInputs, RiderId,
    RiderPlan, Stop, WholePlanEvaluation, World, WorldData, WorldVersion, evaluate_recovery_plan,
    validate_world,
};

/// Explicit churn policy; no universal numeric defaults.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryPolicy {
    /// Supported version is 1.
    pub version: u32,
    /// Seconds charged per changed rider plan relative to original baseline.
    pub reroute_penalty: Seconds,
    /// Seconds charged per changed existing order owner.
    pub assignment_stability_penalty: Seconds,
    /// Minimum positive penalized saving for optional changes.
    pub minimum_improvement: Seconds,
    /// Minimum time between optional successful recoveries.
    pub cooldown: Seconds,
}

/// Exact pinned policy/execution/trigger context.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecoveryContext {
    /// Whole physical prediction context.
    pub pooling: PoolingContext,
    /// Explicit churn policy.
    pub policy: RecoveryPolicy,
    /// Caller-supplied semantic trigger identity.
    pub trigger: String,
    /// Last successful recovery instant, never a host clock.
    pub last_applied: Option<DispatchInstant>,
}
impl RecoveryContext {
    /// Bind a coherent recovery context.
    ///
    /// # Errors
    /// Rejects unknown policy versions, empty triggers and future history.
    pub fn new(
        snapshot: &DispatchSnapshot<'_>,
        inputs: PoolingInputs,
        policy: RecoveryPolicy,
        trigger: String,
        last_applied: Option<DispatchInstant>,
    ) -> Result<Self, PoolingError> {
        if policy.version != 1
            || trigger.is_empty()
            || last_applied.is_some_and(|t| t > snapshot.at)
        {
            return Err(PoolingError::UnsupportedPolicy);
        }
        Ok(Self {
            pooling: PoolingContext::bind(snapshot, inputs, "dynamic-recovery/v1")?,
            policy,
            trigger,
            last_applied,
        })
    }
}

/// Scoped termination; no recovery is not a proof of arbitrary-resequencing infeasibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RecoveryTermination {
    /// Complete declared neighborhoods reached a strict local minimum.
    LocalMinimum,
    /// Baseline already healthy but cooldown suppresses optional churn.
    Cooldown,
    /// No complete single neighborhood move repairs the unhealthy baseline.
    NoRecovery,
    /// Budget ended; no proposal may publish.
    SearchIncomplete,
}

/// Chosen candidate, available for review but constructible only by the search.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecoveryProposal {
    assignments: BTreeMap<crate::OrderId, CommittedAssignment>,
    plans: BTreeMap<RiderId, RiderPlan>,
    evaluations: BTreeMap<RiderId, WholePlanEvaluation>,
    /// Road-travel seconds over projected editable horizons.
    pub travel_seconds: f64,
    /// Road distance over the same horizons.
    pub distance_meters: f64,
    /// Penalty seconds relative to original owners/plans.
    pub penalty_seconds: f64,
    /// Per-order raw marginal and original cumulative impacts.
    pub impacts: Vec<CompletionImpact>,
}
impl RecoveryProposal {
    /// Complete resulting owners.
    #[must_use]
    pub const fn assignments(&self) -> &BTreeMap<crate::OrderId, CommittedAssignment> {
        &self.assignments
    }
    /// Complete resulting plans.
    #[must_use]
    pub const fn plans(&self) -> &BTreeMap<RiderId, RiderPlan> {
        &self.plans
    }
}

/// Versioned semantic evidence and private publication guard.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecoveryDecision {
    /// Evidence contract version.
    pub schema_version: u32,
    /// Exact coherent source identity.
    pub world_identity: u64,
    /// Exact coherent source version.
    pub world_version: WorldVersion,
    /// Complete policy/forecast/traffic/clock/execution context.
    pub context: RecoveryContext,
    /// Existing hard breach or unavailable movable owner requires repair.
    pub baseline_requires_repair: bool,
    /// Baseline physical evaluations, including explicit hard breaches.
    pub baseline: BTreeMap<RiderId, WholePlanEvaluation>,
    /// Complete candidate submissions evaluated.
    pub evaluated: u64,
    /// Fully feasible submissions.
    pub feasible: u64,
    /// Typed/reviewable rejection counts.
    pub rejections: BTreeMap<String, u64>,
    /// Canonical covered riders; no spatial shortlist.
    pub riders: Vec<RiderId>,
    /// Movable original committed orders.
    pub movable_orders: Vec<crate::OrderId>,
    /// Original custody/frozen orders explicitly excluded from ownership moves.
    pub locked_orders: Vec<crate::OrderId>,
    /// Proven precedence-invalid single-stop relocations pruned before evaluation.
    pub precedence_exclusions: u64,
    /// Complete best-improvement rounds.
    pub rounds: u64,
    /// Declared neighborhood termination.
    pub termination: RecoveryTermination,
    expected: WorldData,
    proposal: Option<RecoveryProposal>,
}
impl RecoveryDecision {
    /// Only a complete successful decision offers a publication proposal.
    #[must_use]
    pub const fn proposal(&self) -> Option<&RecoveryProposal> {
        self.proposal.as_ref()
    }
}

fn locked(data: &WorldData, ctx: &RecoveryContext, order: crate::OrderId) -> bool {
    data.fulfillment[&order] != FulfillmentState::AwaitingPickup
        || ctx.pooling.inputs.projections.values().any(|p| {
            p.frozen
                .as_ref()
                .is_some_and(|f| f.timeline.stop.order() == order)
        })
}

fn evaluate(
    snapshot: &DispatchSnapshot<'_>,
    ctx: &RecoveryContext,
    plans: BTreeMap<RiderId, RiderPlan>,
    assignments: BTreeMap<crate::OrderId, CommittedAssignment>,
) -> Result<(RecoveryProposal, Vec<String>), PoolingError> {
    let data = snapshot.world.data();
    let mut evaluations = BTreeMap::new();
    let mut rejections = Vec::new();
    for (order, owner) in &assignments {
        if data.riders[&owner.rider].availability != Availability::Available
            && !locked(data, ctx, *order)
        {
            rejections.push("UnavailableOwner".into());
        }
    }
    for (rider, plan) in &plans {
        match evaluate_recovery_plan(snapshot, &ctx.pooling.inputs, *rider, plan, &assignments)? {
            Ok(e) => {
                rejections.extend(e.breaches.iter().map(|r| format!("{r:?}")));
                evaluations.insert(*rider, e);
            }
            Err(reason) => rejections.push(format!("{reason:?}")),
        }
    }
    let travel_seconds = evaluations.values().map(|e| e.travel.value()).sum();
    let distance_meters = evaluations.values().map(|e| e.distance.value()).sum();
    let plan_penalty: f64 = plans
        .iter()
        .filter(|(r, p)| data.plans[*r] != **p)
        .map(|_| ctx.policy.reroute_penalty.value())
        .sum();
    let owner_penalty: f64 = assignments
        .iter()
        .filter(|(o, a)| data.assignments[*o] != **a)
        .map(|_| ctx.policy.assignment_stability_penalty.value())
        .sum();
    let penalty_seconds = plan_penalty + owner_penalty;
    for value in [
        travel_seconds,
        distance_meters,
        penalty_seconds,
        travel_seconds + penalty_seconds,
    ] {
        Seconds::new(value).map_err(|_| crate::DispatchEvaluationError::InvalidMetric)?;
    }
    Ok((
        RecoveryProposal {
            assignments,
            plans,
            evaluations,
            travel_seconds,
            distance_meters,
            penalty_seconds,
            impacts: Vec::new(),
        },
        rejections,
    ))
}
fn identity(p: &RecoveryProposal) -> Vec<(RiderId, Vec<(u64, bool)>)> {
    p.plans
        .iter()
        .map(|(r, p)| {
            (
                *r,
                p.stops
                    .iter()
                    .map(|s| (s.order().value(), matches!(s, Stop::Dropoff(_))))
                    .collect(),
            )
        })
        .collect()
}
fn better(a: &RecoveryProposal, b: &RecoveryProposal) -> bool {
    (a.travel_seconds + a.penalty_seconds)
        .total_cmp(&(b.travel_seconds + b.penalty_seconds))
        .then_with(|| a.distance_meters.total_cmp(&b.distance_meters))
        .then_with(|| identity(a).cmp(&identity(b)))
        .is_lt()
}

/// Recover committed work using complete canonical best-improvement neighborhoods.
/// Required inputs must cover every rider; unknown health is a typed failure.
///
/// # Errors
/// Returns structural/input/routing errors without modifying the world.
#[allow(clippy::too_many_lines)]
pub fn recover_fleet(
    snapshot: &DispatchSnapshot<'_>,
    context: RecoveryContext,
) -> Result<RecoveryDecision, PoolingError> {
    validate_world(snapshot.world.data())?;
    let data = snapshot.world.data();
    if !data
        .profiles
        .keys()
        .eq(context.pooling.inputs.projections.keys())
    {
        return Err(PoolingError::InvalidContext);
    }
    let rebound = RecoveryContext::new(
        snapshot,
        context.pooling.inputs.clone(),
        context.policy,
        context.trigger.clone(),
        context.last_applied,
    )?;
    if rebound != context {
        return Err(PoolingError::InvalidContext);
    }
    let (baseline, reasons) = evaluate(
        snapshot,
        &context,
        data.plans.clone(),
        data.assignments.clone(),
    )?;
    if let Some(rider) = data
        .profiles
        .keys()
        .find(|r| !baseline.evaluations.contains_key(r))
    {
        return Err(PoolingError::BaselineUnavailable(*rider));
    }
    let repair = !reasons.is_empty();
    let mut decision = RecoveryDecision {
        schema_version: 1,
        world_identity: snapshot.world.identity(),
        world_version: snapshot.world.version(),
        baseline_requires_repair: repair,
        baseline: baseline.evaluations.clone(),
        evaluated: 0,
        feasible: 0,
        rejections: BTreeMap::new(),
        riders: data.profiles.keys().copied().collect(),
        movable_orders: data
            .assignments
            .keys()
            .copied()
            .filter(|o| !locked(data, &context, *o))
            .collect(),
        locked_orders: data
            .assignments
            .keys()
            .copied()
            .filter(|o| locked(data, &context, *o))
            .collect(),
        precedence_exclusions: 0,
        rounds: 0,
        termination: RecoveryTermination::LocalMinimum,
        expected: data.clone(),
        proposal: None,
        context,
    };
    if !repair
        && decision.context.last_applied.is_some_and(|t| {
            snapshot.at.value() - t.value() < decision.context.policy.cooldown.value()
        })
    {
        decision.termination = RecoveryTermination::Cooldown;
        return Ok(decision);
    }
    let mut incumbent = baseline;
    let mut unhealthy = repair;
    loop {
        let mut best: Option<RecoveryProposal> = None;
        // Enumerate semantic moves, never spatially shortlist recipients.
        let mut candidates = Vec::new();
        for (order, owner) in &incumbent.assignments {
            if locked(data, &decision.context, *order) {
                continue;
            }
            let mut base = incumbent.plans.clone();
            base.get_mut(&owner.rider)
                .ok_or(PoolingError::InvalidContext)?
                .stops
                .retain(|s| s.order() != *order);
            for target in data
                .profiles
                .keys()
                .copied()
                .filter(|r| data.riders[r].availability == Availability::Available)
            {
                let prefix = usize::from(
                    decision.context.pooling.inputs.projections[&target]
                        .frozen
                        .is_some(),
                );
                let n = base[&target].stops.len();
                for pickup in prefix..=n {
                    for dropoff in pickup + 1..=n + 1 {
                        let mut plans = base.clone();
                        let p = &mut plans
                            .get_mut(&target)
                            .ok_or(PoolingError::InvalidContext)?
                            .stops;
                        p.insert(pickup, Stop::Pickup(*order));
                        p.insert(dropoff, Stop::Dropoff(*order));
                        if plans == incumbent.plans {
                            continue;
                        }
                        let mut assignments = incumbent.assignments.clone();
                        assignments.insert(
                            *order,
                            CommittedAssignment {
                                order: *order,
                                rider: target,
                            },
                        );
                        candidates.push((plans, assignments));
                    }
                }
            }
        }
        for (rider, plan) in &incumbent.plans {
            let prefix = usize::from(
                decision.context.pooling.inputs.projections[rider]
                    .frozen
                    .is_some(),
            );
            for from in prefix..plan.stops.len() {
                for to in prefix..plan.stops.len() {
                    if from == to {
                        continue;
                    }
                    let mut plans = incumbent.plans.clone();
                    let p = &mut plans
                        .get_mut(rider)
                        .ok_or(PoolingError::InvalidContext)?
                        .stops;
                    let stop = p.remove(from);
                    p.insert(to, stop);
                    // Prove pickup precedence before submitting; full load remains evaluator-owned.
                    let valid = plans[rider].stops.iter().enumerate().all(|(i, s)| match s {
                        Stop::Dropoff(o)
                            if data.fulfillment[o] == FulfillmentState::AwaitingPickup =>
                        {
                            plans[rider].stops[..i].contains(&Stop::Pickup(*o))
                        }
                        _ => true,
                    });
                    if valid {
                        candidates.push((plans, incumbent.assignments.clone()));
                    } else {
                        decision.precedence_exclusions += 1;
                    }
                }
            }
        }
        for (plans, assignments) in candidates {
            if decision.evaluated == decision.context.pooling.inputs.work_budget {
                decision.termination = RecoveryTermination::SearchIncomplete;
                return Ok(decision);
            }
            decision.evaluated += 1;
            let (candidate, reasons) = evaluate(snapshot, &decision.context, plans, assignments)?;
            if !reasons.is_empty() {
                for reason in reasons {
                    *decision.rejections.entry(reason).or_default() += 1;
                }
                continue;
            }
            decision.feasible += 1;
            if (unhealthy || better(&candidate, &incumbent))
                && best.as_ref().is_none_or(|b| better(&candidate, b))
            {
                best = Some(candidate);
            }
        }
        decision.rounds += 1;
        let Some(best) = best else {
            if unhealthy {
                decision.termination = RecoveryTermination::NoRecovery;
            }
            break;
        };
        incumbent = best;
        unhealthy = false;
    }
    if !unhealthy && incumbent.plans != data.plans {
        let baseline_travel: f64 = decision.baseline.values().map(|e| e.travel.value()).sum();
        let saving = baseline_travel - incumbent.travel_seconds - incumbent.penalty_seconds;
        if repair || (saving > 0.0 && saving >= decision.context.policy.minimum_improvement.value())
        {
            let baseline_all = WholePlanEvaluation {
                plan: RiderPlan::default(),
                travel: Seconds::ZERO,
                distance: roadrunner_core::geo::Meters::ZERO,
                stops: Vec::new(),
                completions: decision
                    .baseline
                    .values()
                    .flat_map(|e| e.completions.clone())
                    .collect(),
                breaches: Vec::new(),
            };
            for evaluation in incumbent.evaluations.values() {
                incumbent.impacts.extend(crate::pooling::impact(
                    data,
                    &baseline_all,
                    evaluation,
                    &decision.context.pooling.inputs,
                ));
            }
            decision.proposal = Some(incumbent);
        }
    }
    Ok(decision)
}
impl World {
    /// Atomically replace committed owners/plans, retaining every acceptance reference.
    ///
    /// # Errors
    /// Any relevant mismatch is stale with zero mutation; incomplete decisions cannot commit.
    pub fn commit_recovery(
        &mut self,
        decision: &RecoveryDecision,
        current: &RecoveryContext,
    ) -> Result<(), CommitError> {
        if self.identity() != decision.world_identity
            || self.version() != decision.world_version
            || *current != decision.context
            || *self.data() != decision.expected
        {
            return Err(CommitError::Stale);
        }
        let p = decision
            .proposal
            .as_ref()
            .ok_or(CommitError::InvalidTransition)?;
        if decision.termination != RecoveryTermination::LocalMinimum {
            return Err(CommitError::InvalidTransition);
        }
        let mut next = self.data().clone();
        for (o, a) in &p.assignments {
            if next.assignments[o] != *a && locked(self.data(), current, *o) {
                return Err(CommitError::InvalidTransition);
            }
        }
        next.assignments = p.assignments.clone();
        next.plans = p.plans.clone();
        self.publish(next)
    }
}

/// Why cancellation cannot remove a mandatory active obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CancellationRefusal {
    /// Already picked up, completed, cancelled or unknown.
    NotAwaitingPickup,
    /// Active departed leg/destination wait/service must finish.
    FrozenExecution,
    /// Cancellation precedes creation.
    InvalidTime,
}
impl World {
    /// Cancel unstarted work, retaining historical accepted terms.
    /// Active execution identities are explicit caller-owned domain inputs.
    ///
    /// # Errors
    /// Failed whole-world validation or version overflow leaves all state unchanged.
    pub fn cancel_order(
        &mut self,
        order: crate::OrderId,
        at: DispatchInstant,
        active_orders: &std::collections::BTreeSet<crate::OrderId>,
    ) -> Result<Result<(), CancellationRefusal>, CommitError> {
        if self.data().fulfillment.get(&order) != Some(&FulfillmentState::AwaitingPickup) {
            return Ok(Err(CancellationRefusal::NotAwaitingPickup));
        }
        if active_orders.contains(&order) {
            return Ok(Err(CancellationRefusal::FrozenExecution));
        }
        if at < self.data().orders[&order].created_at {
            return Ok(Err(CancellationRefusal::InvalidTime));
        }
        let mut next = self.data().clone();
        next.assignments.remove(&order);
        for plan in next.plans.values_mut() {
            plan.stops.retain(|s| s.order() != order);
        }
        next.fulfillment
            .insert(order, FulfillmentState::Cancelled { at });
        self.publish(next)?;
        Ok(Ok(()))
    }
}
