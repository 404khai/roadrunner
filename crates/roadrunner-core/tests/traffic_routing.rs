//! Phase 10 traffic-aware routing correctness checks.
#![allow(clippy::float_cmp)]

use roadrunner_core::cost::{
    DistanceCost, RoutingContext, TrafficAwareCost, TrafficError, TrafficLevel, TrafficMultiplier,
    TrafficSnapshot, TravelTimeCost,
};
use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour};
use roadrunner_core::graph::{
    AccessClass, BuilderNodeId, BuilderSegmentId, EdgeId, EdgeProperties, FrozenGraph,
    GraphBuildIdentity, GraphBuilder, GraphMetadata, GraphSnapshotId, NodeId,
};
use roadrunner_core::routing::{
    AlternativeRouteOptions, RoutingError, TravelTimeHaversine, alternatives, astar, dijkstra,
};

fn coordinate(lat: i32, lon: i32) -> CanonicalCoordinate {
    CanonicalCoordinate::new(lat, lon).unwrap_or_else(|error| panic!("fixture coordinate: {error}"))
}

fn network(snapshot_id: u64) -> FrozenGraph {
    let mut builder = GraphBuilder::new(
        GraphSnapshotId::new(snapshot_id),
        GraphMetadata::new(
            "traffic_test",
            "test",
            GraphBuildIdentity::new("traffic-fixture", "fixture-digest", "v1", "static"),
        ),
    );
    let points = [
        coordinate(0, 0),
        coordinate(0, 10_000),
        coordinate(10_000, 15_000),
        coordinate(0, 30_000),
    ];
    for (id, point) in points.into_iter().enumerate() {
        builder
            .add_node(BuilderNodeId::new(id as u64), point)
            .unwrap_or_else(|error| panic!("add node: {error}"));
    }
    let speed = KilometersPerHour::new(36.0).unwrap_or_else(|error| panic!("speed: {error}"));
    let properties = EdgeProperties::new(speed, None, AccessClass::General);
    for (id, from, to) in [(0_u64, 0_usize, 1_usize), (1, 1, 3), (2, 0, 2), (3, 2, 3)] {
        builder
            .add_segment(
                BuilderSegmentId::new(id),
                BuilderNodeId::new(from as u64),
                BuilderNodeId::new(to as u64),
                vec![points[from], points[to]],
                Some(properties),
                None,
            )
            .unwrap_or_else(|error| panic!("add segment: {error}"));
    }
    builder
        .finalize()
        .unwrap_or_else(|error| panic!("freeze graph: {error}"))
}

fn edge_between(graph: &FrozenGraph, from: u32, to: u32) -> EdgeId {
    graph
        .outgoing_edges(NodeId::new(from))
        .unwrap_or_else(|error| panic!("outgoing edges: {error}"))
        .iter()
        .find(|edge| edge.to() == NodeId::new(to))
        .unwrap_or_else(|| panic!("missing edge {from}->{to}"))
        .id()
}

