//! Phase 7 pipeline measurements on the versioned real OSM fixture.

use criterion::Criterion;
use roadrunner_osm::{
    compile_motorcycle_graph, decode_dataset_artifact, encode_dataset_artifact, extract_pbf,
};

const SOURCE_ID: &str = "openstreetmap-api:map:3.3780,6.5230,3.3810,6.5260:2026-09-19";

fn benchmark(c: &mut Criterion) {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/fixtures/phase-7/lagos-marina.osm.pbf");
    let dataset = must(extract_pbf(&fixture, SOURCE_ID));
    let bytes = must(encode_dataset_artifact(&dataset));
    let decoded = must(decode_dataset_artifact(&bytes));

    c.bench_function("osm_lagos_fixture_extract_two_pass", |bencher| {
        bencher.iter(|| must(extract_pbf(&fixture, SOURCE_ID)));
    });
    c.bench_function("osm_lagos_fixture_decode_validate", |bencher| {
        bencher.iter(|| must(decode_dataset_artifact(&bytes)));
    });
    c.bench_function("osm_lagos_fixture_compile_motorcycle", |bencher| {
        bencher.iter(|| must(compile_motorcycle_graph(&decoded)));
    });
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
