//! Reproducible bounded-export measurements. Routing/building occur outside timed samples.
use roadrunner_core::{
    cost::{RoutingContext, TravelTimeCost},
    geo::{CanonicalCoordinate, KilometersPerHour, Seconds},
    graph::{
        AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph,
        GraphBuildIdentity, GraphBuilder, GraphMetadata, GraphSnapshotId, NodeId,
        encode_graph_artifact,
    },
    routing::{EvaluationProvenance, GeometryLimits, dijkstra, route_evidence},
};
use serde_json::json;
use std::{fmt::Debug, time::Instant};
fn ok<T, E: Debug>(v: Result<T, E>) -> T {
    v.unwrap_or_else(|e| panic!("{e:?}"))
}
fn graph(segments: u32, vertices: u32) -> FrozenGraph {
    let mut b = GraphBuilder::new(
        GraphSnapshotId::new(26),
        GraphMetadata::new(
            "synthetic-motorcycle/v1",
            "synthetic",
            GraphBuildIdentity::new(
                "phase26-chain/v1",
                "generated",
                "e7-chain/v1",
                format!("segments={segments};vertices={vertices}"),
            ),
        ),
    );
    let point = |x: u32, y: i32| {
        ok(CanonicalCoordinate::new(
            y,
            i32::try_from(x).unwrap_or_else(|e| panic!("{e}")),
        ))
    };
    for i in 0..=segments {
        ok(b.add_node(BuilderNodeId::new(u64::from(i)), point(i * vertices, 0)));
    }
    let p = EdgeProperties::new(ok(KilometersPerHour::new(36.0)), None, AccessClass::General);
    for i in 0..segments {
        let shape = (0..vertices)
            .map(|j| {
                point(
                    i * vertices + if j == vertices - 1 { vertices } else { j },
                    i32::from(j != 0 && j != vertices - 1),
                )
            })
            .collect();
        ok(b.add_segment(
            BuilderSegmentId::new(u64::from(i)),
            BuilderNodeId::new(u64::from(i)),
            BuilderNodeId::new(u64::from(i + 1)),
            shape,
            Some(p),
            None,
        ));
    }
    ok(b.finalize())
}
fn stats(mut values: Vec<u128>) -> serde_json::Value {
    let raw = values.clone();
    values.sort_unstable();
    let n = values.len();
    json!({"raw_nanoseconds":raw,"median_nanoseconds":values[n/2],"p95_nanoseconds":values[(n*95).div_ceil(100)-1],"p99_nanoseconds":values[(n*99).div_ceil(100)-1]})
}
fn main() {
    let mut results = Vec::new();
    let runs = 31;
    for (segments, vertices) in [(100, 4), (1000, 16), (5000, 32)] {
        let g = graph(segments, vertices);
        let route = ok(dijkstra(
            &g,
            NodeId::new(0),
            NodeId::new(segments),
            &TravelTimeCost,
            &RoutingContext::new(),
        ));
        let provenance = EvaluationProvenance {
            evaluator: "free-flow-travel-time/v1".into(),
            traffic_digest: None,
            departure_seconds: Seconds::ZERO,
            routing_context: RoutingContext::new(),
            time_domain: "synthetic-seconds/v1".into(),
            heuristic: None,
        };
        let mut build = Vec::new();
        let mut serialize = Vec::new();
        let mut bytes = 0;
        for index in 0..runs + 3 {
            let start = Instant::now();
            let export = ok(route_evidence(
                &g,
                &route,
                provenance.clone(),
                GeometryLimits::default(),
            ));
            let elapsed = start.elapsed().as_nanos();
            let start = Instant::now();
            let encoded = ok(serde_json::to_vec(&export));
            let encoding = start.elapsed().as_nanos();
            bytes = encoded.len();
            std::hint::black_box(encoded);
            if index >= 3 {
                build.push(elapsed);
                serialize.push(encoding);
            }
        }
        results.push(json!({"nodes":g.node_count(),"edges":g.edge_count(),"segments":segments,"points_per_segment":vertices,"exported_points":segments*vertices,"graph_digest":g.metadata().snapshot_digest(),"algorithm":"dijkstra","configuration":{"warmup":3,"runs":runs,"limits":{"occurrences":20_000,"points":200_000}},"payload_bytes":bytes,"checked_export":stats(build),"serialization":stats(serialize)}));
        if segments == 100 {
            if let Some(path) = std::env::args().nth(2) {
                ok(std::fs::write(
                    path,
                    ok(serde_json::to_vec_pretty(&ok(route_evidence(
                        &g,
                        &route,
                        provenance.clone(),
                        GeometryLimits::default(),
                    )))),
                ));
            }
        }
        if segments == 5000 {
            if let Some(path) = std::env::args().nth(1) {
                ok(std::fs::write(path, ok(encode_graph_artifact(&g))));
            }
        }
    }
    println!(
        "{}",
        json!({"schema_version":1,"workloads":results,"scope":"in-process export and JSON encoding; build/routing excluded; no coordinator lock; no rendering or throughput claim"})
    );
}
