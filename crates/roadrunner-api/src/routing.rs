use crate::{
    Api,
    error::ApiError,
    model::{Algorithm, Location, Objective, RouteRequest},
};
use roadrunner_core::{
    cost::{
        DistanceCost, RoutingContext, TrafficAwareCost, TrafficSnapshot, TravelTimeCost,
        TraversalEvaluator,
    },
    geo::Seconds,
    graph::{FrozenGraph, NodeId, Orientation},
    routing::{
        AlternativeRouteOptions, DistanceHaversine, RouteResult, RoutingError, TravelTimeHaversine,
        alternatives, astar, dijkstra,
    },
};
use roadrunner_dispatch::RoutingAnchor;
use serde_json::{Value, json};

pub(crate) fn anchor(graph: &FrozenGraph, location: &Location) -> Result<RoutingAnchor, ApiError> {
    if location.graph_digest != graph.metadata().snapshot_digest() {
        return Err(ApiError::conflict("graph snapshot mismatch"));
    }
    let coordinate = location.coordinate.coordinate()?;
    let node = NodeId::new(location.node);
    if graph.node(node).ok_or_else(ApiError::missing)?.coordinate() != coordinate {
        return Err(ApiError::invalid(
            "coordinate must match the explicit graph node; snapping is unsupported",
        ));
    }
    Ok(RoutingAnchor {
        node,
        coordinate,
        graph_digest: location.graph_digest.clone(),
    })
}
pub(crate) fn route_error(error: RoutingError) -> ApiError {
    match error {
        RoutingError::NoRoute { .. } => {
            ApiError::new(axum::http::StatusCode::NOT_FOUND, "no_route", error)
        }
        _ => ApiError::invalid(error),
    }
}
pub(crate) fn geometry(graph: &FrozenGraph, route: &RouteResult) -> Result<Value, ApiError> {
    let mut points = Vec::new();
    for id in route.edges() {
        let edge = graph.edge(*id).ok_or_else(ApiError::internal)?;
        let geometry = graph
            .segment_geometry(edge.segment())
            .map_err(ApiError::invalid)?;
        let ordered: Vec<_> = if edge.orientation() == Orientation::Reverse {
            geometry.iter().rev().copied().collect()
        } else {
            geometry.to_vec()
        };
        for p in ordered {
            let c = p.to_coordinate();
            let point = [c.longitude(), c.latitude()];
            if points.last() != Some(&point) {
                points.push(point);
            }
        }
    }
    // GeoJSON LineStrings need at least two positions, including a zero-length route.
    if points.is_empty() {
        let node = route
            .path()
            .first()
            .and_then(|id| graph.node(*id))
            .ok_or_else(ApiError::internal)?;
        points.push([node.coordinate().longitude(), node.coordinate().latitude()]);
    }
    if points.len() == 1 {
        points.push(points[0]);
    }
    Ok(json!({"type": "LineString", "coordinates": points}))
}
pub(crate) fn response(
    graph: &FrozenGraph,
    route: &RouteResult,
    rank: usize,
) -> Result<Value, ApiError> {
    Ok(
        json!({"rank":rank, "graph_digest":route.graph_snapshot_digest(),"routing_profile":graph.metadata().routing_profile(), "algorithm":route.algorithm(), "nodes":route.path(), "edges":route.edges(), "distance_meters":route.total_distance().value(), "eta_seconds":route.elapsed_travel_time().value(), "cost":route.total_cost(), "expanded_states":route.expanded_states(), "geometry":geometry(graph, route)?}),
    )
}
pub(crate) fn query(
    api: &Api,
    request: &RouteRequest,
    traffic: Option<&TrafficSnapshot>,
    multiple: bool,
) -> Result<Value, ApiError> {
    let graph = &api.inner.graph;
    let from = anchor(graph, &request.origin)?.node;
    let to = anchor(graph, &request.destination)?.node;
    let departure = if let Some(timestamp) = &request.departure_time {
        let time = chrono::DateTime::parse_from_rfc3339(timestamp).map_err(ApiError::invalid)?;
        #[allow(clippy::cast_precision_loss)]
        let seconds =
            time.timestamp() as f64 + f64::from(time.timestamp_subsec_nanos()) / 1_000_000_000.0;
        Seconds::new(seconds).map_err(ApiError::invalid)?
    } else {
        Seconds::new(api.inner.clock.now().map_err(ApiError::invalid)?.value())
            .map_err(ApiError::invalid)?
    };
    let static_cost;
    let cost: &dyn TraversalEvaluator = match request.objective {
        Objective::Distance => &DistanceCost,
        Objective::TravelTime => &TravelTimeCost,
        Objective::TrafficAware => {
            let overlay =
                traffic.ok_or_else(|| ApiError::conflict("no adopted traffic snapshot"))?;
            static_cost = TrafficAwareCost::new(overlay);
            &static_cost
        }
    };
    let context = RoutingContext::with_departure_time(departure);
    if multiple {
        if matches!(request.algorithm, Algorithm::Astar) {
            return Err(ApiError::invalid(
                "alternative enumeration uses Yen with Dijkstra; omit algorithm or use dijkstra",
            ));
        }
        let count = request.alternatives.unwrap_or(3);
        if !(1..=5).contains(&count) {
            return Err(ApiError::invalid("alternatives must be in 1..=5"));
        }
        let options = AlternativeRouteOptions {
            max_routes: count,
            ..AlternativeRouteOptions::default()
        };
        let result = alternatives(graph, from, to, cost, &context, options).map_err(route_error)?;
        let routes = result
            .routes
            .iter()
            .map(|r| response(graph, &r.route, r.rank))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(
            json!({"routes":routes,"truncated":result.truncated,"termination":result.termination,"options":result.options,"search_states":result.search_states,"departure_seconds":departure}),
        )
    } else {
        if request.alternatives.is_some() {
            return Err(ApiError::invalid(
                "use /v1/routes/alternatives for alternative routes",
            ));
        }
        let route = match request.algorithm {
            Algorithm::Dijkstra => dijkstra(graph, from, to, cost, &context),
            Algorithm::Astar => match request.objective {
                Objective::Distance => astar(graph, from, to, cost, &DistanceHaversine, &context),
                _ => astar(
                    graph,
                    from,
                    to,
                    cost,
                    &TravelTimeHaversine::for_graph(
                        graph,
                        roadrunner_core::geo::KilometersPerHour::new(
                            graph
                                .edges()
                                .iter()
                                .map(|e| e.properties().effective_free_flow_speed().value())
                                .fold(1.0_f64, f64::max),
                        )
                        .map_err(ApiError::invalid)?,
                    )
                    .map_err(ApiError::invalid)?,
                    &context,
                ),
            },
        }
        .map_err(route_error)?;
        Ok(
            json!({"routes":[response(graph,&route,1)?],"departure_seconds":departure,"objective":request.objective,"traffic_digest":if matches!(request.objective,Objective::TrafficAware){traffic.map(TrafficSnapshot::traffic_snapshot_digest)}else{None}}),
        )
    }
}
