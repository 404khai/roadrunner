//! Independent correctness checks for the frozen routing core.
#![allow(clippy::float_cmp)]

use std::collections::BTreeMap;

use roadrunner_core::cost::{
    CostError, CostKind, DistanceCost, RouteCost, RoutingContext, SearchCapability, TravelTimeCost,
    TraversalEvaluation, TraversalEvaluator, TraversalState,
};
use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour, Meters, Seconds};
use roadrunner_core::graph::{
    AccessClass, BuilderNodeId, BuilderSegmentId, DirectedEdge, EdgeId, EdgeProperties,
    FrozenGraph, GraphBuildIdentity, GraphBuilder, GraphMetadata, GraphSnapshotId, NodeId,
    RoadSegment, decode_graph_artifact, encode_graph_artifact, write_graph_artifact_atomic,
};
use roadrunner_core::routing::{
    AlternativeRouteOptions, AlternativeTermination, DistanceHaversine, RoutingError,
    TravelTimeHaversine, ZeroHeuristic, alternatives, astar, dijkstra,
};

#[test]
fn alternatives_skip_tiny_detour_and_keep_distinct_corridor() {
    let network = graph(
        7,
        &[
            (0, 0, 1),
            (1, 1, 2),
            (2, 2, 3),
            (3, 3, 4),
            (4, 2, 5),
            (5, 5, 3),
            (6, 0, 6),
            (7, 6, 4),
        ],
    );
    let weights = BTreeMap::from([
        (edge_between(&network, 0, 1), 1.0),
        (edge_between(&network, 1, 2), 1.0),
        (edge_between(&network, 2, 3), 1.0),
        (edge_between(&network, 3, 4), 1.0),
        (edge_between(&network, 2, 5), 1.0),
        (edge_between(&network, 5, 3), 1.0),
        (edge_between(&network, 0, 6), 3.0),
        (edge_between(&network, 6, 4), 3.0),
    ]);
    let evaluator = WeightedEvaluator {
        weights,
        forbidden: None,
        fail: None,
    };
    let options = AlternativeRouteOptions {
        max_routes: 3,
        max_shared_distance_ratio: 0.6,
        max_cost_factor: 3.0,
        ..AlternativeRouteOptions::default()
    };
    let result = alternatives(
        &network,
        NodeId::new(0),
        NodeId::new(4),
        &evaluator,
        &RoutingContext::new(),
        options,
    )
    .unwrap_or_else(|error| panic!("alternatives failed: {error}"));
    assert_eq!(result.routes.len(), 2);
    assert_eq!(
        result.routes[0].route.path(),
        &[
            NodeId::new(0),
            NodeId::new(1),
            NodeId::new(2),
            NodeId::new(3),
            NodeId::new(4)
        ]
    );
    assert_eq!(
        result.routes[1].route.path(),
        &[NodeId::new(0), NodeId::new(6), NodeId::new(4)]
    );
    assert_eq!(result.routes[1].rank, 2);
    assert_eq!(result.routes[1].max_shared_distance_ratio, 0.0);
    assert!(result.ranked_paths >= 3);
    assert!(!result.truncated);
    let baseline = dijkstra(
        &network,
        NodeId::new(0),
        NodeId::new(4),
        &evaluator,
        &RoutingContext::new(),
    )
    .unwrap_or_else(|error| panic!("baseline failed: {error}"));
    assert_eq!(result.routes[0].route.total_cost(), baseline.total_cost());
}

#[test]
fn alternatives_respect_incoming_maneuver_and_one_way_edges() {
    let unrestricted = graph(5, &[(0, 0, 1), (1, 1, 2), (2, 2, 4), (3, 0, 3), (4, 3, 4)]);
    let forbidden = (
        edge_between(&unrestricted, 1, 2),
        edge_between(&unrestricted, 2, 4),
    );
    let network = unrestricted
        .with_forbidden_maneuvers(vec![forbidden])
        .unwrap_or_else(|error| panic!("maneuver fixture failed: {error}"));
    let result = alternatives(
        &network,
        NodeId::new(0),
        NodeId::new(4),
        &DistanceCost,
        &RoutingContext::new(),
        AlternativeRouteOptions::default(),
    )
    .unwrap_or_else(|error| panic!("alternatives failed: {error}"));
    assert_eq!(result.routes.len(), 1);
    assert_eq!(
        result.routes[0].route.path(),
        &[NodeId::new(0), NodeId::new(3), NodeId::new(4)]
    );
    assert!(
        result.routes[0]
            .route
            .edges()
            .windows(2)
            .all(|pair| network.is_maneuver_allowed(pair[0], pair[1]))
    );
    assert!(matches!(
        alternatives(
            &network,
            NodeId::new(4),
            NodeId::new(0),
            &DistanceCost,
            &RoutingContext::new(),
            AlternativeRouteOptions::default()
        ),
        Err(RoutingError::NoRoute { .. })
    ));
}

