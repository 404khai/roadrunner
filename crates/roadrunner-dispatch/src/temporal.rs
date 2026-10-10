//! Static-road monotone temporal applicability, never a universal TTL.
//!
//! Static road costs preserve every insertion's road objective and identity. Fixed
//! readiness/service schedules make completion nondecreasing as idle departure is
//! delayed. Structural/load/custody constraints are unchanged. A candidate feasible
//! at the certified upper endpoint is feasible throughout the interval; previously
//! infeasible candidates cannot become feasible. Thus the original complete-search
//! winner remains best whenever its endpoint baseline and candidate are healthy.
//! Frozen prefix projections must remain in the future throughout the interval.
use std::collections::BTreeMap;

use serde::Serialize;
use thiserror::Error;

use crate::{
    AcceptedTerms, AdoptedContext, CommitError, CoreRouteProvider, DispatchInstant,
    DispatchSnapshot, FleetDecision, InsertionDecision, OperationalClock, OperationalRevision,
    OperationalState, PoolingContext, PoolingError, PoolingInputs, RecoveryDecision, RouteProvider,
    WholePlanEvaluation, evaluate_batch_plan, evaluate_recovery_plan, evaluate_whole_plan,
};

/// Policy semantics, not a duration default.
pub const STATIC_MONOTONE_POLICY: &str = "static-road-monotone/v1";

/// Typed applicability rejection or unavailable prediction, not infeasibility scoring.
#[derive(Debug, Error)]
pub enum TemporalError {
    /// Operational state/context adoption changed after evaluation/certification.
    #[error("stale operational evaluation")]
    Stale,
    /// Interval cannot preserve required feasibility or selection assumptions.
    #[error("evaluation context expired or unproven")]
    EvaluationContextExpired,
    /// Time-dependent/custom routing has no v1 interval proof.
    #[error("unsupported temporal routing semantics")]
    UnsupportedTemporalModel,
    /// Required inputs or route evaluation unavailable.
    #[error(transparent)]
    Prediction(#[from] PoolingError),
    /// Domain publication rejects exact proposal.
    #[error(transparent)]
    Commit(#[from] CommitError),
}

/// Independently versioned provenance envelope, not internal struct persistence.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PublicationProvenance {
    /// Record schema (independent of policy/algorithm semantics).
    pub schema_version: u32,
    /// Operational context from evaluation.
    pub source: OperationalRevision,
    /// Evaluation instant, never relabeled as commit time.
    pub evaluated_at: DispatchInstant,
    /// Publication validation instant.
    pub publication_at: DispatchInstant,
    /// Authoritative clock sample at linearization.
    pub committed_at: DispatchInstant,
    /// Resulting coherent operational revision.
    pub committed_revision: OperationalRevision,
    /// Named time-domain/origin contract.
    pub time_domain: String,
    /// Versioned applicability proof semantics.
    pub validity_policy: String,
    /// Explicit certified upper endpoint; not a universal TTL.
    pub valid_until: DispatchInstant,
    /// Exact adopted contexts and ABA-protecting revisions.
    pub contexts: BTreeMap<String, AdoptedContext>,
    /// Independent immutable prediction/service/optimizer/routing provenance.
    pub prediction: crate::PredictionIdentity,
    /// Exact successful original decision evidence, not recomputed history.
    pub decision_record: crate::HistoricalRecord<serde_json::Value>,
    /// Actual committed facts, including authentically stamped new terms and preserved execution.
    pub commitment_record: crate::HistoricalRecord<serde_json::Value>,
    /// Completion predictions at upper endpoint, separate from accepted references.
    pub endpoint_completions: BTreeMap<crate::OrderId, DispatchInstant>,
}

#[derive(Debug, Clone)]
enum Decision {
    Insertion(InsertionDecision),
    Fleet(FleetDecision),
    Recovery(RecoveryDecision),
}
/// Server-assembled planning dependencies. Clock/time conversion and artifact
/// ownership belong to the trusted adapter, never arbitrary client JSON.
pub struct OperationalPlanningContext<'a> {
    /// Pinned core provider.
    pub provider: &'a CoreRouteProvider<'a>,
    /// Validated snapshot-qualified anchors.
    pub anchors: &'a crate::RoutingAnchors,
    /// Explicit dispatch/routing origin mapping.
    pub epoch: crate::RoutingEpoch,
    /// Authoritative clock supplied by the adapter.
    pub clock: &'a dyn OperationalClock,
}

