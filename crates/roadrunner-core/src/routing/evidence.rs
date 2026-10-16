//! Versioned route evidence, independent of renderers and operational commitment.
use super::{
    CheckedRouteGeometry, GeometryError, GeometryLimits, RouteResult, RoutingAlgorithm,
    checked_route_geometry,
};
use crate::{cost::CostKind, geo::Seconds, graph::FrozenGraph};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Required semantics. Unknown required variants must reject; optional capabilities may be ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteEvidenceSemantics {
    /// Envelope version; currently 1 only.
    pub schema_version: u32,
    /// Canonical geometry encoding.
    pub geometry: GeometryEncoding,
    /// Route evidence is a deterministic prediction, never physical telemetry or commitment.
    pub authority: RouteAuthority,
}
/// Supported canonical encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryEncoding {
    /// WGS84 integer latitude/longitude at 1e-7 degrees, without altitude.
    Wgs84E7,
}
/// Authority of standalone routing output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteAuthority {
    /// Computed road route, with no assignment or journey implication.
    DeterministicPrediction,
}
impl Default for RouteEvidenceSemantics {
    fn default() -> Self {
        Self {
            schema_version: 1,
            geometry: GeometryEncoding::Wgs84E7,
            authority: RouteAuthority::DeterministicPrediction,
        }
    }
}
impl RouteEvidenceSemantics {
    /// Checks envelope version after required enums have been decoded.
    /// # Errors
    /// Returns unsupported for an unknown required envelope version.
    pub fn check(&self) -> Result<(), GeometryError> {
        if self.schema_version == 1 {
            Ok(())
        } else {
            Err(GeometryError::Unsupported)
        }
    }
}

/// Context captured by the evaluating producer, not reconstructed from current state.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvaluationProvenance {
    /// Versioned evaluator policy supplied by the producer.
    pub evaluator: String,
    /// Original applicable traffic digest; absent for static policies.
    pub traffic_digest: Option<String>,
    /// Original routing departure seconds.
    pub departure_seconds: Seconds,
    /// Original complete routing context, including access authorizations.
    pub routing_context: crate::cost::RoutingContext,
    /// Declared origin/clock domain of those seconds.
    pub time_domain: String,
    /// Actual selected heuristic policy/configuration, absent for non-A* producers.
    pub heuristic: Option<String>,
}
/// Exact algorithm-specific summary. No queue/frontier trace is implied.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteSearchSummary {
    /// Algorithm that produced the route.
    pub algorithm: RoutingAlgorithm,
    /// Node or incoming-edge-qualified state; Yen uses complete loopless path states.
    pub state_mode: &'static str,
    /// Exact legacy counter, qualified by `counter_semantics`.
    pub expanded_states: usize,
    /// Counter definition; no generic steps or unique-node interpretation.
    pub counter_semantics: &'static str,
    /// Strict objective improvement; equal objective does not replace a label.
    pub relaxation: &'static str,
    /// Route was found; alternatives-set termination is separately exposed.
    pub termination: &'static str,
    /// Detailed trace availability.
    pub detailed_trace: &'static str,
}
/// Portable checked route with explicit identity scopes and existing metrics.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RouteEvidence {
    /// Required reader semantics.
    pub semantics: RouteEvidenceSemantics,
    /// Checked embedded geometry; no artifact resolver is required.
    pub geometry: CheckedRouteGeometry,
    /// SHA-256 evaluation artifact identity, independent of returned rank.
    pub evaluation_id: String,
    /// Original evaluator and temporal context.
    pub provenance: EvaluationProvenance,
    /// Pinned routing profile.
    pub routing_profile: String,
    /// Pinned jurisdiction subset.
    pub jurisdiction_policy: String,
    /// Canonical spherical-polyline meters from the route.
    pub distance_meters: f64,
    /// Predicted elapsed road seconds, separate from objective.
    pub predicted_elapsed_seconds: f64,
    /// Existing tagged objective, unchanged.
    pub objective: crate::cost::RouteCost,
    /// Explicit objective unit.
    pub objective_unit: &'static str,
    /// Unsupported decomposition remains unavailable.
    pub cost_breakdown: &'static str,
    /// Algorithm-specific retained summary.
    pub search: RouteSearchSummary,
    /// Optional capabilities can degrade independently of required semantics.
    pub optional_capabilities: Vec<&'static str>,
}

