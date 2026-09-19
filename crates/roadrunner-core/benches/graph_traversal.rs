//! Frozen adjacency traversal benchmark.

mod support;

use std::collections::VecDeque;
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion};
use roadrunner_core::graph::{FrozenGraph, NodeId};

fn breadth_first_count(graph: &FrozenGraph) -> usize {
    let mut seen = vec![false; graph.node_count()];
    let mut queue = VecDeque::from([NodeId::new(0)]);
    seen[0] = true;
    while let Some(node) = queue.pop_front() {
        let Ok(edges) = graph.outgoing_edges(node) else {
            return 0;
        };
        for edge in edges {
            let target = edge.to().value() as usize;
            if !seen[target] {
                seen[target] = true;
                queue.push_back(edge.to());
            }
        }
    }
    seen.into_iter().filter(|value| *value).count()
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    let mut group = criterion.benchmark_group("frozen_graph_traversal");
    group.sample_size(20);
    for node_count in [1_000_u32, 10_000, 100_000] {
        let graph = support::generated_graph(node_count, 3);
        group.bench_with_input(
            BenchmarkId::new("breadth_first", node_count),
            &graph,
            |bencher, graph| bencher.iter(|| black_box(breadth_first_count(black_box(graph)))),
        );
    }
    group.finish();
    criterion.final_summary();
}
