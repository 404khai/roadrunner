//! JSON traffic scenario routing against a validated OSM graph snapshot.

use roadrunner_core::cost::{
    DistanceCost, RoutingContext, TrafficAwareCost, TrafficLevel, TrafficMultiplier,
    TrafficSnapshot, TravelTimeCost,
};
use roadrunner_core::geo::Seconds;
use roadrunner_core::graph::{EdgeId, FrozenGraph};
use roadrunner_core::routing::{RouteResult, dijkstra};
use roadrunner_osm::load_snapshot_bundle;
use serde::{Deserialize, Serialize};

use crate::alternatives_command::source_node;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    schema_version: u32,
    graph_snapshot_digest: String,
    overrides: Vec<Override>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Override {
    edge_id: u32,
    multiplier: TrafficInput,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TrafficInput {
    Level(TrafficLevel),
    Numeric(TrafficMultiplier),
}

impl TrafficInput {
    fn multiplier(&self) -> TrafficMultiplier {
        match self {
            Self::Level(level) => level.multiplier(),
            Self::Numeric(value) => *value,
        }
    }
}

#[derive(Serialize)]
struct RouteComparison {
    route: RouteResult,
    traffic_adjusted_eta_seconds: f64,
}

#[derive(Serialize)]
struct Output {
    graph_snapshot_digest: String,
    traffic_snapshot_digest: String,
    from_osm_node: i64,
    to_osm_node: i64,
    applied_overrides: usize,
    shortest_distance: RouteComparison,
    free_flow_fastest: RouteComparison,
    traffic_fastest: RouteComparison,
}

pub(super) fn run(
    snapshot: &str,
    source: &str,
    destination: &str,
    scenario_path: &str,
) -> Result<(), String> {
    let source_osm = source
        .parse::<i64>()
        .map_err(|error| format!("invalid source OSM node: {error}"))?;
    let destination_osm = destination
        .parse::<i64>()
        .map_err(|error| format!("invalid destination OSM node: {error}"))?;
    let loaded = load_snapshot_bundle(snapshot, true).map_err(|error| error.to_string())?;
    let scenario_bytes = std::fs::read(scenario_path)
        .map_err(|error| format!("could not read traffic scenario {scenario_path}: {error}"))?;
    let scenario: Scenario =
        serde_json::from_slice(&scenario_bytes).map_err(|error| error.to_string())?;
    if scenario.schema_version != 1 {
        return Err("traffic scenario schema_version must be 1".to_owned());
    }
    if scenario.graph_snapshot_digest != loaded.manifest.graph_snapshot_digest {
        return Err("traffic scenario belongs to a different graph snapshot".to_owned());
    }
    let source_id = source_node(&loaded.provenance, source_osm)?;
    let destination_id = source_node(&loaded.provenance, destination_osm)?;
    let traffic = TrafficSnapshot::new(
        &loaded.graph,
        scenario.overrides.iter().map(|override_| {
            (
                EdgeId::new(override_.edge_id),
                override_.multiplier.multiplier(),
            )
        }),
    )
    .map_err(|error| error.to_string())?;
    let context = RoutingContext::new();
    let shortest = dijkstra(
        &loaded.graph,
        source_id,
        destination_id,
        &DistanceCost,
        &context,
    )
    .map_err(|error| error.to_string())?;
    let free_flow = dijkstra(
        &loaded.graph,
        source_id,
        destination_id,
        &TravelTimeCost,
        &context,
    )
    .map_err(|error| error.to_string())?;
    let fastest = dijkstra(
        &loaded.graph,
        source_id,
        destination_id,
        &TrafficAwareCost::new(&traffic),
        &context,
    )
    .map_err(|error| error.to_string())?;
    let output = Output {
        graph_snapshot_digest: loaded.manifest.graph_snapshot_digest,
        traffic_snapshot_digest: traffic.traffic_snapshot_digest().to_owned(),
        from_osm_node: source_osm,
        to_osm_node: destination_osm,
        applied_overrides: scenario.overrides.len(),
        shortest_distance: comparison(&loaded.graph, &traffic, shortest)?,
        free_flow_fastest: comparison(&loaded.graph, &traffic, free_flow)?,
        traffic_fastest: comparison(&loaded.graph, &traffic, fastest)?,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn comparison(
    graph: &FrozenGraph,
    traffic: &TrafficSnapshot,
    route: RouteResult,
) -> Result<RouteComparison, String> {
    let mut time = Seconds::ZERO;
    for edge_id in route.edges() {
        let edge = graph
            .edge(*edge_id)
            .ok_or_else(|| format!("route edge {edge_id} is absent"))?;
        let multiplier = traffic
            .multiplier(*edge_id)
            .ok_or_else(|| format!("traffic factor for edge {edge_id} is absent"))?;
        let seconds = Seconds::new(edge.free_flow_travel_time().value() * multiplier.value())
            .map_err(|error| error.to_string())?;
        time = time
            .checked_add(seconds)
            .map_err(|error| error.to_string())?;
    }
    Ok(RouteComparison {
        route,
        traffic_adjusted_eta_seconds: time.value(),
    })
}
