//! Graph-qualified headless showcase over the shared workspace scenario definitions.
use roadrunner_core::{
    cost::{RoutingContext, TravelTimeCost},
    graph::NodeId,
    routing::{AlternativeRouteOptions, alternatives},
};
use roadrunner_simulation::{SimulationScenario, simulate};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) fn run(args: &[String]) -> Result<(), String> {
    let (options, machine) = crate::command_options::flags(args, &["--riders", "--graph"])?;
    let count = options.get("--riders").copied().unwrap_or("12");
    if !matches!(count, "12" | "24") {
        return Err("demo --riders must be 12 or 24".into());
    }
    let definitions: Value =
        serde_json::from_str(include_str!("../../../web/src/fleet-demo-scenarios.json"))
            .map_err(|e| e.to_string())?;
    let scenario: SimulationScenario =
        serde_json::from_value(definitions[count].clone()).map_err(|e| e.to_string())?;
    let path = options
        .get("--graph")
        .copied()
        .unwrap_or("data/fixtures/phase-10/snapshot.semantic-v2");
    let input = crate::graph_command::load(path)?;
    let result = simulate(&input.graph, &scenario).map_err(|e| e.to_string())?;
    let alternatives = alternatives(
        &input.graph,
        NodeId::new(40),
        NodeId::new(34),
        &TravelTimeCost,
        &RoutingContext::new(),
        AlternativeRouteOptions::default(),
    )
    .map_err(|e| e.to_string())?;
    let working = result
        .trace
        .starts
        .iter()
        .map(|s| s.rider)
        .collect::<BTreeSet<_>>()
        .len();
    let summary = &result.summary;
    let human = format!(
        "Fleet showcase · {count} configured riders · {working} riders received work\n  {} orders scheduled · {} delivered · {} outstanding\n  {} distinct route alternatives on the pinned comparison pair\n  Synthetic traffic changes at 30s and 180s; preparation, service and capacity-two insertion.\n\nOpen /workspace?mode=delivery in the running web app, then choose Open recorded fleet demo.\nChoose {count} riders there to replay the same scenario at 1×.\nFor live recomputation, connect the pinned graph and run the fleet preset.\nUse the delay preset for accepted schedule changes and the routing workspace for alternatives.",
        summary.scheduled_orders,
        summary.delivered_orders,
        summary.outstanding_orders,
        alternatives.routes.len()
    );
    let output = json!({"schema_version":1,"semantics":"headless-fleet-demo/v1","scenario":scenario,"configured_riders":scenario.riders.len(),"riders_receiving_work":working,"simulation":result,"alternatives":alternatives,"workspace":"/workspace?mode=delivery","recorded_replay":format!("/demos/fleet-{count}.json"),"limitations":["synthetic_not_physical_gps","bounded_visual_capture_not_city_scale","replay_is_retained_evidence_not_a_live_operation","delay_recovery_is_a_separate_preset"]});
    crate::terminal::output(&output, &human, machine)
}
