use roadrunner_core::geo::{Meters, Seconds};
use serde::Serialize;

use crate::{
    CommittedAssignment, DecisionId, DispatchInstant, OrderId, PlanEvaluation, PlanValidityError,
    RiderId, RiderPlan, RoutingEpoch, RoutingProvenance, WorldVersion,
};

/// Candidate-generation policy with recorded configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum CandidatePolicy {
    /// Evaluate all riders passing Basic Dispatch eligibility.
    Exhaustive,
    /// Evaluate a spatial radius/limit shortlist of eligible positions.
    Spatial {
        /// Inclusive straight-line radius.
        radius: Meters,
        /// Maximum number of candidates; zero produces an empty shortlist.
        limit: usize,
    },
}

/// Whether candidate generation covers the complete eligible rider fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CandidateCoverage {
    /// Every eligible rider is evaluated.
    Complete,
    /// Bounded screening may exclude feasible or better riders.
    PotentiallyIncomplete,
}

impl CandidatePolicy {
    /// Coverage is a policy guarantee, never inferred from returned count.
    #[must_use]
    pub const fn coverage(self) -> CandidateCoverage {
        match self {
            Self::Exhaustive => CandidateCoverage::Complete,
            Self::Spatial { .. } => CandidateCoverage::PotentiallyIncomplete,
        }
    }
}

/// Typed structural rejection, never an infinite or penalized score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CandidateRejection {
    /// Valid anchored locations have no legal connecting road route.
    NoRoute,
    /// Demand cannot fit maximum scalar rider capacity.
    CapacityExceeded,
    /// Pending work violates state-aware plan validity.
    InvalidPlan,
    /// Proposed responsibility contradicts onboard custody.
    CustodyViolation,
    /// Rider cannot use the pinned routing profile.
    UnsupportedCandidateProfile,
}

impl From<PlanValidityError> for CandidateRejection {
    fn from(value: PlanValidityError) -> Self {
        match value {
            PlanValidityError::InvalidPlan => Self::InvalidPlan,
            PlanValidityError::CustodyViolation => Self::CustodyViolation,
            PlanValidityError::CapacityExceeded => Self::CapacityExceeded,
        }
    }
}

/// Explicit Phase 13 deadline semantics; no lateness penalty or hard rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DeadlinePolicy {
    /// Observe completed-dropoff lateness without affecting ranking or feasibility.
    SoftObserved,
}

/// Structured baseline contributions, all expressed in road travel seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ScoreContributions {
    /// Rider-to-pickup road movement.
    pub pickup_travel: Seconds,
    /// Pickup-to-dropoff road movement.
    pub delivery_travel: Seconds,
    /// Exact checked sum used for ranking.
    pub total: Seconds,
}

/// Evidence for one evaluated candidate, ordered canonically by `RiderId`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CandidateEvidence {
    /// Rider considered.
    pub rider: RiderId,
    /// Metrics and score, or structural rejection.
    pub result: CandidateResult,
}

/// Feasible measured outcome is distinct from structural candidate rejection.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum CandidateResult {
    /// Feasible, fully evaluated remaining plan.
    Feasible {
        /// Timing and physical metrics, independent of strategy preference.
        evaluation: PlanEvaluation,
        /// Structured baseline objective contributions.
        score: ScoreContributions,
    },
    /// Structurally infeasible candidate.
    Rejected(CandidateRejection),
}

/// Exact responsibility and remaining-plan state change proposed by evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssignmentProposal {
    /// New request.
    pub order: OrderId,
    /// Selected idle rider; derived convenience identity.
    pub rider: RiderId,
    /// Exact prior responsibility expected at commit.
    pub expected_assignment: Option<CommittedAssignment>,
    /// Exact prior remaining work expected at commit.
    pub expected_plan: RiderPlan,
    /// Proposed current responsibility.
    pub proposed_assignment: CommittedAssignment,
    /// Proposed complete remaining plan.
    pub proposed_plan: RiderPlan,
}

/// Scope of the completed infeasibility claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum UnassignedScope {
    /// No feasible rider among the complete Basic Dispatch eligible fleet.
    EligibleFleet,
    /// No feasible rider among the bounded candidates evaluated.
    EvaluatedCandidates,
}

/// Completed evaluation outcome; evaluation failures use a separate Result error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum DispatchDecisionOutcome {
    /// Exact coherent state proposal, not a committed assignment.
    Assigned(AssignmentProposal),
    /// Valid completed decision without a feasible selected plan.
    Unassigned {
        /// Meaning of the unsuccessful candidate search.
        scope: UnassignedScope,
    },
}

/// Structured reason for deterministic selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SelectionReason {
    /// Unique minimum baseline score among evaluated feasible candidates.
    LowestScore,
    /// Exact minimum score tie resolved by lower rider identity.
    ExactScoreThenRiderId {
        /// Canonical identities of all riders tied at the minimum.
        tied_riders: Vec<RiderId>,
    },
}

/// Compact replay/explanation evidence; full hypothetical road geometry is optional.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DecisionEvidence {
    /// Caller-supplied immutable decision identity.
    pub decision_id: DecisionId,
    /// Source world identity.
    pub world_identity: u64,
    /// Whole-world source version.
    pub world_version: WorldVersion,
    /// Explicit caller-supplied evaluation instant.
    pub evaluated_at: DispatchInstant,
    /// Declared checked conversion epoch.
    pub routing_epoch: RoutingEpoch,
    /// Fixed graph, traffic, and profile.
    pub routing: RoutingProvenance,
    /// Versioned pure ranking policy (no tunable baseline weights).
    pub strategy: String,
    /// Versioned hard constraint policy.
    pub feasibility_policy: String,
    /// Versioned eligibility policy.
    pub eligibility_policy: String,
    /// Explicit baseline deadline semantics.
    pub deadline_policy: DeadlinePolicy,
    /// Candidate-generation policy/configuration.
    pub candidate_policy: CandidatePolicy,
    /// Complete or potentially incomplete coverage.
    pub coverage: CandidateCoverage,
    /// Canonical evidence for every evaluated candidate.
    pub candidates: Vec<CandidateEvidence>,
    /// Selection reason, absent for Unassigned.
    pub selection_reason: Option<SelectionReason>,
}

/// Immutable completed decision; only the evaluator can construct it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssignmentDecision {
    evidence: DecisionEvidence,
    outcome: DispatchDecisionOutcome,
}

impl AssignmentDecision {
    pub(crate) const fn new(evidence: DecisionEvidence, outcome: DispatchDecisionOutcome) -> Self {
        Self { evidence, outcome }
    }
    /// Returns read-only structured evidence.
    #[must_use]
    pub const fn evidence(&self) -> &DecisionEvidence {
        &self.evidence
    }
    /// Returns the exact proposal or structured unsuccessful outcome.
    #[must_use]
    pub const fn outcome(&self) -> &DispatchDecisionOutcome {
        &self.outcome
    }
    /// Renders a concise explanation from structured source-of-truth evidence.
    #[must_use]
    pub fn explanation(&self) -> String {
        match &self.outcome {
            DispatchDecisionOutcome::Assigned(p) => format!(
                "Rider {} has the lowest pickup + delivery road travel score among evaluated candidates ({:?} coverage)",
                p.rider.value(),
                self.evidence.coverage
            ),
            DispatchDecisionOutcome::Unassigned { scope } => {
                format!("No feasible rider in scope {scope:?}")
            }
        }
    }
}
