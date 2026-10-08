//! Versioned scenario loading and composition over the deterministic simulation library.

use std::collections::BTreeMap;
use std::path::Path;

use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour};
use roadrunner_core::graph::{
    AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph, GraphBuildIdentity,
    GraphBuilder, GraphMetadata, GraphSnapshotId, decode_graph_artifact,
};
use roadrunner_simulation::{SimulationScenario, compare_strategies, simulate};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    graph: GraphSource,
    scenario: SimulationScenario,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum GraphSource {
    Inline {
        profile: String,
        nodes: Vec<NodeInput>,
        roads: Vec<RoadInput>,
    },
    Artifact {
        path: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeInput {
    id: u64,
    coordinate: CoordinateInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoadInput {
    id: u64,
    from: u64,
    to: u64,
    speed_kph: KilometersPerHour,
    bidirectional: bool,
    #[serde(default)]
    geometry: Option<Vec<CoordinateInput>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinateInput {
    latitude_e7: i32,
    longitude_e7: i32,
}

impl CoordinateInput {
    fn canonical(&self) -> Result<CanonicalCoordinate, String> {
        CanonicalCoordinate::new(self.latitude_e7, self.longitude_e7).map_err(|e| e.to_string())
    }
}

fn inline_graph(
    profile: &str,
    nodes: &[NodeInput],
    roads: &[RoadInput],
) -> Result<FrozenGraph, String> {
    if profile.is_empty() {
        return Err("inline graph profile must not be empty".into());
    }
    let coordinates: BTreeMap<_, _> = nodes
        .iter()
        .map(|n| Ok((n.id, n.coordinate.canonical()?)))
        .collect::<Result<_, String>>()?;
    if coordinates.len() != nodes.len() {
        return Err("duplicate inline node identity".into());
    }
    let mut builder = GraphBuilder::new(
        GraphSnapshotId::new(15),
        GraphMetadata::new(
            profile,
            "synthetic scenario",
            GraphBuildIdentity::new("simulation-inline", "v1", "v1", "synthetic"),
        ),
    );
    for (index, (&id, &coordinate)) in coordinates.iter().enumerate() {
        if u64::try_from(index).map_err(|e| e.to_string())? != id {
            return Err("inline node IDs must be contiguous from zero".into());
        }
        builder
            .add_node(BuilderNodeId::new(id), coordinate)
            .map_err(|e| e.to_string())?;
    }
    for road in roads {
        let from = coordinates.get(&road.from).ok_or("absent road origin")?;
        let to = coordinates.get(&road.to).ok_or("absent road destination")?;
        let properties = EdgeProperties::new(road.speed_kph, None, AccessClass::General);
        builder
            .add_segment(
                BuilderSegmentId::new(road.id),
                BuilderNodeId::new(road.from),
                BuilderNodeId::new(road.to),
                road.geometry
                    .as_ref()
                    .map(|points| {
                        points
                            .iter()
                            .map(CoordinateInput::canonical)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?
                    .unwrap_or_else(|| vec![*from, *to]),
                Some(properties),
                road.bidirectional.then_some(properties),
            )
            .map_err(|e| e.to_string())?;
    }
    builder.finalize().map_err(|e| e.to_string())
}

fn load(path: &str) -> Result<(FrozenGraph, SimulationScenario), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("could not read scenario {path}: {e}"))?;
    let document: Document = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let graph = match document.graph {
        GraphSource::Inline {
            profile,
            nodes,
            roads,
        } => inline_graph(&profile, &nodes, &roads)?,
        GraphSource::Artifact { path: artifact } => {
            if document.scenario.graph_snapshot_digest.is_none() {
                return Err("artifact scenarios must bind graph_snapshot_digest".into());
            }
            let artifact = Path::new(path)
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(artifact);
            decode_graph_artifact(&std::fs::read(&artifact).map_err(|e| {
                format!("could not read graph artifact {}: {e}", artifact.display())
            })?)
            .map_err(|e| e.to_string())?
        }
    };
    Ok((graph, document.scenario))
}

pub(super) fn compare_arguments(path: &str, arguments: &[String]) -> Result<(), String> {
    let mut json = false;
    let mut weight = 1.0;
    let mut args = arguments.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" if !json => json = true,
            "--idle-penalty-weight" => {
                weight = args
                    .next()
                    .ok_or("missing idle penalty weight")?
                    .parse::<f64>()
                    .map_err(|e| e.to_string())?;
            }
            _ => return Err(format!("unknown comparison argument: {arg}")),
        }
    }
    compare(path, json, weight)
}

pub(super) fn compare(path: &str, json: bool, weight: f64) -> Result<(), String> {
    let (graph, scenario) = load(path)?;
    let result = compare_strategies(&graph, &scenario, weight).map_err(|e| e.to_string())?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
        );
    } else {
        for run in &result.runs {
            println!("\nStrategy: {:?}", run.dispatch);
            print_summary(run);
        }
    }
    Ok(())
}