/// Immutable evaluation bound to the exact operational revision and adoptions.
#[derive(Debug, Clone)]
pub struct EvaluatedOperationalDecision {
    source: OperationalRevision,
    contexts: BTreeMap<String, AdoptedContext>,
    time_domain: String,
    decision: Decision,
}
impl EvaluatedOperationalDecision {
    /// Original insertion evidence, if this was an admission.
    #[must_use]
    pub fn insertion(&self) -> Option<&InsertionDecision> {
        if let Decision::Insertion(d) = &self.decision {
            Some(d)
        } else {
            None
        }
    }
    /// Original fleet evidence.
    #[must_use]
    pub fn fleet(&self) -> Option<&FleetDecision> {
        if let Decision::Fleet(d) = &self.decision {
            Some(d)
        } else {
            None
        }
    }
    /// Original recovery evidence.
    #[must_use]
    pub fn recovery(&self) -> Option<&RecoveryDecision> {
        if let Decision::Recovery(d) = &self.decision {
            Some(d)
        } else {
            None
        }
    }
}

pub(crate) fn fingerprint<T: Serialize>(value: &T) -> Result<String, TemporalError> {
    use sha2::{Digest, Sha256};
    let bytes = serde_json::to_vec(value).map_err(|_| TemporalError::EvaluationContextExpired)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn context_content(context: &PoolingContext) -> Result<BTreeMap<String, String>, TemporalError> {
    Ok(BTreeMap::from([
        (
            "graph".into(),
            context.inputs.identity.routing.graph_digest.clone(),
        ),
        (
            "traffic".into(),
            fingerprint(&context.inputs.identity.routing.traffic)?,
        ),
        ("readiness".into(), fingerprint(&context.inputs.forecasts)?),
        (
            "profile".into(),
            fingerprint(&context.inputs.identity.routing.profile)?,
        ),
        (
            "policies".into(),
            fingerprint(&(
                &context.inputs.policies,
                &context.inputs.identity.prediction,
                &context.inputs.identity.service,
                &context.inputs.identity.optimizer,
            ))?,
        ),
    ]))
}

/// Sealed certificate: only authoritative evaluation plus endpoint validation can
/// create it. No client-supplied interval can establish applicability by assertion.
#[derive(Debug, Clone)]
pub struct TemporalProposal {
    source: OperationalRevision,
    contexts: BTreeMap<String, AdoptedContext>,
    time_domain: String,
    evaluated_at: DispatchInstant,
    valid_until: DispatchInstant,
    decision: Decision,
    endpoint_completions: BTreeMap<crate::OrderId, DispatchInstant>,
}

fn endpoint_inputs(
    context: &PoolingContext,
    until: DispatchInstant,
    data: &crate::WorldData,
) -> Result<PoolingInputs, TemporalError> {
    if until < context.at {
        return Err(TemporalError::EvaluationContextExpired);
    }
    let mut inputs = context.inputs.clone();
    for (order, forecast) in &inputs.forecasts {
        if data
            .readiness
            .get(order)
            .is_some_and(|r| r.observed_at.is_some())
            || inputs
                .policies
                .get(order)
                .is_some_and(|p| p.readiness == crate::ReadinessRule::LegacyV1)
        {
            continue;
        }
        if forecast.generated_at > context.at || forecast.valid_until < until {
            return Err(TemporalError::Prediction(
                PoolingError::PredictionUnavailable(*order),
            ));
        }
    }
    for projection in inputs.projections.values_mut() {
        if projection.frozen.is_some() {
            if projection.at < until {
                return Err(TemporalError::EvaluationContextExpired);
            }
        } else {
            projection.at = until;
        }
    }
    Ok(inputs)
}
fn feasible(
    result: Result<WholePlanEvaluation, crate::InsertionRejection>,
) -> Result<WholePlanEvaluation, TemporalError> {
    let evaluation = result.map_err(|_| TemporalError::EvaluationContextExpired)?;
    if !evaluation.breaches.is_empty() {
        return Err(TemporalError::EvaluationContextExpired);
    }
    Ok(evaluation)
}
fn new_terms_valid(terms: &AcceptedTerms, completion: DispatchInstant) -> bool {
    let delay = (completion.value() - terms.completion_reference.value()).max(0.0);
    terms
        .policy
        .max_completion_delay
        .is_none_or(|bound| delay <= bound.value())
}
impl OperationalState {
    /// Atomically adopts validated planning content identities; projection state is
    /// separately governed by operational revision and execution authority.
    ///
    /// # Errors
    /// Rejects stale authority or invalid canonical/context data.
    pub fn adopt_planning_context(
        &mut self,
        expected: OperationalRevision,
        context: &PoolingContext,
    ) -> Result<(), TemporalError> {
        self.validate_context_authority(context)?;
        let identities = context_content(context)?;
        self.adopt_batch(expected, identities)?;
        Ok(())
    }
    fn check_adoptions(&self, context: &PoolingContext) -> Result<(), TemporalError> {
        for (category, content) in context_content(context)? {
            if self
                .contexts()
                .get(&category)
                .is_none_or(|c| c.identity.content != content)
            {
                return Err(TemporalError::Stale);
            }
        }
        self.validate_context_authority(context)?;
        Ok(())
    }
    fn bound(
        &self,
        decision: Decision,
        context: &PoolingContext,
        domain: &str,
    ) -> Result<EvaluatedOperationalDecision, TemporalError> {
        self.check_adoptions(context)?;
        if domain.is_empty() {
            return Err(TemporalError::EvaluationContextExpired);
        }
        Ok(EvaluatedOperationalDecision {
            source: self.revision(),
            contexts: self.contexts().clone(),
            time_domain: domain.into(),
            decision,
        })
    }
    /// Evaluates one admission against server-owned operational state and records its
    /// exact operational revision/adoption dependencies before releasing evaluation.
    ///
    /// # Errors
    /// Preserves typed input/evaluation failures; no assignment or terms are established.
    pub fn evaluate_insertion(
        &self,
        context: &OperationalPlanningContext<'_>,
        order: crate::OrderId,
        inputs: PoolingInputs,
    ) -> Result<EvaluatedOperationalDecision, TemporalError> {
        let at = context
            .clock
            .now()
            .map_err(|_| TemporalError::EvaluationContextExpired)?;
        let snapshot =
            DispatchSnapshot::new(self, at, context.epoch, context.provider, context.anchors)
                .map_err(PoolingError::from)?;
        let decision = crate::insert_order(&snapshot, order, inputs)?;
        let bound_context = decision.evidence().context.clone();
        self.bound(
            Decision::Insertion(decision),
            &bound_context,
            context.clock.time_domain(),
        )
    }
    /// Evaluates a Phase 18 batch with fixed committed ownership.
    ///
    /// # Errors
    /// Preserves typed prediction/context errors and does not publish incomplete work.
    pub fn evaluate_fleet(
        &self,
        context: &OperationalPlanningContext<'_>,
        batch: &[crate::OrderId],
        inputs: crate::FleetInputs,
    ) -> Result<EvaluatedOperationalDecision, TemporalError> {
        let at = context
            .clock
            .now()
            .map_err(|_| TemporalError::EvaluationContextExpired)?;
        let snapshot =
            DispatchSnapshot::new(self, at, context.epoch, context.provider, context.anchors)
                .map_err(PoolingError::from)?;
        let decision = crate::optimize_fleet(&snapshot, batch, inputs)?;
        let bound_context = decision.evidence().context.pooling.clone();
        self.bound(
            Decision::Fleet(decision),
            &bound_context,
            context.clock.time_domain(),
        )
    }
    /// Evaluates recovery with authoritative cooldown, never caller-asserted history.
    ///
    /// # Errors
    /// Preserves typed policy/input/baseline failures.
    pub fn evaluate_recovery(
        &self,
        context: &OperationalPlanningContext<'_>,
        inputs: PoolingInputs,
        policy: crate::RecoveryPolicy,
        trigger: String,
    ) -> Result<EvaluatedOperationalDecision, TemporalError> {
        let at = context
            .clock
            .now()
            .map_err(|_| TemporalError::EvaluationContextExpired)?;
        let snapshot =
            DispatchSnapshot::new(self, at, context.epoch, context.provider, context.anchors)
                .map_err(PoolingError::from)?;
        let recovery =
            crate::RecoveryContext::new(&snapshot, inputs, policy, trigger, self.recovery_at())?;
        let decision = crate::recover_fleet(&snapshot, recovery)?;
        let bound_context = decision.evidence().context.pooling.clone();
        self.bound(
            Decision::Recovery(decision),
            &bound_context,
            context.clock.time_domain(),
        )
    }
    /// Certifies exact evaluated work; callers cannot rebind an old decision to a new
    /// operational revision or adopt A/B/A and resurrect its original applicability.
    ///
    /// # Errors
    /// Rejects changed authority/adoptions, expired/unproven model or infeasible endpoint.
    pub fn certify(
        &self,
        evaluated: EvaluatedOperationalDecision,
        provider: &CoreRouteProvider<'_>,
        until: DispatchInstant,
    ) -> Result<TemporalProposal, TemporalError> {
        if evaluated.source != self.revision() || evaluated.contexts != *self.contexts() {
            return Err(TemporalError::Stale);
        }
        match evaluated.decision {
            Decision::Insertion(d) => self.certify_insertion(
                evaluated.source,
                &d,
                provider,
                until,
                &evaluated.time_domain,
            ),
            Decision::Fleet(d) => self.certify_fleet(
                evaluated.source,
                &d,
                provider,
                until,
                &evaluated.time_domain,
            ),
            Decision::Recovery(d) => self.certify_recovery(
                evaluated.source,
                &d,
                provider,
                until,
                &evaluated.time_domain,
            ),
        }
    }
    fn interval_snapshot<'a>(
        &'a self,
        source: OperationalRevision,
        context: &'a PoolingContext,
        provider: &'a CoreRouteProvider<'a>,
        until: DispatchInstant,
    ) -> Result<DispatchSnapshot<'a>, TemporalError> {
        self.check_adoptions(context)?;
        if source != self.revision() {
            return Err(TemporalError::Stale);
        }
        if !provider.departure_invariant() {
            return Err(TemporalError::UnsupportedTemporalModel);
        }
        let snapshot =
            DispatchSnapshot::new(self, until, context.epoch, provider, &context.anchors)
                .map_err(PoolingError::from)?;
        if snapshot.provenance != context.inputs.identity.routing {
            return Err(TemporalError::Stale);
        }
        Ok(snapshot)
    }
    fn certificate(
        &self,
        source: OperationalRevision,
        context: &PoolingContext,
        until: DispatchInstant,
        time_domain: &str,
        decision: Decision,
        completions: BTreeMap<crate::OrderId, DispatchInstant>,
    ) -> Result<TemporalProposal, TemporalError> {
        if time_domain.is_empty() {
            return Err(TemporalError::EvaluationContextExpired);
        }
        Ok(TemporalProposal {
            source,
            contexts: self.contexts().clone(),
            time_domain: time_domain.into(),
            evaluated_at: context.at,
            valid_until: until,
            decision,
            endpoint_completions: completions,
        })
    }
    /// Certifies an original exhaustive insertion winner over an explicit interval.
    /// Static objective invariance plus monotone feasibility protects the whole-search
    /// claim without silently selecting/rebasing a different proposal.
    ///
    /// # Errors
    /// Rejects stale context, expired forecast, unproven routing, baseline/candidate
    /// breach or invalid original proposal. Accepted terms are never rewritten.
    fn certify_insertion(
        &self,
        source: OperationalRevision,
        decision: &InsertionDecision,
        provider: &CoreRouteProvider<'_>,
        until: DispatchInstant,
        time_domain: &str,
    ) -> Result<TemporalProposal, TemporalError> {
        let context = &decision.evidence().context;
        let mut guard = self.staged_copy();
        guard.certified_commit_insertion(decision, context)?;
        let snapshot = self.interval_snapshot(source, context, provider, until)?;
        let inputs = endpoint_inputs(context, until, self.data())?;
        let p = decision
            .proposal()
            .ok_or(TemporalError::EvaluationContextExpired)?;
        feasible(evaluate_whole_plan(
            &snapshot,
            &inputs,
            p.rider(),
            &self.data().plans[&p.rider()],
            None,
        )?)?;
        let evaluation = feasible(evaluate_whole_plan(
            &snapshot,
            &inputs,
            p.rider(),
            &p.evaluation().plan,
            Some(p.proposed_assignment.order),
        )?)?;
        if !new_terms_valid(
            &p.accepted,
            evaluation.completions[&p.proposed_assignment.order],
        ) {
            return Err(TemporalError::EvaluationContextExpired);
        }
        self.certificate(
            source,
            context,
            until,
            time_domain,
            Decision::Insertion(decision.clone()),
            evaluation.completions,
        )
    }
    /// Certifies a Phase 18 incumbent's feasibility, not a global optimality claim.
    ///
    /// # Errors
    /// Same expiry/stale/model errors as insertion; preserves original terms/plans.
    fn certify_fleet(
        &self,
        source: OperationalRevision,
        decision: &FleetDecision,
        provider: &CoreRouteProvider<'_>,
        until: DispatchInstant,
        time_domain: &str,
    ) -> Result<TemporalProposal, TemporalError> {
        let context = &decision.evidence().context.pooling;
        let mut guard = self.staged_copy();
        guard.certified_commit_fleet(decision, &decision.evidence().context)?;
        let snapshot = self.interval_snapshot(source, context, provider, until)?;
        let inputs = endpoint_inputs(context, until, self.data())?;
        let p = decision
            .proposal()
            .ok_or(TemporalError::EvaluationContextExpired)?;

        let mut completions = BTreeMap::new();
        for (rider, plan) in p.plans() {
            feasible(evaluate_whole_plan(
                &snapshot,
                &inputs,
                *rider,
                &self.data().plans[rider],
                None,
            )?)?;
            let orders: Vec<_> = p
                .assignments()
                .iter()
                .filter(|(_, a)| a.rider == *rider)
                .map(|(o, _)| *o)
                .collect();
            let evaluation = feasible(evaluate_batch_plan(
                &snapshot, &inputs, *rider, plan, &orders,
            )?)?;
            completions.extend(evaluation.completions);
        }
        for (order, terms) in p.accepted() {
            if !new_terms_valid(terms, completions[order]) {
                return Err(TemporalError::EvaluationContextExpired);
            }
        }
        self.certificate(
            source,
            context,
            until,
            time_domain,
            Decision::Fleet(decision.clone()),
            completions,
        )
    }
    /// Certifies Phase 19 recovery feasibility with unchanged ownership locks/terms.
    /// Static travel and churn objectives remain unchanged through the interval.
    ///
    /// # Errors
    /// Rejects stale/unproven/expired inputs or remaining hard breach.
    fn certify_recovery(
        &self,
        source: OperationalRevision,
        decision: &RecoveryDecision,
        provider: &CoreRouteProvider<'_>,
        until: DispatchInstant,
        time_domain: &str,
    ) -> Result<TemporalProposal, TemporalError> {
        let context = &decision.evidence().context.pooling;
        let mut guard = self.staged_copy();
        guard.certified_commit_recovery(decision, &decision.evidence().context)?;
        let snapshot = self.interval_snapshot(source, context, provider, until)?;
        let inputs = endpoint_inputs(context, until, self.data())?;
        let p = decision
            .proposal()
            .ok_or(TemporalError::EvaluationContextExpired)?;
        let mut completions = BTreeMap::new();
        for (rider, plan) in p.plans() {
            let evaluation = feasible(evaluate_recovery_plan(
                &snapshot,
                &inputs,
                *rider,
                plan,
                p.assignments(),
            )?)?;
            completions.extend(evaluation.completions);
        }
        self.certificate(
            source,
            context,
            until,
            time_domain,
            Decision::Recovery(decision.clone()),
            completions,
        )
    }
    /// Publishes the exact original certified proposal, recording actual clock samples
    /// separately from evaluated completion references. No automatic reevaluation.
    ///
    /// # Errors
    /// Rejects context/revision/time-domain change or interval expiry with zero mutation.
    pub fn publish_temporal(
        &mut self,
        proposal: TemporalProposal,
        clock: &impl OperationalClock,
        provider: &CoreRouteProvider<'_>,
    ) -> Result<PublicationProvenance, TemporalError> {
        if proposal.source != self.revision() || proposal.contexts != *self.contexts() {
            return Err(TemporalError::Stale);
        }
        let context = match &proposal.decision {
            Decision::Insertion(d) => &d.evidence().context,
            Decision::Fleet(d) => &d.evidence().context.pooling,
            Decision::Recovery(d) => &d.evidence().context.pooling,
        };
        if !provider.departure_invariant()
            || provider.provenance() != context.inputs.identity.routing
        {
            return Err(TemporalError::Stale);
        }
        let decision_payload = match &proposal.decision {
            Decision::Insertion(d) => serde_json::to_value(d),
            Decision::Fleet(d) => serde_json::to_value(d),
            Decision::Recovery(d) => serde_json::to_value(d),
        }
        .map_err(|_| TemporalError::EvaluationContextExpired)?;
        let prediction = context.inputs.identity.clone();
        if clock.time_domain() != proposal.time_domain {
            return Err(TemporalError::EvaluationContextExpired);
        }
        let publication_at = clock
            .now()
            .map_err(|_| TemporalError::EvaluationContextExpired)?;
        if publication_at < proposal.evaluated_at || publication_at > proposal.valid_until {
            return Err(TemporalError::EvaluationContextExpired);
        }
        let mut next = self.staged_copy();
        match &proposal.decision {
            Decision::Insertion(d) => next.certified_commit_insertion(d, &d.evidence().context)?,
            Decision::Fleet(d) => next.certified_commit_fleet(d, &d.evidence().context)?,
            Decision::Recovery(d) => next.certified_commit_recovery(d, &d.evidence().context)?,
        }
        let committed_at = clock
            .now()
            .map_err(|_| TemporalError::EvaluationContextExpired)?;
        if committed_at < publication_at || committed_at > proposal.valid_until {
            return Err(TemporalError::EvaluationContextExpired);
        }
        next.world
            .stamp_new_acceptance(self.data(), committed_at)
            .map_err(|_| TemporalError::EvaluationContextExpired)?;
        if matches!(&proposal.decision, Decision::Recovery(_)) {
            next.stamp_recovery_commit(committed_at);
        }
        let commitment_payload = serde_json::json!({
            "world": next.data(), "execution": next.execution(),
            "plan_revisions": next.plan_revisions(), "recovery_at": next.recovery_at(),
        });
        let record = PublicationProvenance {
            schema_version: 1,
            source: proposal.source,
            evaluated_at: proposal.evaluated_at,
            publication_at,
            committed_at,
            committed_revision: next.revision(),
            time_domain: proposal.time_domain,
            validity_policy: STATIC_MONOTONE_POLICY.into(),
            valid_until: proposal.valid_until,
            decision_record: crate::HistoricalRecord {
                schema_version: 1,
                kind: crate::HistoricalRecordKind::Decision,
                operational_revision: proposal.source,
                policy_semantics: "order-protection/v1".into(),
                policy_configuration: fingerprint(&context.inputs.policies)?,
                algorithm_semantics: prediction.optimizer.clone(),
                prediction: prediction.clone(),
                evaluated_at: proposal.evaluated_at,
                committed_at: Some(committed_at),
                payload: decision_payload,
            },
            commitment_record: crate::HistoricalRecord {
                schema_version: 1,
                kind: crate::HistoricalRecordKind::Commitment,
                operational_revision: next.revision(),
                policy_semantics: "order-protection/v1".into(),
                policy_configuration: fingerprint(&context.inputs.policies)?,
                algorithm_semantics: prediction.optimizer.clone(),
                prediction: prediction.clone(),
                evaluated_at: proposal.evaluated_at,
                committed_at: Some(committed_at),
                payload: commitment_payload,
            },
            prediction,
            contexts: proposal.contexts,
            endpoint_completions: proposal.endpoint_completions,
        };
        next.publications.push(record.clone());
        *self = next;
        Ok(record)
    }
}

