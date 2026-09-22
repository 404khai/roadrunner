//! Phase 7 pipeline measurements on the versioned real OSM fixture.

use criterion::Criterion;
use roadrunner_core::cost::{DistanceCost, RoutingContext};
use roadrunner_core::graph::{
    NodeId, decode_graph_artifact, encode_graph_artifact, verify_graph_deep,
};
use roadrunner_core::routing::{DistanceHaversine, astar, dijkstra};
use roadrunner_osm::{
    compile_motorcycle_graph, decode_dataset_artifact, decode_provenance_artifact,
    encode_dataset_artifact, encode_provenance_artifact, extract_pbf,
};

const SOURCE_ID: &str = "openstreetmap-api:map:3.3780,6.5230,3.3810,6.5260:2026-09-19";

#[allow(clippy::too_many_lines)]
fn benchmark(c: &mut Criterion) {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/phase-7/lagos-marina.osm.pbf");
    let dataset = must(extract_pbf(&fixture, SOURCE_ID));
    let bytes = must(encode_dataset_artifact(&dataset));
    let decoded = must(decode_dataset_artifact(&bytes));
    let compiled = must(compile_motorcycle_graph(&decoded));
    let graph_bytes = must(encode_graph_artifact(&compiled.graph));
    let provenance_bytes = must(encode_provenance_artifact(
        &compiled.provenance,
        &compiled.graph,
    ));

    c.bench_function("osm_lagos_fixture_extract_two_pass", |bencher| {
        bencher.iter(|| must(extract_pbf(&fixture, SOURCE_ID)));
    });
    c.bench_function("osm_lagos_fixture_decode_validate", |bencher| {
        bencher.iter(|| must(decode_dataset_artifact(&bytes)));
    });
    c.bench_function("osm_lagos_fixture_compile_motorcycle", |bencher| {
        bencher.iter(|| must(compile_motorcycle_graph(&decoded)));
    });
    c.bench_function("osm_lagos_fixture_encode_graph", |bencher| {
        bencher.iter(|| must(encode_graph_artifact(&compiled.graph)));
    });
    c.bench_function("osm_lagos_fixture_encode_provenance", |bencher| {
        bencher.iter(|| {
            must(encode_provenance_artifact(
                &compiled.provenance,
                &compiled.graph,
            ))
        });
    });
    c.bench_function("osm_lagos_fixture_load_graph", |bencher| {
        bencher.iter(|| must(decode_graph_artifact(&graph_bytes)));
    });
    c.bench_function("osm_lagos_fixture_load_provenance", |bencher| {
        bencher.iter(|| {
            must(decode_provenance_artifact(
                &provenance_bytes,
                &compiled.graph,
            ))
        });
    });
    c.bench_function("osm_lagos_fixture_deep_verify", |bencher| {
        bencher.iter(|| must(verify_graph_deep(&compiled.graph)));
    });
    let (source, destination) = longest_reachable_pair(&compiled.graph);
    c.bench_function("osm_lagos_fixture_route_dijkstra", |bencher| {
        bencher.iter(|| {
            must(dijkstra(
                &compiled.graph,
                source,
                destination,
                &DistanceCost,
                &RoutingContext::new(),
            ))
        });
    });
    c.bench_function("osm_lagos_fixture_route_astar", |bencher| {
        bencher.iter(|| {
            must(astar(
                &compiled.graph,
                source,
                destination,
                &DistanceCost,
                &DistanceHaversine,
                &RoutingContext::new(),
            ))
        });
    });

    let engineering_fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/phase-7/lagos-island-engineering.osm.pbf");
    let engineering_source = "openstreetmap-api:map:3.370,6.518,3.385,6.533:2026-09-21";
    let engineering_dataset = must(extract_pbf(&engineering_fixture, engineering_source));
    let engineering_normalized = must(encode_dataset_artifact(&engineering_dataset));
    let engineering_decoded = must(decode_dataset_artifact(&engineering_normalized));
    let engineering_compiled = must(compile_motorcycle_graph(&engineering_decoded));
    let engineering_graph_bytes = must(encode_graph_artifact(&engineering_compiled.graph));
    c.bench_function("osm_engineering_extract_two_pass", |bencher| {
        bencher.iter(|| must(extract_pbf(&engineering_fixture, engineering_source)));
    });
    c.bench_function("osm_engineering_compile_motorcycle", |bencher| {
        bencher.iter(|| must(compile_motorcycle_graph(&engineering_decoded)));
    });
    c.bench_function("osm_engineering_load_graph", |bencher| {
        bencher.iter(|| must(decode_graph_artifact(&engineering_graph_bytes)));
    });
    c.bench_function("osm_engineering_deep_verify", |bencher| {
        bencher.iter(|| must(verify_graph_deep(&engineering_compiled.graph)));
    });
    let (engineering_source_node, engineering_destination) =
        longest_reachable_pair(&engineering_compiled.graph);
    c.bench_function("osm_engineering_route_dijkstra", |bencher| {
        bencher.iter(|| {
            must(dijkstra(
                &engineering_compiled.graph,
                engineering_source_node,
                engineering_destination,
                &DistanceCost,
                &RoutingContext::new(),
            ))
        });
    });
    c.bench_function("osm_engineering_route_astar", |bencher| {
        bencher.iter(|| {
            must(astar(
                &engineering_compiled.graph,
                engineering_source_node,
                engineering_destination,
                &DistanceCost,
                &DistanceHaversine,
                &RoutingContext::new(),
            ))
        });
    });
}

fn longest_reachable_pair(graph: &roadrunner_core::graph::FrozenGraph) -> (NodeId, NodeId) {
    let mut best = (0_usize, 0_usize, 0_usize);
    for source in 0..graph.node_count() {
        let mut distance = vec![usize::MAX; graph.node_count()];
        let mut queue = std::collections::VecDeque::from([source]);
        distance[source] = 0;
        while let Some(node) = queue.pop_front() {
            let Some(node_id) = u32::try_from(node).ok().map(NodeId::new) else {
                continue;
            };
            for edge in must(graph.outgoing_edges(node_id)) {
                let target = edge.to().value() as usize;
                if distance[target] == usize::MAX {
                    distance[target] = distance[node] + 1;
                    queue.push_back(target);
                }
            }
        }
        if let Some((destination, hops)) = distance
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, hops)| *hops != usize::MAX)
            .max_by_key(|(destination, hops)| (*hops, *destination))
        {
            if hops > best.2 {
                best = (source, destination, hops);
            }
        }
    }
    (
        NodeId::new(u32::try_from(best.0).unwrap_or(0)),
        NodeId::new(u32::try_from(best.1).unwrap_or(0)),
    )
}

fn must<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected benchmark setup failure: {error:?}"),
    }
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    benchmark(&mut criterion);
    criterion.final_summary();
}
