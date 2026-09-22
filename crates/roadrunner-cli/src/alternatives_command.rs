//! Source-node CLI entry point for Phase 9 route comparison.

use roadrunner_core::cost::{RoutingContext, TravelTimeCost};
use roadrunner_core::graph::NodeId;
use roadrunner_core::routing::{AlternativeRouteOptions, alternatives};
use roadrunner_osm::{GraphProvenance, load_snapshot_bundle};
use serde::Serialize;

#[derive(Serialize)]
struct Output {
    graph_snapshot_digest: String,
    from_osm_node: i64,
    to_osm_node: i64,
    objective: &'static str,
    diversity_metric: &'static str,
    result: roadrunner_core::routing::AlternativeRoutes,
}

pub(super) fn run(
    snapshot: &str,
    source: &str,
    destination: &str,
    count: &str,
) -> Result<(), String> {
    let source_osm = source
        .parse::<i64>()
        .map_err(|error| format!("invalid source OSM node: {error}"))?;
    let destination_osm = destination
        .parse::<i64>()
        .map_err(|error| format!("invalid destination OSM node: {error}"))?;
    let max_routes = count
        .parse::<usize>()
        .map_err(|error| format!("invalid alternative count: {error}"))?;
    let loaded = load_snapshot_bundle(snapshot, true).map_err(|error| error.to_string())?;
    let source_id = source_node(&loaded.provenance, source_osm)?;
    let destination_id = source_node(&loaded.provenance, destination_osm)?;
    let result = alternatives(
        &loaded.graph,
        source_id,
        destination_id,
        &TravelTimeCost,
        &RoutingContext::new(),
        AlternativeRouteOptions {
            max_routes,
            ..AlternativeRouteOptions::default()
        },
    )
    .map_err(|error| error.to_string())?;
    let output = Output {
        graph_snapshot_digest: loaded.manifest.graph_snapshot_digest,
        from_osm_node: source_osm,
        to_osm_node: destination_osm,
        objective: "travel_time_seconds",
        diversity_metric: "shared_physical_segment_distance_over_shorter_route_distance",
        result,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn source_node(provenance: &GraphProvenance, osm_node: i64) -> Result<NodeId, String> {
    provenance
        .nodes
        .binary_search_by_key(&osm_node, |mapping| mapping.osm_node_id)
        .ok()
        .map(|index| NodeId::new(provenance.nodes[index].node_id))
        .ok_or_else(|| format!("OSM node {osm_node} is absent from graph provenance"))
}
