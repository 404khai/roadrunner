//! Machine-readable route corpus export for reference-engine validation.

use std::collections::BTreeMap;

use roadrunner_core::cost::{RoutingContext, TravelTimeCost};
use roadrunner_core::graph::{FrozenGraph, NodeId, Orientation};
use roadrunner_core::routing::{RoutingError, dijkstra};
use roadrunner_osm::load_snapshot_bundle;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u32,
    fixture: String,
    queries: Vec<Query>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    id: String,
    from_osm_node: i64,
    to_osm_node: i64,
    expected: String,
}

#[derive(Serialize)]
struct Export {
    schema_version: u32,
    fixture: String,
    graph_snapshot_digest: String,
    source_sha256: String,
    routing_profile: String,
    graph_node_count: usize,
    graph_edge_count: usize,
    queries: Vec<ExportQuery>,
}

#[derive(Serialize)]
struct ExportQuery {
    id: String,
    from_osm_node: i64,
    to_osm_node: i64,
    origin: [f64; 2],
    destination: [f64; 2],
    expected_status: String,
    status: &'static str,
    distance_meters: Option<f64>,
    duration_seconds: Option<f64>,
    geometry: Option<Vec<[f64; 2]>>,
}

pub(super) fn run(snapshot: &str, corpus_path: &str) -> Result<(), String> {
    let loaded = load_snapshot_bundle(snapshot, true).map_err(|error| error.to_string())?;
    let corpus_bytes = std::fs::read(corpus_path)
        .map_err(|error| format!("could not read route corpus {corpus_path}: {error}"))?;
    let corpus: Corpus =
        serde_json::from_slice(&corpus_bytes).map_err(|error| error.to_string())?;
    if corpus.schema_version != 1 || corpus.queries.is_empty() {
        return Err("route corpus must contain schema version 1 and queries".to_owned());
    }
    let source_nodes: BTreeMap<_, _> = loaded
        .provenance
        .nodes
        .iter()
        .map(|entry| (entry.osm_node_id, NodeId::new(entry.node_id)))
        .collect();
    let mut output = Vec::with_capacity(corpus.queries.len());
    for query in corpus.queries {
        if !matches!(query.expected.as_str(), "reachable" | "unreachable") {
            return Err(format!("invalid expectation for query {}", query.id));
        }
        let source = source_nodes
            .get(&query.from_osm_node)
            .ok_or_else(|| format!("query {} origin is absent from graph provenance", query.id))?;
        let destination = source_nodes.get(&query.to_osm_node).ok_or_else(|| {
            format!(
                "query {} destination is absent from graph provenance",
                query.id
            )
        })?;
        let origin = coordinate(&loaded.graph, *source)?;
        let destination_coordinate = coordinate(&loaded.graph, *destination)?;
        let (status, distance_meters, duration_seconds, geometry) = match dijkstra(
            &loaded.graph,
            *source,
            *destination,
            &TravelTimeCost,
            &RoutingContext::new(),
        ) {
            Ok(route) => (
                "reachable",
                Some(route.total_distance().value()),
                Some(route.elapsed_travel_time().value()),
                Some(route_geometry(&loaded.graph, route.edges(), origin)?),
            ),
            Err(RoutingError::NoRoute { .. }) => ("unreachable", None, None, None),
            Err(error) => return Err(format!("query {} failed: {error}", query.id)),
        };
        output.push(ExportQuery {
            id: query.id,
            from_osm_node: query.from_osm_node,
            to_osm_node: query.to_osm_node,
            origin,
            destination: destination_coordinate,
            expected_status: query.expected,
            status,
            distance_meters,
            duration_seconds,
            geometry,
        });
    }
    let export = Export {
        schema_version: 1,
        fixture: corpus.fixture,
        graph_snapshot_digest: loaded.manifest.graph_snapshot_digest,
        source_sha256: loaded.manifest.source_pbf_sha256,
        routing_profile: loaded.manifest.routing_profile,
        graph_node_count: loaded.graph.node_count(),
        graph_edge_count: loaded.graph.edge_count(),
        queries: output,
    };
    println!(
        "{}",
        serde_json::to_string(&export).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn coordinate(graph: &FrozenGraph, node_id: NodeId) -> Result<[f64; 2], String> {
    let node = graph
        .node(node_id)
        .ok_or_else(|| format!("node {node_id} is missing"))?;
    let coordinate = node.coordinate();
    Ok([coordinate.longitude(), coordinate.latitude()])
}

fn route_geometry(
    graph: &FrozenGraph,
    edges: &[roadrunner_core::graph::EdgeId],
    origin: [f64; 2],
) -> Result<Vec<[f64; 2]>, String> {
    let mut line = Vec::new();
    for edge_id in edges {
        let edge = graph
            .edge(*edge_id)
            .ok_or_else(|| format!("edge {edge_id} is missing"))?;
        let geometry = graph
            .segment_geometry(edge.segment())
            .map_err(|error| error.to_string())?;
        let coordinates = geometry.iter().map(|point| {
            let point = point.to_coordinate();
            [point.longitude(), point.latitude()]
        });
        match edge.orientation() {
            Orientation::Forward => line.extend(coordinates.skip(usize::from(!line.is_empty()))),
            Orientation::Reverse => {
                line.extend(coordinates.rev().skip(usize::from(!line.is_empty())));
            }
        }
    }
    if line.is_empty() {
        line.push(origin);
    }
    Ok(line)
}
