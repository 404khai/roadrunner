//! Dijkstra baseline over deterministic generated graphs.
//!
//! Criterion records search time, while each benchmark identifier records the
//! graph size and finalized-node count. Portable memory measurement is deferred
//! until the project adopts a controlled allocator or profiler configuration.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput};
use roadrunner_core::cost::{DistanceCost, RoutingContext};
use roadrunner_core::geo::{Coordinate, Meters, Seconds};
use roadrunner_core::graph::{Edge, EdgeId, Graph, Node, NodeId};
use roadrunner_core::routing::{RouteResult, dijkstra};

const FAN_OUT: u32 = 3;
const SAMPLE_SIZE: usize = 20;

fn meters(value: f64) -> Meters {
    match Meters::new(value) {
        Ok(value) => value,
        Err(error) => panic!("generated distance is invalid: {error}"),
    }
}

fn seconds(value: f64) -> Seconds {
    match Seconds::new(value) {
        Ok(value) => value,
        Err(error) => panic!("generated duration is invalid: {error}"),
    }
}

fn generated_graph(node_count: u32) -> Graph {
    assert!(node_count > FAN_OUT);
    let mut graph = Graph::new();

    for value in 0..node_count {
        let result = graph.add_node(Node::new(NodeId::new(value), Coordinate::ORIGIN));
        assert!(
            result.is_ok(),
            "generated node insertion failed: {result:?}"
        );
    }

    let mut edge_value = 0_u64;
    for source in 0..node_count {
        for offset in 1..=FAN_OUT {
            let edge = Edge::new(
                EdgeId::new(edge_value),
                NodeId::new(source),
                NodeId::new((source + offset) % node_count),
                meters(f64::from(offset)),
                seconds(f64::from(offset)),
            );
            let result = graph.add_edge(edge);
            assert!(
                result.is_ok(),
                "generated edge insertion failed: {result:?}"
            );
            edge_value += 1;
        }
    }

    graph
}

fn route(graph: &Graph, destination: NodeId) -> RouteResult {
    let result = dijkstra(
        graph,
        NodeId::new(0),
        destination,
        &DistanceCost,
        &RoutingContext::new(),
    );
    match result {
        Ok(route) => route,
        Err(error) => panic!("generated graph routing failed: {error}"),
    }
}

fn dijkstra_baseline(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("dijkstra_baseline");
    group.sample_size(SAMPLE_SIZE);

    for node_count in [1_000_u32, 10_000, 100_000] {
        let graph = generated_graph(node_count);
        let destination = NodeId::new(node_count - 1);
        let visited_nodes = route(&graph, destination).visited_nodes();
        let parameter = format!("{node_count}_nodes_{visited_nodes}_visited");
        let throughput = match u64::try_from(visited_nodes) {
            Ok(value) => value,
            Err(error) => panic!("visited-node count does not fit in u64: {error}"),
        };
        group.throughput(Throughput::Elements(throughput));
        group.bench_with_input(
            BenchmarkId::new("distance", parameter),
            &graph,
            |bencher, graph| {
                bencher.iter(|| black_box(route(black_box(graph), destination)));
            },
        );
    }

    group.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    dijkstra_baseline(&mut criterion);
    criterion.final_summary();
}