#[test]
fn traffic_changes_fastest_route_but_not_shortest_distance() {
    let graph = network(7);
    let origin = NodeId::new(0);
    let destination = NodeId::new(3);
    let context = RoutingContext::new();
    let short = dijkstra(&graph, origin, destination, &DistanceCost, &context)
        .unwrap_or_else(|error| panic!("distance route: {error}"));
    let free_flow = dijkstra(&graph, origin, destination, &TravelTimeCost, &context)
        .unwrap_or_else(|error| panic!("base route: {error}"));
    assert_eq!(short.path(), &[origin, NodeId::new(1), destination]);
    assert_eq!(free_flow.path(), short.path());

    let congested = edge_between(&graph, 1, 3);
    let overlay = TrafficSnapshot::new(&graph, [(congested, TrafficLevel::Severe.multiplier())])
        .unwrap_or_else(|error| panic!("traffic overlay: {error}"));
    let evaluator = TrafficAwareCost::new(&overlay);
    let fastest = dijkstra(&graph, origin, destination, &evaluator, &context)
        .unwrap_or_else(|error| panic!("traffic route: {error}"));
    assert_eq!(fastest.path(), &[origin, NodeId::new(2), destination]);
    assert!(fastest.total_distance() > short.total_distance());
    assert_eq!(
        fastest.total_cost().value(),
        fastest.elapsed_travel_time().value()
    );
    let congested_short_time = graph
        .edge(edge_between(&graph, 0, 1))
        .unwrap_or_else(|| panic!("first route edge missing"))
        .free_flow_travel_time()
        .value()
        + graph
            .edge(congested)
            .unwrap_or_else(|| panic!("congested route edge missing"))
            .free_flow_travel_time()
            .value()
            * TrafficLevel::Severe.multiplier().value();
    assert!(fastest.elapsed_travel_time().value() < congested_short_time);
    assert_eq!(
        graph.metadata().snapshot_digest(),
        overlay.graph_snapshot_digest()
    );

    let heuristic = TravelTimeHaversine::for_graph(
        &graph,
        KilometersPerHour::new(36.0).unwrap_or_else(|error| panic!("speed: {error}")),
    )
    .unwrap_or_else(|error| panic!("heuristic: {error}"));
    let a_star = astar(
        &graph,
        origin,
        destination,
        &evaluator,
        &heuristic,
        &context,
    )
    .unwrap_or_else(|error| panic!("A*: {error}"));
    assert_eq!(a_star.total_cost(), fastest.total_cost());
    let alternatives = alternatives(
        &graph,
        origin,
        destination,
        &evaluator,
        &context,
        AlternativeRouteOptions {
            max_cost_factor: 4.0,
            ..AlternativeRouteOptions::default()
        },
    )
    .unwrap_or_else(|error| panic!("alternatives: {error}"));
    assert_eq!(
        alternatives.routes[0].route.total_cost(),
        fastest.total_cost()
    );
}

#[test]
fn overlay_validates_inputs_and_graph_identity() {
    let graph = network(7);
    assert!(matches!(
        TrafficMultiplier::new(0.9),
        Err(TrafficError::InvalidMultiplier { .. })
    ));
    assert!(matches!(
        TrafficMultiplier::new(f64::NAN),
        Err(TrafficError::InvalidMultiplier { .. })
    ));
    assert!(matches!(
        TrafficMultiplier::new(f64::INFINITY),
        Err(TrafficError::InvalidMultiplier { .. })
    ));
    let edge = edge_between(&graph, 1, 3);
    assert!(matches!(
        TrafficSnapshot::new(
            &graph,
            [
                (edge, TrafficMultiplier::NORMAL),
                (edge, TrafficLevel::Heavy.multiplier())
            ]
        ),
        Err(TrafficError::DuplicateEdge { .. })
    ));
    assert!(matches!(
        TrafficSnapshot::new(
            &graph,
            [(EdgeId::new(999), TrafficLevel::Heavy.multiplier())]
        ),
        Err(TrafficError::UnknownEdge { .. })
    ));
    let first = TrafficSnapshot::new(
        &graph,
        [
            (edge, TrafficLevel::Heavy.multiplier()),
            (
                edge_between(&graph, 0, 1),
                TrafficLevel::Moderate.multiplier(),
            ),
        ],
    )
    .unwrap_or_else(|error| panic!("first overlay: {error}"));
    let second = TrafficSnapshot::new(
        &graph,
        [
            (
                edge_between(&graph, 0, 1),
                TrafficLevel::Moderate.multiplier(),
            ),
            (edge, TrafficLevel::Heavy.multiplier()),
        ],
    )
    .unwrap_or_else(|error| panic!("second overlay: {error}"));
    assert_eq!(
        first.traffic_snapshot_digest(),
        second.traffic_snapshot_digest()
    );
    let empty =
        TrafficSnapshot::new(&graph, []).unwrap_or_else(|error| panic!("empty overlay: {error}"));
    let explicit_normal = TrafficSnapshot::new(&graph, [(edge, TrafficMultiplier::NORMAL)])
        .unwrap_or_else(|error| panic!("normal overlay: {error}"));
    assert_eq!(
        empty.traffic_snapshot_digest(),
        explicit_normal.traffic_snapshot_digest()
    );
    assert_eq!(
        first.multiplier(edge_between(&graph, 0, 2)),
        Some(TrafficMultiplier::NORMAL)
    );
    assert!(serde_json::from_str::<TrafficMultiplier>("0.9").is_err());
    assert_eq!(
        serde_json::from_str::<TrafficMultiplier>("1.6")
            .unwrap_or_else(|error| panic!("deserialize multiplier: {error}")),
        TrafficLevel::Heavy.multiplier()
    );
    let other_graph = network(8);
    assert!(matches!(
        dijkstra(
            &other_graph,
            NodeId::new(0),
            NodeId::new(3),
            &TrafficAwareCost::new(&first),
            &RoutingContext::new()
        ),
        Err(RoutingError::EvaluatorGraphMismatch)
    ));
}

