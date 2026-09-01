//! Generated-graph traversal benchmark for the Phase 2 adjacency list.

use std::collections::{HashSet, VecDeque};
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput};
use roadrunner_core::graph::{Edge, EdgeId, Graph, Node, NodeId};

const FAN_OUT: u32 = 3;
const SAMPLE_SIZE: usize = 20;

fn generated_graph(node_count: u32) -> Graph {
    assert!(node_count > FAN_OUT);
    let mut graph = Graph::new();

    for value in 0..node_count {
        let result = graph.add_node(Node::new(NodeId::new(value)));
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

fn breadth_first_node_count(graph: &Graph, start: NodeId) -> usize {
    let mut visited = HashSet::with_capacity(graph.node_count());
    let mut queue = VecDeque::new();
    visited.insert(start);
    queue.push_back(start);

    while let Some(node_id) = queue.pop_front() {
        let edges = match graph.neighbors(node_id) {
            Ok(edges) => edges,
            Err(error) => panic!("generated graph traversal failed: {error}"),
        };
        for edge in edges {
            if visited.insert(edge.to()) {
                queue.push_back(edge.to());
            }
        }
    }

    visited.len()
}

fn graph_traversal(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("graph_traversal");
    group.sample_size(SAMPLE_SIZE);

    for node_count in [1_000_u32, 10_000, 100_000] {
        let graph = generated_graph(node_count);
        group.throughput(Throughput::Elements(u64::from(node_count)));
        group.bench_with_input(
            BenchmarkId::new("breadth_first", node_count),
            &graph,
            |bencher, graph| {
                bencher
                    .iter(|| black_box(breadth_first_node_count(black_box(graph), NodeId::new(0))));
            },
        );
    }

    group.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    graph_traversal(&mut criterion);
    criterion.final_summary();
}
