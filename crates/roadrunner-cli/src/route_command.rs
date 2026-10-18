//! Free-flow routing adapters; core algorithms retain ownership of route semantics.
use crate::{
    command_options::{flags, required},
    graph_command::Input,
};
use roadrunner_core::cost::{RoutingContext, TravelTimeCost};
use roadrunner_core::geo::{Coordinate, KilometersPerHour, haversine_distance};
use roadrunner_core::graph::NodeId;
use roadrunner_core::routing::{
    AlternativeRouteOptions, RouteResult, TravelTimeHaversine, alternatives, astar, dijkstra,
};
use serde_json::{Value, json};

fn endpoint(input: &Input, text: &str, radius: f64) -> Result<(NodeId, Value), String> {
    let (id, distance, method) = if let Some(n) = text.strip_prefix("node:") {
        (
            NodeId::new(n.parse().map_err(|_| "invalid graph node ID")?),
            0.0,
            "graph_node",
        )
    } else if let Some(n) = text.strip_prefix("osm:") {
        let provenance = input
            .provenance
            .as_ref()
            .ok_or("OSM IDs require a snapshot bundle with provenance")?;
        (
            crate::alternatives_command::source_node(
                provenance,
                n.parse().map_err(|_| "invalid OSM node ID")?,
            )?,
            0.0,
            "osm_node",
        )
    } else {
        let (lat, lng) = text
            .split_once(',')
            .ok_or("endpoint must be lat,lng, node:<id>, or osm:<id>")?;
        let coordinate = Coordinate::new(
            lat.trim().parse().map_err(|_| "invalid latitude")?,
            lng.trim().parse().map_err(|_| "invalid longitude")?,
        )
        .map_err(|e| e.to_string())?;
        let (id, distance) = input
            .graph
            .nodes()
            .iter()
            .map(|node| {
                (
                    node.id(),
                    haversine_distance(coordinate, node.coordinate()).value(),
                )
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .ok_or("cannot snap to an empty graph")?;
        if distance > radius {
            return Err(format!(
                "endpoint {text} is {distance:.1}m from the nearest graph node, exceeding snap radius {radius}m"
            ));
        }
        (id, distance, "nearest_node")
    };
    let node = input
        .graph
        .node(id)
        .ok_or_else(|| format!("graph node {id:?} is absent"))?;
    Ok((
        id,
        json!({"input":text,"node_id":id,"coordinate":node.coordinate(),"snap_distance_meters":distance,"resolution":method}),
    ))
}

fn search(input: &Input, from: NodeId, to: NodeId, algorithm: &str) -> Result<RouteResult, String> {
    let graph = &input.graph;
    let context = RoutingContext::new();
    match algorithm {
        "dijkstra" => dijkstra(graph, from, to, &TravelTimeCost, &context),
        "astar" => {
            let speed = graph
                .edges()
                .iter()
                .map(|edge| edge.properties().effective_free_flow_speed().value())
                .fold(1.0_f64, f64::max);
            let heuristic = TravelTimeHaversine::for_graph(
                graph,
                KilometersPerHour::new(speed).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            astar(graph, from, to, &TravelTimeCost, &heuristic, &context)
        }
        _ => return Err("algorithm must be dijkstra or astar".into()),
    }
    .map_err(|e| e.to_string())
}

fn summary(route: &RouteResult) -> String {
    format!(
        "{:.1}m · {:.3}s travel · {} expansions\n    Nodes {:?}\n    Directed edges {:?}",
        route.total_distance().value(),
        route.total_cost().value(),
        route.expanded_states(),
        route.path(),
        route.edges()
    )
}

pub(super) fn run(args: &[String], compare: bool) -> Result<(), String> {
    let (options, machine) = flags(
        args,
        &[
            "--graph",
            "--from",
            "--to",
            "--algorithm",
            "--alternatives",
            "--snap-radius-meters",
        ],
    )?;
    let algorithm = options.get("--algorithm").copied().unwrap_or("dijkstra");
    if !["dijkstra", "astar"].contains(&algorithm) {
        return Err("algorithm must be dijkstra or astar".into());
    }
    if compare && (options.contains_key("--algorithm") || options.contains_key("--alternatives")) {
        return Err(
            "route compare selects both algorithms; omit --algorithm and --alternatives".into(),
        );
    }
    let count = options
        .get("--alternatives")
        .copied()
        .unwrap_or("1")
        .parse::<usize>()
        .map_err(|_| "alternatives must be a positive integer")?;
    if count == 0 {
        return Err("alternatives must be a positive integer".into());
    }
    let enumerate = options.contains_key("--alternatives");
    if enumerate && algorithm != "dijkstra" {
        return Err("alternative enumeration uses Dijkstra; omit --algorithm astar".into());
    }
    let radius = options
        .get("--snap-radius-meters")
        .copied()
        .unwrap_or("1000")
        .parse::<f64>()
        .map_err(|_| "invalid snap radius")?;
    if !radius.is_finite() || radius < 0.0 {
        return Err("snap radius must be finite and nonnegative".into());
    }
    let input = crate::graph_command::load(required(&options, "--graph")?)?;
    let (from, source) = endpoint(&input, required(&options, "--from")?, radius)?;
    let (to, destination) = endpoint(&input, required(&options, "--to")?, radius)?;
    let mut output = json!({"schema_version":1,"graph_snapshot_digest":input.graph.metadata().snapshot_digest(),"profile":input.graph.metadata().routing_profile(),"objective":"travel_time_seconds","traffic":"free_flow","source":source,"destination":destination,"snap_radius_meters":radius});
    let human = if compare {
        let d = search(&input, from, to, "dijkstra")?;
        let a = search(&input, from, to, "astar")?;
        let delta = a.total_cost().value() - d.total_cost().value();
        output["cost_delta_seconds"] = json!(delta);
        output["dijkstra"] = json!(d);
        output["astar"] = json!(a);
        format!(
            "Free-flow route comparison\n  Dijkstra {}\n  A*       {}\n  Cost delta {delta:.9}s",
            summary(&d),
            summary(&a)
        )
    } else if enumerate {
        let result = alternatives(
            &input.graph,
            from,
            to,
            &TravelTimeCost,
            &RoutingContext::new(),
            AlternativeRouteOptions {
                max_routes: count,
                ..AlternativeRouteOptions::default()
            },
        )
        .map_err(|e| e.to_string())?;
        let routes = result
            .routes
            .iter()
            .map(|r| format!("  Rank {}: {}", r.rank, summary(&r.route)))
            .collect::<Vec<_>>()
            .join("\n");
        let human = format!(
            "Free-flow alternatives\n{routes}\n  Termination {:?} · truncated {} · search states {}",
            result.termination, result.truncated, result.search_states
        );
        output["alternatives"] = json!(result);
        human
    } else {
        let route = search(&input, from, to, algorithm)?;
        let human = format!("Free-flow route ({algorithm})\n  {}", summary(&route));
        output["route"] = json!(route);
        human
    };
    let human = format!(
        "Graph {}\n  Source {from:?} · destination {to:?}\n{human}",
        input.graph.metadata().snapshot_digest()
    );
    crate::terminal::output(&output, &human, machine)
}