#[test]
fn alternatives_rank_three_loopless_corridors_by_cost() {
    let network = graph(
        5,
        &[
            (0, 0, 1),
            (1, 1, 4),
            (2, 0, 2),
            (3, 2, 4),
            (4, 0, 3),
            (5, 3, 4),
            (6, 4, 0),
        ],
    );
    let weights = BTreeMap::from([
        (edge_between(&network, 0, 1), 1.0),
        (edge_between(&network, 1, 4), 1.0),
        (edge_between(&network, 0, 2), 1.5),
        (edge_between(&network, 2, 4), 1.5),
        (edge_between(&network, 0, 3), 2.0),
        (edge_between(&network, 3, 4), 2.0),
    ]);
    let evaluator = WeightedEvaluator {
        weights,
        forbidden: None,
        fail: None,
    };
    let result = alternatives(
        &network,
        NodeId::new(0),
        NodeId::new(4),
        &evaluator,
        &RoutingContext::new(),
        AlternativeRouteOptions {
            max_cost_factor: 3.0,
            ..AlternativeRouteOptions::default()
        },
    )
    .unwrap_or_else(|error| panic!("alternatives failed: {error}"));
    assert_eq!(result.routes.len(), 3);
    assert_eq!(result.termination, AlternativeTermination::RequestedCount);
    assert_eq!(
        result
            .routes
            .iter()
            .map(|route| route.route.total_cost().value())
            .collect::<Vec<_>>(),
        vec![2.0, 3.0, 4.0]
    );
    for route in result.routes {
        let nodes: std::collections::BTreeSet<_> = route.route.path().iter().copied().collect();
        assert_eq!(nodes.len(), route.route.path().len());
    }
}

#[test]
fn alternatives_report_budget_and_degenerate_cases() {
    let network = graph(3, &[(0, 0, 1), (1, 1, 2), (2, 0, 2)]);
    let one = alternatives(
        &network,
        NodeId::new(1),
        NodeId::new(1),
        &DistanceCost,
        &RoutingContext::new(),
        AlternativeRouteOptions::default(),
    )
    .unwrap_or_else(|error| panic!("same-node failed: {error}"));
    assert_eq!(one.routes.len(), 1);
    assert_eq!(one.routes[0].route.total_distance(), Meters::ZERO);
    let limited = alternatives(
        &network,
        NodeId::new(0),
        NodeId::new(2),
        &DistanceCost,
        &RoutingContext::new(),
        AlternativeRouteOptions {
            max_search_states: 1,
            ..AlternativeRouteOptions::default()
        },
    );
    assert!(matches!(limited, Err(RoutingError::AlternativeSearchLimit)));
    let invalid = alternatives(
        &network,
        NodeId::new(0),
        NodeId::new(2),
        &DistanceCost,
        &RoutingContext::new(),
        AlternativeRouteOptions {
            max_shared_distance_ratio: 1.0,
            ..AlternativeRouteOptions::default()
        },
    );
    assert!(matches!(
        invalid,
        Err(RoutingError::InvalidAlternativeOptions { .. })
    ));

    let weighted = WeightedEvaluator {
        weights: BTreeMap::from([
            (edge_between(&network, 0, 2), 1.0),
            (edge_between(&network, 0, 1), 1.0),
            (edge_between(&network, 1, 2), 1.0),
        ]),
        forbidden: None,
        fail: None,
    };
    let partial = alternatives(
        &network,
        NodeId::new(0),
        NodeId::new(2),
        &weighted,
        &RoutingContext::new(),
        AlternativeRouteOptions {
            max_search_states: 3,
            ..AlternativeRouteOptions::default()
        },
    )
    .unwrap_or_else(|error| panic!("partial result failed: {error}"));
    assert_eq!(partial.routes.len(), 1);
    assert!(partial.truncated);
    assert_eq!(partial.termination, AlternativeTermination::Budget);

    let capped = alternatives(
        &network,
        NodeId::new(0),
        NodeId::new(2),
        &weighted,
        &RoutingContext::new(),
        AlternativeRouteOptions {
            max_cost_factor: 1.0,
            ..AlternativeRouteOptions::default()
        },
    )
    .unwrap_or_else(|error| panic!("cost-capped result failed: {error}"));
    assert_eq!(capped.routes.len(), 1);
    assert!(!capped.truncated);
    assert_eq!(capped.termination, AlternativeTermination::CostLimit);
}

fn canonical(id: u32) -> CanonicalCoordinate {
    match CanonicalCoordinate::new(0, i32::try_from(id).unwrap_or(i32::MAX) * 10_000) {
        Ok(value) => value,
        Err(error) => panic!("invalid fixture coordinate: {error}"),
    }
}

fn properties() -> EdgeProperties {
    let Ok(speed) = KilometersPerHour::new(36.0) else {
        panic!("valid speed");
    };
    EdgeProperties::new(speed, None, AccessClass::General)
}

fn properties_with_access(access: AccessClass) -> EdgeProperties {
    let Ok(speed) = KilometersPerHour::new(36.0) else {
        panic!("valid speed");
    };
    EdgeProperties::new(speed, None, access)
}