/// Produces a portable immutable derivative of an already evaluated route.
/// The caller must supply the original evaluation context; this function never re-evaluates costs.
/// # Errors
/// Returns checked geometry failures or invalid canonical metric association.
pub fn route_evidence(
    graph: &FrozenGraph,
    route: &RouteResult,
    provenance: EvaluationProvenance,
    limits: GeometryLimits,
) -> Result<RouteEvidence, GeometryError> {
    if provenance.routing_context.departure_time() != provenance.departure_seconds {
        return Err(GeometryError::Invalid);
    }
    let geometry = checked_route_geometry(
        Some(graph),
        route.graph_snapshot_digest(),
        route.path(),
        route.edges(),
        limits,
    )?;
    let distance = geometry
        .occurrences
        .iter()
        .try_fold(crate::geo::Meters::ZERO, |sum, o| {
            sum.checked_add(o.distance_meters)
        })
        .map_err(|_| GeometryError::Invalid)?
        .value();
    if (distance - route.total_distance().value()).abs()
        > crate::graph::DISTANCE_INVARIANT_TOLERANCE_METERS
    {
        return Err(GeometryError::Invalid);
    }
    let yen = route.algorithm() == RoutingAlgorithm::Yen;
    let search = RouteSearchSummary {
        algorithm: route.algorithm(),
        state_mode: if yen {
            "loopless_path"
        } else if graph.metadata().turn_restrictions_enforced() {
            "node_incoming_edge_with_source_sentinel"
        } else {
            "node"
        },
        expanded_states: route.expanded_states(),
        counter_semantics: if yen {
            "originating_root_or_spur_search_path_pops_including_goal"
        } else {
            "accepted_non_stale_pops_including_goal;source_equals_destination_convention_1_without_queue_operations"
        },
        relaxation: if yen {
            "loopless_path_enumeration"
        } else {
            "strict_objective_improvement_only"
        },
        termination: "route_found",
        detailed_trace: "deferred",
    };
    // Struct field order and serde float-roundtrip representation are frozen for v1 identity.
    let bytes = serde_json::to_vec(&(
        1_u32,
        &geometry.path_id,
        &provenance,
        route.algorithm(),
        route.total_cost(),
        route.elapsed_travel_time(),
    ))
    .map_err(|_| GeometryError::Invalid)?;
    let evaluation_id = format!("evaluation-v1:{:x}", Sha256::digest(bytes));
    Ok(RouteEvidence {
        semantics: RouteEvidenceSemantics::default(),
        geometry,
        evaluation_id,
        provenance,
        routing_profile: graph.metadata().routing_profile().to_owned(),
        jurisdiction_policy: graph.metadata().jurisdiction_policy().to_owned(),
        distance_meters: route.total_distance().value(),
        predicted_elapsed_seconds: route.elapsed_travel_time().value(),
        objective: route.total_cost(),
        objective_unit: match route.total_cost().kind() {
            CostKind::Distance => "meters",
            CostKind::TravelTime => "seconds",
        },
        cost_breakdown: "unavailable_not_retained",
        search,
        optional_capabilities: vec![],
    })
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn summary_counter_does_not_round_through_a_browser_number() {
        let summary = super::RouteSearchSummary {
            algorithm: super::RoutingAlgorithm::Dijkstra,
            state_mode: "node",
            expanded_states: 9_007_199_254_740_993,
            counter_semantics: "accepted_non_stale_pops",
            relaxation: "strict_objective_improvement_only",
            termination: "route_found",
            detailed_trace: "deferred",
        };
        let json = serde_json::to_string(&summary).unwrap_or_else(|e| panic!("{e}"));
        let decoded: serde_json::Value =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            decoded["expanded_states"].as_u64(),
            Some(9_007_199_254_740_993)
        );
    }
}