#[test]
fn overflowing_adjusted_time_is_a_routing_error() {
    let graph = network(7);
    let edge = edge_between(&graph, 1, 3);
    let huge = TrafficSnapshot::new(
        &graph,
        [(
            edge,
            TrafficMultiplier::new(f64::MAX)
                .unwrap_or_else(|error| panic!("large factor: {error}")),
        )],
    )
    .unwrap_or_else(|error| panic!("large overlay: {error}"));
    assert!(matches!(
        dijkstra(
            &graph,
            NodeId::new(0),
            NodeId::new(3),
            &TrafficAwareCost::new(&huge),
            &RoutingContext::new()
        ),
        Err(RoutingError::TraversalEvaluation { .. })
    ));
}

#[test]
fn directional_override_does_not_slow_reverse_traversal() {
    let mut builder = GraphBuilder::new(
        GraphSnapshotId::new(9),
        GraphMetadata::new(
            "traffic_test",
            "test",
            GraphBuildIdentity::new("two-way", "fixture-digest", "v1", "static"),
        ),
    );
    let a = coordinate(0, 0);
    let b = coordinate(0, 10_000);
    for (id, point) in [(0_u64, a), (1, b)] {
        builder
            .add_node(BuilderNodeId::new(id), point)
            .unwrap_or_else(|error| panic!("add node: {error}"));
    }
    let speed = KilometersPerHour::new(36.0).unwrap_or_else(|error| panic!("speed: {error}"));
    let properties = EdgeProperties::new(speed, None, AccessClass::General);
    builder
        .add_segment(
            BuilderSegmentId::new(0),
            BuilderNodeId::new(0),
            BuilderNodeId::new(1),
            vec![a, b],
            Some(properties),
            Some(properties),
        )
        .unwrap_or_else(|error| panic!("add segment: {error}"));
    let graph = builder
        .finalize()
        .unwrap_or_else(|error| panic!("freeze graph: {error}"));
    let forward = edge_between(&graph, 0, 1);
    let reverse = edge_between(&graph, 1, 0);
    let overlay = TrafficSnapshot::new(&graph, [(forward, TrafficLevel::Heavy.multiplier())])
        .unwrap_or_else(|error| panic!("overlay: {error}"));
    assert_eq!(
        overlay.multiplier(forward),
        Some(TrafficLevel::Heavy.multiplier())
    );
    assert_eq!(overlay.multiplier(reverse), Some(TrafficMultiplier::NORMAL));
    let evaluator = TrafficAwareCost::new(&overlay);
    let forward_route = dijkstra(
        &graph,
        NodeId::new(0),
        NodeId::new(1),
        &evaluator,
        &RoutingContext::new(),
    )
    .unwrap_or_else(|error| panic!("forward route: {error}"));
    let reverse_route = dijkstra(
        &graph,
        NodeId::new(1),
        NodeId::new(0),
        &evaluator,
        &RoutingContext::new(),
    )
    .unwrap_or_else(|error| panic!("reverse route: {error}"));
    assert!(forward_route.elapsed_travel_time() > reverse_route.elapsed_travel_time());
}