pub(super) fn run(path: &str, json: bool) -> Result<(), String> {
    let (graph, scenario) = load(path)?;
    let result = simulate(&graph, &scenario).map_err(|e| e.to_string())?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?
        );
    } else {
        print_summary(&result);
    }
    Ok(())
}

fn print_summary(result: &roadrunner_simulation::SimulationResult) {
    for failure in &result.prediction_failures {
        println!(
            "Insertion order {}: PredictionUnavailable for order {}; inputs_complete=false; committed=false",
            failure.order.value(),
            failure.unavailable_order.value()
        );
    }
    for record in &result.insertions {
        let evidence = record.decision.evidence();
        println!(
            "Insertion order {}: {:?}; committed={}; inputs_complete={}; riders_complete={}; search_complete={}; work={}",
            evidence.order.value(),
            evidence.termination,
            record.committed,
            evidence.input_complete,
            evidence.riders_complete,
            evidence.search_complete,
            evidence.work
        );
        if let Some(proposal) = record.decision.proposal() {
            println!(
                "  rider {}: incremental road {:.3} s, {:.3} m; plan {:?}",
                proposal.rider().value(),
                proposal.incremental_travel(),
                proposal.incremental_distance(),
                proposal.evaluation().plan.stops
            );
        }
        for rider in &evidence.riders {
            println!(
                "  rider {}: suffix={}, pairs={}, evaluated={}, excluded={}, feasible={}, breaches={:?}, rejected={:?}",
                rider.rider.value(),
                rider.suffix_length,
                rider.pairs_total,
                rider.evaluated,
                rider.pruned,
                rider.feasible,
                rider.baseline.breaches,
                rider.rejections
            );
        }
    }
    print_fleets(result);
    let s = &result.summary;
    println!(
        "Orders: {} (scheduled: {}, uncreated: {})\nAssigned: {}\nDelivered: {}\nUnassigned: {}\nAssigned unfinished: {}\nLate deliveries: {}\nOutstanding past deadline: {}",
        s.created_orders,
        s.scheduled_orders,
        s.uncreated_orders,
        s.assigned_orders,
        s.delivered_orders,
        s.unassigned_orders,
        s.assigned_unfinished_orders,
        s.late_deliveries,
        s.outstanding_past_deadline
    );
    let duration = |v: Option<roadrunner_core::geo::Seconds>| {
        v.map_or_else(|| "unavailable".into(), |v| format!("{:.3} s", v.value()))
    };
    println!(
        "Median predicted ETA: {}\np95 predicted ETA: {}\nMean delivered duration: {}\nMedian delivered duration: {}\np95 delivered duration: {}\nMean pickup wait: {}\nMean rider idle time: {}\nRider utilization: {}\nCompleted distance: {:.3} m\nWindow: {}..{} s\nSeed: {}",
        duration(s.predicted_eta.median_seconds),
        duration(s.predicted_eta.p95_seconds),
        duration(s.delivered_duration.mean_seconds),
        duration(s.delivered_duration.median_seconds),
        duration(s.delivered_duration.p95_seconds),
        duration(s.pickup_waiting.mean_seconds),
        duration(s.mean_rider_idle_seconds),
        s.rider_utilization
            .map_or_else(|| "unavailable".into(), |v| format!("{:.3}%", v * 100.0)),
        s.completed_distance_meters.value(),
        result.started_at.value(),
        result.ended_at.value(),
        result.seed
    );
}

fn print_fleets(result: &roadrunner_simulation::SimulationResult) {
    for record in &result.fleets {
        let e = record.decision.evidence();
        println!(
            "Fleet batch {:?}: {:?}; committed={}; inputs_complete={}; riders_complete={}; search_complete={}; work={}",
            e.batch,
            e.termination,
            record.committed,
            e.input_complete,
            e.riders_complete,
            e.search_complete,
            e.work.total()
        );
        if let Some(objective) = &e.selected_objective {
            println!(
                "  admitted={}; remaining road={:.3} s, {:.3} m",
                objective.admitted_orders, objective.road_seconds, objective.road_meters
            );
        }
        for rider in &e.riders {
            println!(
                "  rider {}: isolation={:?}; suffix={}; plan={:?}",
                rider.rider.value(),
                rider.isolation,
                rider.suffix_length,
                rider.selected.as_ref().map(|p| &p.plan.stops)
            );
        }
    }
}
