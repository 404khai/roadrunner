//! Bounded, read-only graph views. Standalone edge costs are not maneuver legality.
use crate::{Api, error::ApiError, model::Objective};
use roadrunner_core::{
    cost::{
        DistanceCost, RoutingContext, TrafficAwareCost, TrafficSnapshot, TravelTimeCost,
        TraversalEvaluation, TraversalEvaluator, TraversalState,
    },
    geo::Seconds,
    graph::{NodeId, Orientation},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
pub(crate) struct GraphQuery {
    pub graph_digest: String,
    #[serde(default)]
    pub node_offset: usize,
    #[serde(default)]
    pub edge_offset: usize,
    #[serde(default)]
    pub objective: Objective,
}

#[allow(clippy::too_many_lines)]
pub(crate) fn export(
    api: &Api,
    query: &GraphQuery,
    traffic: Option<&TrafficSnapshot>,
    departure: Seconds,
) -> Result<Value, ApiError> {
    let graph = &api.inner.graph;
    // Dense IDs never resolve before checking the full semantic identity.
    if query.graph_digest != graph.metadata().snapshot_digest() {
        return Err(ApiError::conflict("graph snapshot mismatch"));
    }
    if query.node_offset > graph.node_count() || query.edge_offset > graph.edge_count() {
        return Err(ApiError::invalid("graph page offset exceeds graph size"));
    }
    let node_end = query
        .node_offset
        .saturating_add(500)
        .min(graph.node_count());
    let mut nodes: BTreeSet<NodeId> = graph.nodes()[query.node_offset..node_end]
        .iter()
        .map(|n| n.id())
        .collect();
    let context = RoutingContext::with_departure_time(departure);
    let traffic_cost = traffic.map(TrafficAwareCost::new);
    let evaluator: Option<&dyn TraversalEvaluator> = match query.objective {
        Objective::Distance => Some(&DistanceCost),
        Objective::TravelTime => Some(&TravelTimeCost),
        Objective::TrafficAware => traffic_cost.as_ref().map(|c| c as &dyn TraversalEvaluator),
    };
    let mut edges = Vec::new();
    let mut point_count = 0;
    let mut edge_end = query.edge_offset;
    for edge in graph.edges().iter().skip(query.edge_offset).take(200) {
        let segment = graph
            .segment(edge.segment())
            .ok_or_else(ApiError::internal)?;
        let points = graph
            .segment_geometry(edge.segment())
            .map_err(ApiError::invalid)?;
        if point_count + points.len() > 20_000 {
            if edges.is_empty() {
                return Err(ApiError::new(
                    axum::http::StatusCode::PAYLOAD_TOO_LARGE,
                    "graph_geometry_limit",
                    "one edge exceeds the graph page's 20000 canonical point budget",
                ));
            }
            break;
        }
        let directed: Vec<_> = if edge.orientation() == Orientation::Reverse {
            points.iter().rev().copied().collect()
        } else {
            points.to_vec()
        };
        let (status, cost, elapsed) = match evaluator
            .map(|e| e.evaluate(edge, segment, TraversalState::new(Seconds::ZERO), &context))
            .transpose()
            .map_err(ApiError::invalid)?
        {
            None => ("unavailable_no_traffic_snapshot", None, None),
            Some(TraversalEvaluation::Forbidden) => ("forbidden", None, None),
            Some(TraversalEvaluation::Traversable {
                objective_cost,
                travel_time,
            }) => (
                "traversable",
                Some(objective_cost.value()),
                Some(travel_time.value()),
            ),
        };
        edges.push(json!({"edge":edge.id(),"segment":edge.segment(),"from":edge.from(),"to":edge.to(),"orientation":edge.orientation(),"points_e7":directed,"distance_meters":segment.distance(),"base_seconds":edge.free_flow_travel_time(),"traffic_multiplier":traffic.and_then(|t|t.multiplier(edge.id())).map(roadrunner_core::cost::TrafficMultiplier::value),"access":edge.properties().access(),"road_class":null,"cost_status":status,"objective_cost":cost,"predicted_elapsed_seconds":elapsed}));
        nodes.insert(edge.from());
        nodes.insert(edge.to());
        point_count += points.len();
        edge_end += 1;
    }
    let nodes = nodes
        .into_iter()
        .map(|id| {
            let n = graph.node(id).ok_or_else(ApiError::internal)?;
            Ok(json!({"node":id,"coordinate_e7":n.canonical_coordinate()}))
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let mut result = json!({"schema_version":1,"semantics":"bounded_directed_graph_inspection/v1","graph":graph.metadata(),"node_count":graph.node_count(),"edge_count":graph.edge_count(),"node_page":{"offset":query.node_offset,"end":node_end,"next_offset":(node_end<graph.node_count()).then_some(node_end)},"edge_page":{"offset":query.edge_offset,"end":edge_end,"next_offset":(edge_end<graph.edge_count()).then_some(edge_end)},"point_count":point_count,"nodes_scope":"node_page_plus_exported_edge_endpoints","edge_scope":"contiguous_page_not_spatial_query","nodes":nodes,"edges":edges,"traffic_snapshot_digest":traffic.map(TrafficSnapshot::traffic_snapshot_digest),"evaluation":{"objective":query.objective,"evaluator":match query.objective {Objective::Distance=>"distance/v1",Objective::TravelTime=>"free-flow-travel-time/v1",Objective::TrafficAware=>"traffic-aware-travel-time/v1"},"objective_unit":if matches!(query.objective,Objective::Distance){"meters"}else{"seconds"},"traffic_digest":if matches!(query.objective,Objective::TrafficAware){traffic.map(TrafficSnapshot::traffic_snapshot_digest)}else{None},"departure_seconds":departure,"time_domain":api.inner.clock.time_domain(),"routing_context":context,"scope":"standalone_directed_traversal_default_access;not_maneuver_or_route_feasibility"},"capabilities":{"road_class":"unavailable_not_retained","explored_node_membership":"unavailable_not_retained","ordered_search_trace":"unavailable_not_retained"}});
    let digest = Sha256::digest(serde_json::to_vec(&result).map_err(ApiError::invalid)?);
    result["graph_view_id"] = json!(format!("graph-view-v1:{digest:x}"));
    Ok(result)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use roadrunner_core::{
        geo::{CanonicalCoordinate, KilometersPerHour},
        graph::*,
    };
    fn properties(access: AccessClass) -> EdgeProperties {
        EdgeProperties::new(KilometersPerHour::new(36.0).unwrap(), None, access)
    }
    fn api(graph: FrozenGraph) -> Api {
        let (original, clock, _) = crate::test_support::fixture();
        Api::with_clock(
            graph,
            original.inner.config.clone(),
            roadrunner_dispatch::OperationalWorldNamespace::from_bytes([9; 16]),
            clock,
        )
        .unwrap()
    }
    fn query(api: &Api, edge_offset: usize, node_offset: usize) -> GraphQuery {
        GraphQuery {
            graph_digest: api.inner.graph.metadata().snapshot_digest().into(),
            node_offset,
            edge_offset,
            objective: Objective::TravelTime,
        }
    }
    fn builder() -> GraphBuilder {
        GraphBuilder::new(
            GraphSnapshotId::new(42),
            GraphMetadata::new(
                "synthetic/v1",
                "test",
                GraphBuildIdentity::new("debug-test", "v1", "v1", "test"),
            ),
        )
    }
    #[test]
    fn node_and_edge_caps_include_page_endpoints_and_forbidden_is_not_zero_cost() {
        let mut b = builder();
        for id in 0..501 {
            b.add_node(
                BuilderNodeId::new(id),
                CanonicalCoordinate::new(i32::try_from(id * 1000).unwrap(), 0).unwrap(),
            )
            .unwrap();
        }
        for id in 0..101 {
            b.add_segment(
                BuilderSegmentId::new(id),
                BuilderNodeId::new(id),
                BuilderNodeId::new(id + 1),
                vec![
                    CanonicalCoordinate::new(i32::try_from(id * 1000).unwrap(), 0).unwrap(),
                    CanonicalCoordinate::new(i32::try_from((id + 1) * 1000).unwrap(), 0).unwrap(),
                ],
                Some(properties(if id == 0 {
                    AccessClass::Private
                } else {
                    AccessClass::General
                })),
                Some(properties(AccessClass::General)),
            )
            .unwrap();
        }
        let api = api(b.finalize().unwrap());
        let first = export(&api, &query(&api, 0, 0), None, Seconds::ZERO).unwrap();
        assert_eq!(first["nodes"].as_array().unwrap().len(), 500);
        assert_eq!(first["edges"].as_array().unwrap().len(), 200);
        assert_eq!(first["edge_page"]["next_offset"], 200);
        assert_eq!(first["node_page"]["next_offset"], 500);
        assert_eq!(first["edges"][0]["cost_status"], "forbidden");
        assert_eq!(first["edges"][0]["objective_cost"], Value::Null);
        let tail = export(&api, &query(&api, 200, 500), None, Seconds::ZERO).unwrap();
        assert_eq!(tail["edges"].as_array().unwrap().len(), 2);
        assert_eq!(tail["nodes"].as_array().unwrap().len(), 3);
        assert_eq!(tail["edge_page"]["next_offset"], Value::Null);
    }
    fn curved_api(vertices: i32) -> Api {
        let mut b = builder();
        let points = (0..vertices)
            .map(|i| CanonicalCoordinate::new(i, 0).unwrap())
            .collect::<Vec<_>>();
        b.add_node(BuilderNodeId::new(0), points[0]).unwrap();
        b.add_node(BuilderNodeId::new(1), *points.last().unwrap())
            .unwrap();
        b.add_segment(
            BuilderSegmentId::new(0),
            BuilderNodeId::new(0),
            BuilderNodeId::new(1),
            points,
            Some(properties(AccessClass::General)),
            Some(properties(AccessClass::General)),
        )
        .unwrap();
        api(b.finalize().unwrap())
    }
    #[test]
    fn point_budget_pages_at_edge_boundaries_and_never_clips_curves() {
        let api = curved_api(10001);
        let first = export(&api, &query(&api, 0, 0), None, Seconds::ZERO).unwrap();
        assert_eq!(first["point_count"], 10001);
        assert_eq!(first["edges"].as_array().unwrap().len(), 1);
        assert_eq!(first["edge_page"]["next_offset"], 1);
        let second = export(&api, &query(&api, 1, 0), None, Seconds::ZERO).unwrap();
        assert_eq!(second["edges"][0]["points_e7"][0]["latitude_e7"], 10000);
        assert_eq!(second["edge_page"]["next_offset"], Value::Null);
        let oversized = curved_api(20001);
        assert_eq!(
            export(&oversized, &query(&oversized, 0, 0), None, Seconds::ZERO)
                .unwrap_err()
                .body
                .code,
            "graph_geometry_limit"
        );
    }
}