/// Explicit monotonic logical clock for simulation/tests and deterministic adapter
/// conversion. Live host-time conversion is an adapter responsibility in Phase 20.
#[derive(Debug)]
pub struct LogicalOperationalClock {
    domain: String,
    now: std::cell::Cell<DispatchInstant>,
}
impl LogicalOperationalClock {
    /// Establishes a named time-domain/origin. Empty domains are rejected.
    ///
    /// # Errors
    /// Rejects an empty conversion contract.
    pub fn new(domain: String, now: DispatchInstant) -> Result<Self, crate::DispatchTimeError> {
        if domain.is_empty() {
            return Err(crate::DispatchTimeError);
        }
        Ok(Self {
            domain,
            now: std::cell::Cell::new(now),
        })
    }
    /// Advances authoritative logical time, never backwards.
    ///
    /// # Errors
    /// Rejects clock reversal.
    pub fn advance(&self, at: DispatchInstant) -> Result<(), crate::DispatchTimeError> {
        if at < self.now.get() {
            return Err(crate::DispatchTimeError);
        }
        self.now.set(at);
        Ok(())
    }
}
impl OperationalClock for LogicalOperationalClock {
    fn time_domain(&self) -> &str {
        &self.domain
    }
    fn now(&self) -> Result<DispatchInstant, crate::DispatchTimeError> {
        Ok(self.now.get())
    }
}
