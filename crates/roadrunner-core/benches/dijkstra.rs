//! Revised-core Dijkstra baseline over deterministic frozen graphs.

mod support;

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput};
use roadrunner_core::cost::{DistanceCost, RoutingContext};
use roadrunner_core::routing::{RouteResult, dijkstra};

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    let mut group = criterion.benchmark_group("dijkstra_revised_core");
    group.sample_size(20);
    for node_count in [1_000_u32, 10_000, 100_000] {
        let graph = support::generated_graph(node_count, 3);
        let corpus = support::query_corpus(node_count, 0x5EED);
        let expanded: usize = corpus
            .iter()
            .filter_map(|query| {
                dijkstra(
                    &graph,
                    query.source,
                    query.destination,
                    &DistanceCost,
                    &RoutingContext::new(),
                )
                .ok()
            })
            .map(|route| RouteResult::expanded_states(&route))
            .sum();
        group.throughput(Throughput::Elements(u64::from(node_count)));
        group.bench_with_input(
            BenchmarkId::new(
                "distance",
                format!("{node_count}_nodes_{expanded}_expanded"),
            ),
            &(&graph, &corpus),
            |bencher, (graph, corpus)| {
                bencher.iter(|| {
                    for query in *corpus {
                        let _ = black_box(dijkstra(
                            black_box(graph),
                            query.source,
                            query.destination,
                            &DistanceCost,
                            &RoutingContext::new(),
                        ));
                    }
                });
            },
        );
    }
    group.finish();
    criterion.final_summary();
}
