//! Deterministic time-dependent scenario routing against an OSM snapshot.

use roadrunner_core::cost::{
    RoutingContext, TimeDependentCost, TimeDependentTrafficSnapshot, TrafficProfile,
    TravelTimeCost, TraversalEvaluation, TraversalEvaluator, TraversalState,
};
use roadrunner_core::geo::Seconds;
use roadrunner_core::graph::FrozenGraph;
use roadrunner_core::routing::{RouteResult, dijkstra};
use roadrunner_osm::load_snapshot_bundle;
use serde::{Deserialize, Serialize};

use crate::alternatives_command::source_node;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    schema_version: u32,
    graph_snapshot_digest: String,
    profiles: Vec<TrafficProfile>,
}

#[derive(Serialize)]
struct ComparedRoute {
    route: RouteResult,
    time_dependent_eta_seconds: f64,
}

#[derive(Serialize)]
struct Output {
    graph_snapshot_digest: String,
    traffic_snapshot_digest: String,
    from_osm_node: i64,
    to_osm_node: i64,
    departure_seconds: f64,
    profile_count: usize,
    free_flow_fastest: ComparedRoute,
    time_dependent_fastest: ComparedRoute,
}

pub(super) fn run(
    snapshot: &str,
    source: &str,
    destination: &str,
    scenario_path: &str,
    departure: &str,
) -> Result<(), String> {
    let source_osm = source
        .parse::<i64>()
        .map_err(|error| format!("invalid source OSM node: {error}"))?;
    let destination_osm = destination
        .parse::<i64>()
        .map_err(|error| format!("invalid destination OSM node: {error}"))?;
    let departure_value = departure
        .parse::<f64>()
        .map_err(|error| format!("invalid departure time: {error}"))?;
    let departure_time = Seconds::new(departure_value).map_err(|error| error.to_string())?;
    let loaded = load_snapshot_bundle(snapshot, true).map_err(|error| error.to_string())?;
    let bytes = std::fs::read(scenario_path)
        .map_err(|error| format!("could not read time profile {scenario_path}: {error}"))?;
    let scenario: Scenario = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if scenario.schema_version != 1 {
        return Err("time-dependent scenario schema_version must be 1".to_owned());
    }
    if scenario.graph_snapshot_digest != loaded.manifest.graph_snapshot_digest {
        return Err("time-dependent scenario belongs to a different graph snapshot".to_owned());
    }
    let source_id = source_node(&loaded.provenance, source_osm)?;
    let destination_id = source_node(&loaded.provenance, destination_osm)?;
    let profile_count = scenario.profiles.len();
    let traffic = TimeDependentTrafficSnapshot::new(&loaded.graph, scenario.profiles)
        .map_err(|error| error.to_string())?;
    let evaluator = TimeDependentCost::new(&traffic);
    let context = RoutingContext::with_departure_time(departure_time);
    let base = dijkstra(
        &loaded.graph,
        source_id,
        destination_id,
        &TravelTimeCost,
        &context,
    )
    .map_err(|error| error.to_string())?;
    let selected = dijkstra(
        &loaded.graph,
        source_id,
        destination_id,
        &evaluator,
        &context,
    )
    .map_err(|error| error.to_string())?;
    let output = Output {
        graph_snapshot_digest: loaded.manifest.graph_snapshot_digest,
        traffic_snapshot_digest: traffic.traffic_snapshot_digest().to_owned(),
        from_osm_node: source_osm,
        to_osm_node: destination_osm,
        departure_seconds: departure_time.value(),
        profile_count,
        free_flow_fastest: compare(&loaded.graph, evaluator, &context, base)?,
        time_dependent_fastest: compare(&loaded.graph, evaluator, &context, selected)?,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn compare(
    graph: &FrozenGraph,
    evaluator: TimeDependentCost<'_>,
    context: &RoutingContext,
    route: RouteResult,
) -> Result<ComparedRoute, String> {
    let mut elapsed = Seconds::ZERO;
    for edge_id in route.edges() {
        let edge = graph
            .edge(*edge_id)
            .ok_or_else(|| format!("route edge {edge_id} is absent"))?;
        let segment = graph
            .segment(edge.segment())
            .ok_or_else(|| format!("route segment for edge {edge_id} is absent"))?;
        let traversal = evaluator
            .evaluate(edge, segment, TraversalState::new(elapsed), context)
            .map_err(|error| error.to_string())?;
        let TraversalEvaluation::Traversable { travel_time, .. } = traversal else {
            return Err(format!("route edge {edge_id} became forbidden"));
        };
        elapsed = elapsed
            .checked_add(travel_time)
            .map_err(|error| error.to_string())?;
    }
    Ok(ComparedRoute {
        route,
        time_dependent_eta_seconds: elapsed.value(),
    })
}