fn graph(node_count: u32, segments: &[(u64, u32, u32)]) -> FrozenGraph {
    let mut builder = GraphBuilder::new(
        GraphSnapshotId::new(7),
        GraphMetadata::new(
            "test_v1",
            "test",
            GraphBuildIdentity::new("fixture", "fixture-sha256", "test-v1", "default"),
        ),
    );
    for id in 0..node_count {
        assert!(
            builder
                .add_node(BuilderNodeId::new(u64::from(id)), canonical(id))
                .is_ok()
        );
    }
    for (key, from, to) in segments {
        assert!(
            builder
                .add_segment(
                    BuilderSegmentId::new(*key),
                    BuilderNodeId::new(u64::from(*from)),
                    BuilderNodeId::new(u64::from(*to)),
                    vec![canonical(*from), canonical(*to)],
                    Some(properties()),
                    None,
                )
                .is_ok()
        );
    }
    match builder.finalize() {
        Ok(value) => value,
        Err(error) => panic!("fixture failed: {error}"),
    }
}

#[derive(Debug)]
struct WeightedEvaluator {
    weights: BTreeMap<EdgeId, f64>,
    forbidden: Option<EdgeId>,
    fail: Option<EdgeId>,
}

impl TraversalEvaluator for WeightedEvaluator {
    fn kind(&self) -> CostKind {
        CostKind::TravelTime
    }
    fn capability(&self) -> SearchCapability {
        SearchCapability::StaticNonNegative
    }
    fn evaluate(
        &self,
        edge: &DirectedEdge,
        _segment: &RoadSegment,
        _state: TraversalState,
        _context: &RoutingContext,
    ) -> Result<TraversalEvaluation, CostError> {
        if self.forbidden == Some(edge.id()) {
            return Ok(TraversalEvaluation::Forbidden);
        }
        if self.fail == Some(edge.id()) {
            return Err(CostError::NotFinite {
                kind: CostKind::TravelTime,
                value: f64::NAN,
            });
        }
        let value = self.weights.get(&edge.id()).copied().unwrap_or(1.0);
        let cost = RouteCost::new(CostKind::TravelTime, value)?;
        let time = Seconds::new(value).map_err(|_| CostError::NotFinite {
            kind: CostKind::TravelTime,
            value,
        })?;
        Ok(TraversalEvaluation::Traversable {
            objective_cost: cost,
            travel_time: time,
        })
    }
}

fn edge_between(graph: &FrozenGraph, from: u32, to: u32) -> EdgeId {
    let Ok(edges) = graph.outgoing_edges(NodeId::new(from)) else {
        panic!("valid node");
    };
    match edges.iter().find(|edge| edge.to() == NodeId::new(to)) {
        Some(edge) => edge.id(),
        None => panic!("missing edge {from}->{to}"),
    }
}

