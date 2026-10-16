//! Renderer-neutral interpretations of retained decisions. No new candidate history or authority.
use crate::{
    AssignmentDecision, CandidateCoverage, DispatchDecisionOutcome, FleetDecision,
    InsertionDecision, RecoveryDecision, RecoveryTermination,
};
use serde::Serialize;

/// Independent quality dimensions, attached to the original evidence (never a replacement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DecisionInspection {
    /// Inspection semantics version.
    pub schema_version: u32,
    /// Producer family; work units remain family-specific.
    pub family: &'static str,
    /// Required inputs were sufficient for the returned declared scope.
    pub input_complete: bool,
    /// Exact population/placement coverage qualification.
    pub coverage: &'static str,
    /// Declared search completion, separate from global optimality.
    pub search_complete: bool,
    /// Retained detail granularity; counts never generate candidate rows.
    pub retention: &'static str,
    /// Algorithm-specific comparison contract.
    pub comparison: &'static str,
    /// Scope of the quality claim.
    pub quality: &'static str,
    /// The pure evaluation offers a proposal; certification/publication can still reject.
    pub proposal_available: bool,
    /// Evaluation authority stage. Commitment requires separate publication evidence.
    pub authority: &'static str,
    /// No discovery history was recorded; canonical result order is not event order.
    pub discovery_order: &'static str,
    /// Hypothetical ordered leg road geometry was not retained by these summaries.
    pub candidate_leg_geometry: &'static str,
    /// Geographic scope cannot be detached without original request/snapshot association.
    pub geographic_scope: &'static str,
    /// Exact work-accounting definition.
    pub work_unit: &'static str,
}
impl DecisionInspection {
    fn new(
        family: &'static str,
        input_complete: bool,
        search_complete: bool,
        proposal_available: bool,
    ) -> Self {
        Self {
            schema_version: 1,
            family,
            input_complete,
            coverage: "declared_evaluable_scope",
            search_complete,
            retention: "aggregate_counts_and_selected_proposal;not_candidate_history",
            comparison: "",
            quality: "declared_scope_only_not_global_optimality",
            proposal_available,
            authority: if proposal_available {
                "recommended_proposal_not_committed"
            } else {
                "evaluation_without_proposal"
            },
            discovery_order: "not_retained",
            candidate_leg_geometry: "unavailable_not_retained",
            geographic_scope: "nonradial_no_radius_claim",
            work_unit: "",
        }
    }
}
impl AssignmentDecision {
    /// Exposes supported basic-dispatch dimensions; the original evidence retains each candidate result.
    #[must_use]
    pub fn inspection(&self) -> DecisionInspection {
        let mut i = DecisionInspection::new(
            "basic_dispatch",
            true,
            true,
            matches!(self.outcome(), DispatchDecisionOutcome::Assigned(_)),
        );
        i.coverage = match self.evidence().coverage {
            CandidateCoverage::Complete => "complete_basic_dispatch_eligible_fleet",
            CandidateCoverage::PotentiallyIncomplete => "potentially_incomplete_spatial_shortlist",
        };
        i.retention = "per_candidate_feasible_or_rejected;recommendation_separate_from_commitment";
        i.comparison =
            "named_strategy_then_canonical_rider_id;waiting_penalty_is_preference_not_road_time";
        i.geographic_scope = match self.evidence().candidate_policy {
            crate::CandidatePolicy::Exhaustive => "exhaustive_no_finite_radius",
            crate::CandidatePolicy::Spatial { .. } => {
                "pickup_center_requires_original_order;inclusive_haversine_meters_radius_and_limit_over_basic_eligible_riders"
            }
        };
        i.work_unit = "retained_candidate_evaluations_in_canonical_rider_order";
        i
    }
}
impl InsertionDecision {
    /// Exposes aggregate insertion work without manufacturing placement histories.
    #[must_use]
    pub fn inspection(&self) -> DecisionInspection {
        let e = self.evidence();
        let mut i = DecisionInspection::new(
            "insertion",
            e.input_complete,
            e.search_complete,
            self.proposal().is_some(),
        );
        i.coverage = if e.riders_complete {
            "all_eligible_riders;declared_pickup_dropoff_insertions"
        } else {
            "incomplete_rider_coverage"
        };
        i.comparison = "signed_incremental_road_seconds_then_meters_then_canonical_ties;accepted_and_current_protections";
        i.work_unit =
            "complete_placement_submissions;baselines_and_policy_exclusions_outside_budget";
        i
    }
}
impl FleetDecision {
    /// Exposes fleet lexicographic accounting and independently qualified completeness.
    #[must_use]
    pub fn inspection(&self) -> DecisionInspection {
        let e = self.evidence();
        let mut i = DecisionInspection::new(
            "fleet",
            e.input_complete,
            e.search_complete,
            self.proposal().is_some(),
        );
        i.coverage = if e.riders_complete {
            "all_riders_classified;evaluable_remainder_excludes_isolated_inputs"
        } else {
            "incomplete_rider_classification"
        };
        i.comparison = "maximize_new_admissions_then_minimize_remaining_road_seconds_then_meters_then_canonical_ties;frozen_constants_excluded;accepted_terms_immutable";
        i.work_unit = "complete_fleet_submissions_by_construction_and_neighborhood;feasible_may_repeat;reasons_may_exceed_submissions";
        i
    }
}
impl RecoveryDecision {
    /// Exposes local recovery scope, churn policy and no-incumbent-on-incomplete semantics.
    #[must_use]
    pub fn inspection(&self) -> DecisionInspection {
        let e = self.evidence();
        let mut i = DecisionInspection::new(
            "recovery",
            true,
            !matches!(
                e.termination,
                RecoveryTermination::SearchIncomplete | RecoveryTermination::Cooldown
            ),
            self.proposal().is_some(),
        );
        i.comparison = "remaining_road_seconds_plus_explicit_churn_penalties_then_meters_then_canonical_ties;health_cooldown_threshold_and_accepted_protections";
        i.quality =
            "declared_recovery_neighborhood_only;cooldown_or_threshold_is_not_infeasibility";
        i.work_unit = "complete_move_evaluations;precedence_exclusions_separate;rejection_occurrences_may_exceed_moves";
        i
    }
}
