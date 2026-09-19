//! A* and Dijkstra comparison over deterministic geographic graphs.
//!
//! Criterion records end-to-end search time, including A* heuristic preparation.
//! Graph construction and correctness checks happen outside timed closures.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput};
use roadrunner_core::cost::{DistanceCost, RoutingContext};
use roadrunner_core::geo::{Coordinate, Seconds, haversine_distance};
use roadrunner_core::graph::{Edge, EdgeId, Graph, Node, NodeId};
use roadrunner_core::routing::{RouteResult, astar, dijkstra};

const BACKBONE_DIVISOR: u32 = 10;
const SAMPLE_SIZE: usize = 20;

fn coordinate(latitude: f64, longitude: f64) -> Coordinate {
    match Coordinate::new(latitude, longitude) {
        Ok(coordinate) => coordinate,
        Err(error) => panic!("generated coordinate is invalid: {error}"),
    }
}

fn seconds(value: f64) -> Seconds {
    match Seconds::new(value) {
        Ok(seconds) => seconds,
        Err(error) => panic!("generated duration is invalid: {error}"),
    }
}

fn add_node(graph: &mut Graph, id: u32, coordinate: Coordinate) {
    let result = graph.add_node(Node::new(NodeId::new(id), coordinate));
    assert!(
        result.is_ok(),
        "generated node insertion failed: {result:?}"
    );
}

fn add_edge(graph: &mut Graph, id: u64, from: NodeId, to: NodeId) {
    let from_coordinate = match graph.node(from) {
        Some(node) => node.coordinate(),
        None => panic!("generated source node {from} is missing"),
    };
    let to_coordinate = match graph.node(to) {
        Some(node) => node.coordinate(),
        None => panic!("generated destination node {to} is missing"),
    };
    let distance = haversine_distance(from_coordinate, to_coordinate);
    let edge = Edge::new(
        EdgeId::new(id),
        from,
        to,
        distance,
        seconds(distance.value()),
    );
    let result = graph.add_edge(edge);
    assert!(
        result.is_ok(),
        "generated edge insertion failed: {result:?}"
    );
}

fn generated_graph(node_count: u32) -> (Graph, NodeId) {
    assert!(node_count >= BACKBONE_DIVISOR * 2);
    let backbone_count = node_count / BACKBONE_DIVISOR;
    let destination = NodeId::new(backbone_count - 1);
    let mut graph = Graph::new();

    for id in 0..backbone_count {
        let longitude = f64::from(id) / f64::from(backbone_count - 1);
        add_node(&mut graph, id, coordinate(0.0, longitude));
    }
    for id in backbone_count..node_count {
        add_node(&mut graph, id, coordinate(0.25, 0.0));
    }

    let mut edge_id = 0_u64;
    for from in 0..(backbone_count - 1) {
        add_edge(
            &mut graph,
            edge_id,
            NodeId::new(from),
            NodeId::new(from + 1),
        );
        edge_id += 1;
    }
    for decoy in backbone_count..node_count {
        add_edge(&mut graph, edge_id, NodeId::new(0), NodeId::new(decoy));
        edge_id += 1;
    }

    (graph, destination)
}

fn dijkstra_route(graph: &Graph, destination: NodeId) -> RouteResult {
    let result = dijkstra(
        graph,
        NodeId::new(0),
        destination,
        &DistanceCost,
        &RoutingContext::new(),
    );
    match result {
        Ok(route) => route,
        Err(error) => panic!("generated graph Dijkstra search failed: {error}"),
    }
}

fn astar_route(graph: &Graph, destination: NodeId) -> RouteResult {
    let result = astar(
        graph,
        NodeId::new(0),
        destination,
        &DistanceCost,
        &RoutingContext::new(),
    );
    match result {
        Ok(route) => route,
        Err(error) => panic!("generated graph A* search failed: {error}"),
    }
}

fn astar_comparison(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("astar_comparison");
    group.sample_size(SAMPLE_SIZE);

    for node_count in [1_000_u32, 10_000, 100_000] {
        let (graph, destination) = generated_graph(node_count);
        let dijkstra_result = dijkstra_route(&graph, destination);
        let astar_result = astar_route(&graph, destination);
        assert_eq!(astar_result.total_cost(), dijkstra_result.total_cost());
        assert_eq!(astar_result.path(), dijkstra_result.path());

        group.throughput(Throughput::Elements(u64::from(node_count)));
        let dijkstra_parameter = format!(
            "{node_count}_nodes_{}_visited",
            dijkstra_result.visited_nodes()
        );
        group.bench_with_input(
            BenchmarkId::new("dijkstra_distance", dijkstra_parameter),
            &graph,
            |bencher, graph| {
                bencher.iter(|| black_box(dijkstra_route(black_box(graph), destination)));
            },
        );

        let astar_parameter = format!(
            "{node_count}_nodes_{}_visited",
            astar_result.visited_nodes()
        );
        group.bench_with_input(
            BenchmarkId::new("astar_distance", astar_parameter),
            &graph,
            |bencher, graph| {
                bencher.iter(|| black_box(astar_route(black_box(graph), destination)));
            },
        );
    }

    group.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    astar_comparison(&mut criterion);
    criterion.final_summary();
}
