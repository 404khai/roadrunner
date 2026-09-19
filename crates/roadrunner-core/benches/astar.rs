//! Revised-core A* and Dijkstra comparison on identical frozen snapshots.

mod support;

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion};
use roadrunner_core::cost::{DistanceCost, RoutingContext};
use roadrunner_core::routing::{DistanceHaversine, astar, dijkstra};

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    let mut group = criterion.benchmark_group("astar_revised_core");
    group.sample_size(20);
    for node_count in [1_000_u32, 10_000, 100_000] {
        let graph = support::generated_graph(node_count, 3);
        let corpus = support::query_corpus(node_count, 0x5EED);
        let mut dijkstra_expanded = 0_usize;
        let mut astar_expanded = 0_usize;
        for query in &corpus {
            let dijkstra_result = dijkstra(
                &graph,
                query.source,
                query.destination,
                &DistanceCost,
                &RoutingContext::new(),
            );
            let astar_result = astar(
                &graph,
                query.source,
                query.destination,
                &DistanceCost,
                &DistanceHaversine,
                &RoutingContext::new(),
            );
            assert_eq!(
                dijkstra_result
                    .as_ref()
                    .map(roadrunner_core::routing::RouteResult::total_cost),
                astar_result
                    .as_ref()
                    .map(roadrunner_core::routing::RouteResult::total_cost)
            );
            dijkstra_expanded += dijkstra_result
                .as_ref()
                .map_or(0, roadrunner_core::routing::RouteResult::expanded_states);
            astar_expanded += astar_result
                .as_ref()
                .map_or(0, roadrunner_core::routing::RouteResult::expanded_states);
        }
        group.bench_with_input(
            BenchmarkId::new(
                "dijkstra",
                format!("{node_count}_nodes_{dijkstra_expanded}_expanded"),
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
        group.bench_with_input(
            BenchmarkId::new(
                "astar",
                format!("{node_count}_nodes_{astar_expanded}_expanded"),
            ),
            &(&graph, &corpus),
            |bencher, (graph, corpus)| {
                bencher.iter(|| {
                    for query in *corpus {
                        let _ = black_box(astar(
                            black_box(graph),
                            query.source,
                            query.destination,
                            &DistanceCost,
                            &DistanceHaversine,
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