fn bellman_ford(
    graph: &FrozenGraph,
    source: NodeId,
    destination: NodeId,
    weights: &BTreeMap<EdgeId, f64>,
    forbidden: Option<EdgeId>,
) -> Option<f64> {
    let mut distances = vec![f64::INFINITY; graph.node_count()];
    distances[source.value() as usize] = 0.0;
    for _ in 1..graph.node_count() {
        let mut changed = false;
        for edge in graph.edges() {
            if forbidden == Some(edge.id()) {
                continue;
            }
            let from = edge.from().value() as usize;
            let to = edge.to().value() as usize;
            let candidate = distances[from] + weights.get(&edge.id()).copied().unwrap_or(1.0);
            if candidate < distances[to] {
                distances[to] = candidate;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let result = distances[destination.value() as usize];
    result.is_finite().then_some(result)
}

fn validate_route(
    graph: &FrozenGraph,
    source: NodeId,
    destination: NodeId,
    route: &roadrunner_core::routing::RouteResult,
    weights: &BTreeMap<EdgeId, f64>,
) {
    assert_eq!(route.graph_snapshot_id(), graph.snapshot_id());
    assert_eq!(route.path().first(), Some(&source));
    assert_eq!(route.path().last(), Some(&destination));
    assert_eq!(route.path().len(), route.edges().len() + 1);
    let mut objective = 0.0;
    let mut distance = Meters::ZERO;
    for (index, edge_id) in route.edges().iter().enumerate() {
        let Some(edge) = graph.edge(*edge_id) else {
            panic!("route edge missing");
        };
        assert_eq!(edge.from(), route.path()[index]);
        assert_eq!(edge.to(), route.path()[index + 1]);
        objective += weights.get(edge_id).copied().unwrap_or(1.0);
        let Some(segment) = graph.segment(edge.segment()) else {
            panic!("route segment missing");
        };
        distance = match distance.checked_add(segment.distance()) {
            Ok(value) => value,
            Err(error) => panic!("distance failed: {error}"),
        };
    }
    assert_eq!(route.total_cost().value(), objective);
    assert_eq!(route.elapsed_travel_time().value(), objective);
    assert_eq!(route.total_distance(), distance);
}

fn generated(seed: u64) -> (FrozenGraph, BTreeMap<EdgeId, f64>) {
    let node_count = 12_u32;
    let mut segments = Vec::new();
    for id in 0..(node_count - 1) {
        segments.push((u64::from(id), id, id + 1));
    }
    let mut state = seed;
    for key in 100..145_u64 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let from = u32::try_from(state % u64::from(node_count)).unwrap_or(0);
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let to = u32::try_from(state % u64::from(node_count)).unwrap_or(0);
        segments.push((key, from, to));
    }
    let graph = graph(node_count, &segments);
    let weights = graph
        .edges()
        .iter()
        .map(|edge| (edge.id(), f64::from(edge.segment().value() % 9)))
        .collect();
    (graph, weights)
}

#[test]
fn seeded_dijkstra_matches_independent_bellman_ford_and_zero_astar() {
    for seed in [1_u64, 7, 19, 42, 9_001] {
        let (graph, weights) = generated(seed);
        let evaluator = WeightedEvaluator {
            weights: weights.clone(),
            forbidden: None,
            fail: None,
        };
        for destination in 0..12_u32 {
            let expected = bellman_ford(
                &graph,
                NodeId::new(0),
                NodeId::new(destination),
                &weights,
                None,
            );
            let dijkstra_result = dijkstra(
                &graph,
                NodeId::new(0),
                NodeId::new(destination),
                &evaluator,
                &RoutingContext::new(),
            );
            let astar_result = astar(
                &graph,
                NodeId::new(0),
                NodeId::new(destination),
                &evaluator,
                &ZeroHeuristic::new(CostKind::TravelTime),
                &RoutingContext::new(),
            );
            match (expected, dijkstra_result, astar_result) {
                (Some(cost), Ok(dijkstra_route), Ok(astar_route)) => {
                    assert_eq!(dijkstra_route.total_cost().value(), cost);
                    assert_eq!(astar_route.total_cost(), dijkstra_route.total_cost());
                    validate_route(
                        &graph,
                        NodeId::new(0),
                        NodeId::new(destination),
                        &dijkstra_route,
                        &weights,
                    );
                    validate_route(
                        &graph,
                        NodeId::new(0),
                        NodeId::new(destination),
                        &astar_route,
                        &weights,
                    );
                }
                (None, Err(RoutingError::NoRoute { .. }), Err(RoutingError::NoRoute { .. })) => {}
                outcome => panic!("oracle disagreement for seed {seed}: {outcome:?}"),
            }
        }
    }
}

#[test]
fn maneuver_aware_search_keeps_distinct_incoming_edge_states() {
    let unrestricted = graph(4, &[(0, 0, 2), (1, 0, 1), (2, 1, 2), (3, 2, 3)]);
    let direct = edge_between(&unrestricted, 0, 2);
    let exit = edge_between(&unrestricted, 2, 3);
    let graph = match unrestricted.with_forbidden_maneuvers(vec![(direct, exit)]) {
        Ok(value) => value,
        Err(error) => panic!("maneuver fixture failed: {error}"),
    };
    let weights = BTreeMap::from([
        (direct, 1.0),
        (edge_between(&graph, 0, 1), 2.0),
        (edge_between(&graph, 1, 2), 2.0),
        (exit, 1.0),
    ]);
    let evaluator = WeightedEvaluator {
        weights,
        forbidden: None,
        fail: None,
    };
    let dijkstra_route = match dijkstra(
        &graph,
        NodeId::new(0),
        NodeId::new(3),
        &evaluator,
        &RoutingContext::new(),
    ) {
        Ok(value) => value,
        Err(error) => panic!("expanded Dijkstra failed: {error}"),
    };
    let astar_route = match astar(
        &graph,
        NodeId::new(0),
        NodeId::new(3),
        &evaluator,
        &ZeroHeuristic::new(CostKind::TravelTime),
        &RoutingContext::new(),
    ) {
        Ok(value) => value,
        Err(error) => panic!("expanded A* failed: {error}"),
    };
    assert_eq!(
        dijkstra_route.path(),
        &[
            NodeId::new(0),
            NodeId::new(1),
            NodeId::new(2),
            NodeId::new(3)
        ]
    );
    assert_eq!(dijkstra_route.path(), astar_route.path());
    assert_eq!(dijkstra_route.total_cost(), astar_route.total_cost());
    let encoded = match encode_graph_artifact(&graph) {
        Ok(value) => value,
        Err(error) => panic!("maneuver graph encoding failed: {error}"),
    };
    let decoded = match decode_graph_artifact(&encoded) {
        Ok(value) => value,
        Err(error) => panic!("maneuver graph decoding failed: {error}"),
    };
    assert!(decoded.metadata().turn_restrictions_enforced());
    assert_eq!(decoded.forbidden_maneuvers(), &[(direct, exit)]);
}

#[test]
fn built_in_distance_heuristic_matches_dijkstra() {
    let (graph, _) = generated(73);
    for destination in 1..12_u32 {
        let dijkstra_route = dijkstra(
            &graph,
            NodeId::new(0),
            NodeId::new(destination),
            &DistanceCost,
            &RoutingContext::new(),
        );
        let astar_route = astar(
            &graph,
            NodeId::new(0),
            NodeId::new(destination),
            &DistanceCost,
            &DistanceHaversine,
            &RoutingContext::new(),
        );
        assert_eq!(
            dijkstra_route
                .as_ref()
                .map(roadrunner_core::routing::RouteResult::total_cost),
            astar_route
                .as_ref()
                .map(roadrunner_core::routing::RouteResult::total_cost)
        );
    }
}

#[test]
fn built_in_travel_time_heuristic_matches_dijkstra() {
    let (graph, _) = generated(91);
    let Ok(maximum_speed) = KilometersPerHour::new(36.0) else {
        panic!("valid maximum speed");
    };
    let Ok(heuristic) = TravelTimeHaversine::for_graph(&graph, maximum_speed) else {
        panic!("valid heuristic configuration");
    };
    for destination in 1..12_u32 {
        let dijkstra_route = dijkstra(
            &graph,
            NodeId::new(0),
            NodeId::new(destination),
            &TravelTimeCost,
            &RoutingContext::new(),
        );
        let astar_route = astar(
            &graph,
            NodeId::new(0),
            NodeId::new(destination),
            &TravelTimeCost,
            &heuristic,
            &RoutingContext::new(),
        );
        assert_eq!(
            dijkstra_route
                .as_ref()
                .map(roadrunner_core::routing::RouteResult::total_cost),
            astar_route
                .as_ref()
                .map(roadrunner_core::routing::RouteResult::total_cost)
        );
    }
    assert!(TravelTimeHaversine::for_graph(&graph, KilometersPerHour::ZERO).is_err());
}

#[test]
fn astar_geographic_rounding_regression_matches_dijkstra_exactly() {
    let node_count = 1_000_u32;
    let mut segments = Vec::new();
    let mut key = 0_u64;
    for from in 0..node_count {
        for offset in 1..=3_u32 {
            let Some(to) = from.checked_add(offset).filter(|to| *to < node_count) else {
                continue;
            };
            segments.push((key, from, to));
            key += 1;
        }
    }
    let graph = graph(node_count, &segments);
    for (source, destination) in [(0, 999), (13, 23), (0, 500)] {
        let dijkstra_route = dijkstra(
            &graph,
            NodeId::new(source),
            NodeId::new(destination),
            &DistanceCost,
            &RoutingContext::new(),
        );
        let astar_route = astar(
            &graph,
            NodeId::new(source),
            NodeId::new(destination),
            &DistanceCost,
            &DistanceHaversine,
            &RoutingContext::new(),
        );
        assert_eq!(
            dijkstra_route
                .as_ref()
                .map(roadrunner_core::routing::RouteResult::total_cost),
            astar_route
                .as_ref()
                .map(roadrunner_core::routing::RouteResult::total_cost)
        );
    }
}

#[test]
fn stale_entries_parallel_edges_self_loops_and_zero_cycles_are_safe() {
    let graph = graph(
        4,
        &[
            (1, 0, 1),
            (2, 0, 1),
            (3, 0, 2),
            (4, 2, 1),
            (5, 1, 1),
            (6, 1, 2),
            (7, 1, 3),
        ],
    );
    let mut weights = BTreeMap::new();
    for edge in graph.edges() {
        let weight = match (
            edge.from().value(),
            edge.to().value(),
            edge.segment().value(),
        ) {
            (0, 1, 0) => 10.0,
            (0, 1, 1) => 7.0,
            (0, 2, _) | (2, 1, _) | (1, 3, _) => 1.0,
            (1, 1 | 2, _) => 0.0,
            _ => 50.0,
        };
        weights.insert(edge.id(), weight);
    }
    let evaluator = WeightedEvaluator {
        weights: weights.clone(),
        forbidden: None,
        fail: None,
    };
    let route = dijkstra(
        &graph,
        NodeId::new(0),
        NodeId::new(3),
        &evaluator,
        &RoutingContext::new(),
    );
    let Ok(route) = route else {
        panic!("expected route: {route:?}");
    };
    assert_eq!(route.total_cost().value(), 3.0);
    validate_route(&graph, NodeId::new(0), NodeId::new(3), &route, &weights);
}

#[test]
fn forbidden_edges_are_skipped_but_evaluator_errors_propagate() {
    let graph = graph(3, &[(1, 0, 1), (2, 1, 2), (3, 0, 2)]);
    let direct = edge_between(&graph, 0, 2);
    let weights: BTreeMap<EdgeId, f64> = graph
        .edges()
        .iter()
        .map(|edge| (edge.id(), if edge.id() == direct { 1.0 } else { 2.0 }))
        .collect();
    let forbidden = WeightedEvaluator {
        weights: weights.clone(),
        forbidden: Some(direct),
        fail: None,
    };
    let route = dijkstra(
        &graph,
        NodeId::new(0),
        NodeId::new(2),
        &forbidden,
        &RoutingContext::new(),
    );
    assert_eq!(
        route.as_ref().map(|value| value.total_cost().value()),
        Ok(4.0)
    );
    let failure = WeightedEvaluator {
        weights,
        forbidden: None,
        fail: Some(direct),
    };
    assert!(matches!(
        dijkstra(
            &graph,
            NodeId::new(0),
            NodeId::new(2),
            &failure,
            &RoutingContext::new()
        ),
        Err(RoutingError::TraversalEvaluation { .. })
    ));
}

#[test]
fn built_in_evaluators_deny_reason_specific_access_by_default() {
    let coordinates = [
        CanonicalCoordinate::new(0, 0).unwrap_or_else(|error| panic!("coordinate: {error}")),
        CanonicalCoordinate::new(20_000, 10_000)
            .unwrap_or_else(|error| panic!("coordinate: {error}")),
        CanonicalCoordinate::new(0, 20_000).unwrap_or_else(|error| panic!("coordinate: {error}")),
    ];
    let mut builder = GraphBuilder::new(
        GraphSnapshotId::new(12),
        GraphMetadata::new(
            "test_v1",
            "test",
            GraphBuildIdentity::new("fixture", "fixture-sha256", "test-v1", "default"),
        ),
    );
    for (index, coordinate) in coordinates.into_iter().enumerate() {
        assert!(
            builder
                .add_node(BuilderNodeId::new(index as u64), coordinate)
                .is_ok()
        );
    }
    for (id, from, to, access) in [
        (1, 0, 2, AccessClass::Private),
        (2, 0, 1, AccessClass::General),
        (3, 1, 2, AccessClass::General),
    ] {
        let from_index =
            usize::try_from(from).unwrap_or_else(|error| panic!("fixture source index: {error}"));
        let to_index =
            usize::try_from(to).unwrap_or_else(|error| panic!("fixture target index: {error}"));
        assert!(
            builder
                .add_segment(
                    BuilderSegmentId::new(id),
                    BuilderNodeId::new(from),
                    BuilderNodeId::new(to),
                    vec![coordinates[from_index], coordinates[to_index]],
                    Some(properties_with_access(access)),
                    None,
                )
                .is_ok()
        );
    }
    let graph = builder
        .finalize()
        .unwrap_or_else(|error| panic!("fixture graph: {error}"));
    let ordinary = dijkstra(
        &graph,
        NodeId::new(0),
        NodeId::new(2),
        &DistanceCost,
        &RoutingContext::new(),
    )
    .unwrap_or_else(|error| panic!("public alternative: {error}"));
    assert_eq!(
        ordinary.path(),
        [NodeId::new(0), NodeId::new(1), NodeId::new(2)]
    );
    let authorized = dijkstra(
        &graph,
        NodeId::new(0),
        NodeId::new(2),
        &DistanceCost,
        &RoutingContext::new().with_private_access(),
    )
    .unwrap_or_else(|error| panic!("authorized private route: {error}"));
    assert_eq!(authorized.path(), [NodeId::new(0), NodeId::new(2)]);
}

#[test]
fn objective_overflow_is_an_error() {
    let graph = graph(3, &[(1, 0, 1), (2, 1, 2)]);
    let weights = graph
        .edges()
        .iter()
        .map(|edge| (edge.id(), f64::MAX))
        .collect();
    let evaluator = WeightedEvaluator {
        weights,
        forbidden: None,
        fail: None,
    };
    assert!(matches!(
        dijkstra(
            &graph,
            NodeId::new(0),
            NodeId::new(2),
            &evaluator,
            &RoutingContext::new()
        ),
        Err(RoutingError::CostAccumulation { .. })
    ));
}

#[test]
fn invalid_evaluator_values_are_routing_errors() {
    let graph = graph(2, &[(1, 0, 1)]);
    let edge = edge_between(&graph, 0, 1);
    for value in [-1.0, f64::NAN, f64::INFINITY] {
        let evaluator = WeightedEvaluator {
            weights: BTreeMap::from([(edge, value)]),
            forbidden: None,
            fail: None,
        };
        assert!(matches!(
            dijkstra(
                &graph,
                NodeId::new(0),
                NodeId::new(1),
                &evaluator,
                &RoutingContext::new()
            ),
            Err(RoutingError::TraversalEvaluation { .. })
        ));
    }
}

#[test]
fn disconnected_and_isolated_nodes_return_precise_outcomes() {
    let graph = graph(4, &[(1, 0, 1)]);
    assert!(matches!(
        dijkstra(
            &graph,
            NodeId::new(0),
            NodeId::new(3),
            &DistanceCost,
            &RoutingContext::new()
        ),
        Err(RoutingError::NoRoute { .. })
    ));
    let isolated = dijkstra(
        &graph,
        NodeId::new(2),
        NodeId::new(2),
        &DistanceCost,
        &RoutingContext::new(),
    );
    assert!(isolated.is_ok_and(|route| route.edges().is_empty() && route.path().len() == 1));
}

#[test]
fn graph_builder_rejects_malformed_segments() {
    let metadata = GraphMetadata::new(
        "test_v1",
        "test",
        GraphBuildIdentity::new("fixture", "fixture-sha256", "test-v1", "default"),
    );
    let mut builder = GraphBuilder::new(GraphSnapshotId::new(8), metadata);
    assert!(
        builder
            .add_node(BuilderNodeId::new(0), canonical(0))
            .is_ok()
    );
    assert!(
        builder
            .add_node(BuilderNodeId::new(1), canonical(1))
            .is_ok()
    );
    assert!(
        builder
            .add_segment(
                BuilderSegmentId::new(1),
                BuilderNodeId::new(0),
                BuilderNodeId::new(9),
                vec![canonical(0), canonical(1)],
                Some(properties()),
                None,
            )
            .is_err()
    );
    assert!(
        builder
            .add_segment(
                BuilderSegmentId::new(2),
                BuilderNodeId::new(0),
                BuilderNodeId::new(1),
                vec![canonical(1), canonical(0)],
                Some(properties()),
                None,
            )
            .is_err()
    );
    let zero_speed = EdgeProperties::new(KilometersPerHour::ZERO, None, AccessClass::General);
    assert!(
        builder
            .add_segment(
                BuilderSegmentId::new(3),
                BuilderNodeId::new(0),
                BuilderNodeId::new(1),
                vec![canonical(0), canonical(1)],
                Some(zero_speed),
                None,
            )
            .is_err()
    );
}

#[derive(Debug)]
struct UnsupportedEvaluator;
impl TraversalEvaluator for UnsupportedEvaluator {
    fn kind(&self) -> CostKind {
        CostKind::TravelTime
    }
    fn capability(&self) -> SearchCapability {
        SearchCapability::RequiresExpandedState
    }
    fn evaluate(
        &self,
        _edge: &DirectedEdge,
        _segment: &RoadSegment,
        _state: TraversalState,
        _context: &RoutingContext,
    ) -> Result<TraversalEvaluation, CostError> {
        Ok(TraversalEvaluation::Forbidden)
    }
}

#[derive(Debug)]
struct NonFifoEvaluator;
impl TraversalEvaluator for NonFifoEvaluator {
    fn kind(&self) -> CostKind {
        CostKind::TravelTime
    }
    fn capability(&self) -> SearchCapability {
        SearchCapability::Unsupported
    }
    fn evaluate(
        &self,
        _edge: &DirectedEdge,
        _segment: &RoadSegment,
        _state: TraversalState,
        _context: &RoutingContext,
    ) -> Result<TraversalEvaluation, CostError> {
        Ok(TraversalEvaluation::Forbidden)
    }
}

#[test]
fn incompatible_capabilities_and_heuristics_are_rejected() {
    let graph = graph(2, &[(1, 0, 1)]);
    assert!(matches!(
        dijkstra(
            &graph,
            NodeId::new(0),
            NodeId::new(1),
            &UnsupportedEvaluator,
            &RoutingContext::new()
        ),
        Err(RoutingError::UnsupportedCapability { .. })
    ));
    assert!(matches!(
        astar(
            &graph,
            NodeId::new(0),
            NodeId::new(1),
            &DistanceCost,
            &ZeroHeuristic::new(CostKind::TravelTime),
            &RoutingContext::new()
        ),
        Err(RoutingError::HeuristicKindMismatch { .. })
    ));
    assert!(matches!(
        dijkstra(
            &graph,
            NodeId::new(0),
            NodeId::new(1),
            &NonFifoEvaluator,
            &RoutingContext::new()
        ),
        Err(RoutingError::UnsupportedCapability {
            capability: SearchCapability::Unsupported
        })
    ));
    let Ok(maximum_speed) = KilometersPerHour::new(36.0) else {
        panic!("valid maximum speed");
    };
    let Ok(time_heuristic) = TravelTimeHaversine::for_graph(&graph, maximum_speed) else {
        panic!("valid graph speed bound");
    };
    let custom_time = WeightedEvaluator {
        weights: BTreeMap::new(),
        forbidden: None,
        fail: None,
    };
    assert!(matches!(
        astar(
            &graph,
            NodeId::new(0),
            NodeId::new(1),
            &custom_time,
            &time_heuristic,
            &RoutingContext::new()
        ),
        Err(RoutingError::HeuristicPolicyMismatch { .. })
    ));
    let Ok(unsafe_speed) = KilometersPerHour::new(35.0) else {
        panic!("valid but insufficient speed");
    };
    assert!(TravelTimeHaversine::for_graph(&graph, unsafe_speed).is_err());
}

#[derive(Debug)]
struct FifoEvaluator;
impl TraversalEvaluator for FifoEvaluator {
    fn kind(&self) -> CostKind {
        CostKind::TravelTime
    }
    fn capability(&self) -> SearchCapability {
        SearchCapability::FifoEarliestArrival
    }
    fn evaluate(
        &self,
        edge: &DirectedEdge,
        _segment: &RoadSegment,
        state: TraversalState,
        context: &RoutingContext,
    ) -> Result<TraversalEvaluation, CostError> {
        let arrival = context.departure_time().value() + state.elapsed_travel_time().value();
        let value = match (edge.from().value(), edge.to().value()) {
            (0, 1) => 5.0,
            (1, 2) if arrival <= 5.0 => 1.0,
            (1, 2) => 100.0,
            (0, 2) => 20.0,
            _ => 1.0,
        };
        let time = Seconds::new(value).map_err(|_| CostError::NotFinite {
            kind: CostKind::TravelTime,
            value,
        })?;
        Ok(TraversalEvaluation::Traversable {
            objective_cost: RouteCost::from_travel_time(time),
            travel_time: time,
        })
    }
}

#[test]
fn fifo_evaluation_receives_arrival_time_at_each_node() {
    let graph = graph(3, &[(1, 0, 1), (2, 1, 2), (3, 0, 2)]);
    let result = dijkstra(
        &graph,
        NodeId::new(0),
        NodeId::new(2),
        &FifoEvaluator,
        &RoutingContext::new(),
    );
    assert_eq!(
        result
            .as_ref()
            .map(|route| route.elapsed_travel_time().value()),
        Ok(6.0)
    );
}

#[test]
fn finalization_is_independent_of_insertion_order_and_artifact_round_trips() {
    fn build(reverse: bool) -> FrozenGraph {
        let mut builder = GraphBuilder::new(
            GraphSnapshotId::new(99),
            GraphMetadata::new(
                "test_v1",
                "test",
                GraphBuildIdentity::new("fixture", "fixture-sha256", "test-v1", "default"),
            ),
        );
        let nodes = if reverse {
            vec![2, 1, 0]
        } else {
            vec![0, 1, 2]
        };
        for id in nodes {
            assert!(
                builder
                    .add_node(
                        BuilderNodeId::new(id),
                        canonical(u32::try_from(id).unwrap_or(0))
                    )
                    .is_ok()
            );
        }
        let segments = if reverse {
            vec![(20, 1, 2), (10, 0, 1)]
        } else {
            vec![(10, 0, 1), (20, 1, 2)]
        };
        for (key, from, to) in segments {
            assert!(
                builder
                    .add_segment(
                        BuilderSegmentId::new(key),
                        BuilderNodeId::new(from),
                        BuilderNodeId::new(to),
                        vec![
                            canonical(u32::try_from(from).unwrap_or(0)),
                            canonical(u32::try_from(to).unwrap_or(0))
                        ],
                        Some(properties()),
                        None
                    )
                    .is_ok()
            );
        }
        match builder.finalize() {
            Ok(value) => value,
            Err(error) => panic!("finalization failed: {error}"),
        }
    }
    let first = build(false);
    let second = build(true);
    let first_bytes = encode_graph_artifact(&first);
    let second_bytes = encode_graph_artifact(&second);
    assert_eq!(first_bytes.as_ref().ok(), second_bytes.as_ref().ok());
    let Ok(bytes) = first_bytes else {
        panic!("encoding failed");
    };
    let decoded = decode_graph_artifact(&bytes);
    let Ok(decoded) = decoded else {
        panic!("decoding failed: {decoded:?}");
    };
    assert_eq!(decoded.node_count(), first.node_count());
    assert_eq!(decoded.edge_count(), first.edge_count());
}

#[test]
fn corrupt_artifacts_are_rejected() {
    let graph = graph(2, &[(1, 0, 1)]);
    let Ok(mut bytes) = encode_graph_artifact(&graph) else {
        panic!("encoding failed");
    };
    let middle = bytes.len() / 2;
    bytes[middle] ^= 1;
    assert!(decode_graph_artifact(&bytes).is_err());
    let Ok(bytes) = encode_graph_artifact(&graph) else {
        panic!("encoding failed");
    };
    assert!(decode_graph_artifact(&bytes[..bytes.len() / 2]).is_err());
    let text = String::from_utf8_lossy(&bytes).replace("ROADRUNNER_GRAPH", "ROADRUNNER_WRONG");
    assert!(decode_graph_artifact(text.as_bytes()).is_err());
}

#[test]
fn atomic_artifact_publication_produces_a_valid_complete_file() {
    let graph = graph(2, &[(1, 0, 1)]);
    let directory =
        std::env::temp_dir().join(format!("roadrunner-artifact-test-{}", std::process::id()));
    assert!(std::fs::create_dir_all(&directory).is_ok());
    let path = directory.join("graph.rrg");
    assert!(write_graph_artifact_atomic(&path, &graph).is_ok());
    let bytes = std::fs::read(&path);
    let Ok(bytes) = bytes else {
        panic!("published artifact should be readable");
    };
    assert!(decode_graph_artifact(&bytes).is_ok());
    assert!(std::fs::remove_file(&path).is_ok());
    assert!(std::fs::remove_dir(&directory).is_ok());
}
