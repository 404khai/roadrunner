//! Constrained historical input boundary: no scenario, label, schedule or final-state access.
use crate::DatasetError;
use roadrunner_core::{
    geo::{Coordinate, Meters, Seconds},
    graph::FrozenGraph,
};
use roadrunner_dispatch::{FrozenExecutionLeg, OperationalRevision};
use roadrunner_simulation::StartedAction;
use serde::{Deserialize, Serialize};

/// Missingness distinguishes predictor absence, archival loss and unsupported semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnavailableReason {
    /// Input did not exist at the predictor boundary.
    PredictorTimeAbsent,
    /// Original input existed but recoverable bytes were not retained.
    ArchivalLoss,
    /// Original schema/interpretation does not establish this field.
    UnsupportedSemantics,
    /// Not meaningful for this source (e.g. civil calendar for run seconds).
    NotApplicable,
}
/// A value or an explicit unavailability reason; missingness is metadata, not a predictor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Availability<T> {
    /// Established by the recorded start inputs.
    Available(T),
    /// No fabricated/backfilled value.
    Unavailable(UnavailableReason),
}
/// Explicit historical availability witness; later archival time cannot substitute it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AvailabilityWitness {
    /// Start commit order, authoritative even when timestamps round equally.
    pub start_revision: OperationalRevision,
    /// Original input captured synchronously inside the successful start operation.
    pub rule: String,
    /// Transform/interpretation version.
    pub transform: String,
}
/// Initial allowlisted values; no identity, seed, policy or latent future predictors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartFeatures {
    /// Original route elapsed estimate in seconds, never rescheduled ETA.
    pub b0_seconds: Seconds,
    /// Original directed route length in meters.
    pub distance_meters: Meters,
    /// Origin location from original graph; archival loss is explicit.
    pub origin: Availability<Coordinate>,
    /// Destination location from original graph.
    pub destination: Availability<Coordinate>,
    /// Per-traversal original static multipliers, matched to original route edges.
    pub traffic_multipliers: Vec<f64>,
    /// Original per-traversal free-flow seconds where supported.
    pub edge_free_flow_seconds: Availability<Vec<Seconds>>,
    /// Core graph does not retain a road-class taxonomy in its current artifact.
    pub road_class: Availability<Vec<String>>,
    /// Simulation seconds have no civil timezone/epoch.
    pub day_of_week: Availability<u8>,
    /// Original complete route and graph/profile/traffic binding (lineage, not predictor IDs).
    pub original_route: FrozenExecutionLeg,
    /// Feature-category availability proof.
    pub witness: AvailabilityWitness,
}
/// Builds from original start inputs only; outcome and generator information is absent by type.
/// # Errors
/// Rejects graph/traffic mismatches and invalid path interpretation; never recomputes B0.
pub fn extract_features(
    start: &StartedAction,
    graph: Option<&FrozenGraph>,
) -> Result<StartFeatures, DatasetError> {
    let route = &start.original.route;
    if start.original.generation.value() != 0 {
        return Err(DatasetError::Source("not an original start".into()));
    }
    let (origin, destination, free_flow) = if let Some(g) = graph {
        if route.routing.graph_digest != g.metadata().snapshot_digest() {
            return Err(DatasetError::Integrity(
                "original graph/traffic identity".into(),
            ));
        }
        let overlay = roadrunner_core::cost::TrafficSnapshot::new(
            g,
            start
                .traffic
                .iter()
                .map(|o| (roadrunner_core::graph::EdgeId::new(o.edge_id), o.multiplier)),
        )
        .map_err(|e| DatasetError::Integrity(e.to_string()))?;
        if route.routing.traffic
            != roadrunner_dispatch::TrafficIdentity::Static(
                overlay.traffic_snapshot_digest().to_owned(),
            )
        {
            return Err(DatasetError::Integrity(
                "original traffic content identity".into(),
            ));
        }
        let node = |id| {
            g.node(id)
                .map(|n| Availability::Available(n.coordinate()))
                .ok_or_else(|| DatasetError::Integrity("route node absent".into()))
        };
        if route.nodes.len() != route.edges.len() + 1 {
            return Err(DatasetError::Integrity("route shape".into()));
        }
        let mut seconds = Vec::new();
        for (edge, pair) in route.edges.iter().zip(route.nodes.windows(2)) {
            let e = g
                .edge(*edge)
                .ok_or_else(|| DatasetError::Integrity("edge absent".into()))?;
            if e.from() != pair[0] || e.to() != pair[1] {
                return Err(DatasetError::Integrity("directed edge mismatch".into()));
            }
            seconds.push(e.free_flow_travel_time());
        }
        (
            node(route.from)?,
            node(route.to)?,
            Availability::Available(seconds),
        )
    } else {
        (
            Availability::Unavailable(UnavailableReason::ArchivalLoss),
            Availability::Unavailable(UnavailableReason::ArchivalLoss),
            Availability::Unavailable(UnavailableReason::ArchivalLoss),
        )
    };
    Ok(StartFeatures {
        b0_seconds: route.travel,
        distance_meters: route.distance,
        origin,
        destination,
        traffic_multipliers: route
            .edges
            .iter()
            .map(|id| {
                start
                    .traffic
                    .iter()
                    .find(|o| o.edge_id == id.value())
                    .map_or(1.0, |o| o.multiplier.value())
            })
            .collect(),
        edge_free_flow_seconds: free_flow,
        road_class: Availability::Unavailable(UnavailableReason::UnsupportedSemantics),
        day_of_week: Availability::Unavailable(UnavailableReason::NotApplicable),
        original_route: route.clone(),
        witness: AvailabilityWitness {
            start_revision: start.committed_revision,
            rule: "original-static-inputs-captured-at-start-commit/v1".into(),
            transform: "allowlist-route-static/v1".into(),
        },
    })
}
