//! Bounded routing comparison probe; timings remain separate from route evidence.
use std::error::Error;
use std::time::Instant;

use roadrunner_core::cost::{
    RoutingContext, TrafficAwareCost, TrafficLevel, TrafficMultiplier, TrafficSnapshot,
    TravelTimeCost,
};
use roadrunner_core::geo::KilometersPerHour;
use roadrunner_core::graph::{EdgeId, FrozenGraph, NodeId};
use roadrunner_core::routing::{RouteResult, TravelTimeHaversine, astar, dijkstra};
use roadrunner_osm::{GraphProvenance, load_snapshot_bundle};
use serde::Deserialize;
use serde_json::{Value, json};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrafficInput {
    schema_version: u32,
    graph_snapshot_digest: String,
    overrides: Vec<Override>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Override {
    edge_id: u32,
    multiplier: Factor,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Factor {
    Level(TrafficLevel),
    Numeric(TrafficMultiplier),
}
impl Factor {
    fn value(&self) -> TrafficMultiplier {
        match self {
            Self::Level(level) => level.multiplier(),
            Self::Numeric(value) => *value,
        }
    }
}
fn node(provenance: &GraphProvenance, text: &str) -> Result<NodeId> {
    let osm = text.parse::<i64>()?;
    let index = provenance
        .nodes
        .binary_search_by_key(&osm, |n| n.osm_node_id)
        .map_err(|_| "OSM endpoint absent")?;
    Ok(NodeId::new(provenance.nodes[index].node_id))
}
fn traffic_eta(graph: &FrozenGraph, traffic: &TrafficSnapshot, route: &RouteResult) -> Result<f64> {
    route.edges().iter().try_fold(0.0, |time, id| {
        Ok(time
            + graph
                .edge(*id)
                .ok_or("edge absent")?
                .free_flow_travel_time()
                .value()
                * traffic.multiplier(*id).ok_or("factor absent")?.value())
    })
}
fn measure<F>(
    id: &str,
    graph: &FrozenGraph,
    traffic: &TrafficSnapshot,
    mut query: F,
) -> Result<Value>
where
    F: FnMut() -> std::result::Result<RouteResult, roadrunner_core::routing::RoutingError>,
{
    let warmup = query()?;
    let began = Instant::now();
    let route = std::hint::black_box(query()?);
    let elapsed = began.elapsed().as_nanos();
    if route != warmup {
        return Err("query failed deterministic replay".into());
    }
    Ok(
        json!({"id":id,"query_ns":elapsed,"traffic_adjusted_eta_seconds":traffic_eta(graph,traffic,&route)?,"route":route}),
    )
}
fn run() -> Result<Value> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 5 {
        return Err("usage: routing_comparison <bundle> <traffic.json> <from-osm> <to-osm> <forward|reverse>".into());
    }
    let loaded = load_snapshot_bundle(&args[0], true)?;
    let graph = &loaded.graph;
    let source: TrafficInput = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    if source.schema_version != 1
        || source.graph_snapshot_digest != graph.metadata().snapshot_digest()
    {
        return Err("traffic schema/graph mismatch".into());
    }
    let traffic = TrafficSnapshot::new(
        graph,
        source
            .overrides
            .iter()
            .map(|o| (EdgeId::new(o.edge_id), o.multiplier.value())),
    )?;
    let from = node(&loaded.provenance, &args[2])?;
    let to = node(&loaded.provenance, &args[3])?;
    let began = Instant::now();
    let speed = graph
        .edges()
        .iter()
        .map(|e| e.properties().effective_free_flow_speed().value())
        .fold(1.0_f64, f64::max);
    let heuristic = TravelTimeHaversine::for_graph(graph, KilometersPerHour::new(speed)?)?;
    let preparation_ns = began.elapsed().as_nanos();
    let context = RoutingContext::new();
    let mut order = vec!["dijkstra_free_flow", "astar_free_flow", "dijkstra_traffic"];
    match args[4].as_str() {
        "forward" => (),
        "reverse" => order.reverse(),
        _ => return Err("order must be forward or reverse".into()),
    }
    let mut variants = Vec::new();
    for id in &order {
        let value = match *id {
            "dijkstra_free_flow" => measure(id, graph, &traffic, || {
                dijkstra(graph, from, to, &TravelTimeCost, &context)
            })?,
            "astar_free_flow" => measure(id, graph, &traffic, || {
                astar(graph, from, to, &TravelTimeCost, &heuristic, &context)
            })?,
            _ => measure(id, graph, &traffic, || {
                dijkstra(graph, from, to, &TrafficAwareCost::new(&traffic), &context)
            })?,
        };
        variants.push(value);
    }
    variants.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    Ok(
        json!({"schema_version":1,"graph_snapshot_digest":graph.metadata().snapshot_digest(),"nodes":graph.node_count(),"directed_edges":graph.edge_count(),"profile":graph.metadata().routing_profile(),"traffic_snapshot_digest":traffic.traffic_snapshot_digest(),"from_osm":args[2],"to_osm":args[3],"heuristic_preparation_ns":preparation_ns,"execution_order":order,"variants":variants}),
    )
}
fn main() -> std::process::ExitCode {
    match run().and_then(|value| Ok(serde_json::to_string(&value)?)) {
        Ok(value) => {
            println!("{value}");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
