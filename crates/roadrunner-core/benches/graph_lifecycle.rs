//! Frozen graph artifact encode and validated-load baseline.

mod support;

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion};
use roadrunner_core::graph::{decode_graph_artifact, encode_graph_artifact};

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    let mut group = criterion.benchmark_group("graph_lifecycle");
    group.sample_size(20);
    for node_count in [1_000_u32, 10_000] {
        group.bench_function(
            BenchmarkId::new("construct_builder", node_count),
            |bencher| {
                bencher.iter(|| black_box(support::generated_builder(node_count, 3)));
            },
        );
        let builder = support::generated_builder(node_count, 3);
        group.bench_function(BenchmarkId::new("finalize", node_count), |bencher| {
            bencher.iter_batched(
                || builder.clone(),
                |builder| black_box(builder.finalize()),
                BatchSize::SmallInput,
            );
        });
        let graph = match builder.finalize() {
            Ok(value) => value,
            Err(error) => panic!("graph finalization failed: {error}"),
        };
        let bytes = match encode_graph_artifact(&graph) {
            Ok(value) => value,
            Err(error) => panic!("artifact encoding failed: {error}"),
        };
        group.bench_with_input(
            BenchmarkId::new("serialize", node_count),
            &graph,
            |bencher, graph| bencher.iter(|| black_box(encode_graph_artifact(black_box(graph)))),
        );
        group.bench_with_input(
            BenchmarkId::new(
                "load_validate",
                format!("{node_count}_nodes_{}_bytes", bytes.len()),
            ),
            &bytes,
            |bencher, bytes| bencher.iter(|| black_box(decode_graph_artifact(black_box(bytes)))),
        );
    }
    group.finish();
    criterion.final_summary();
}
