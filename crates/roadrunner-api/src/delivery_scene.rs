//! Bounded synthetic execution view. No live-world mutations or physical telemetry.
use crate::{Api, error::ApiError, graph_debug, model::Objective};
use roadrunner_core::{
    geo::Seconds,
    routing::{GeometryLimits, checked_route_geometry},
};
use roadrunner_dispatch::{FulfillmentState, Stop, onboard_load};
use roadrunner_simulation::{SimulationScenario, simulate_scene};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(crate) fn export(api: &Api, scenario: &SimulationScenario) -> Result<Value, ApiError> {
    let graph = &api.inner.graph;
    if scenario.graph_snapshot_digest.as_deref() != Some(graph.metadata().snapshot_digest()) {
        return Err(ApiError::conflict(
            "scene must bind the server graph digest",
        ));
    }
    let result = simulate_scene(graph, scenario).map_err(ApiError::invalid)?;
    let run = format!(
        "scene-v1:{:x}",
        Sha256::digest(serde_json::to_vec(scenario).map_err(ApiError::invalid)?)
    );
    let routes: Vec<_> = result.trace.starts.iter().map(|start| {
        let route = &start.original.route;
        let geometry = checked_route_geometry(Some(graph), &route.routing.graph_digest,
            &route.nodes, &route.edges, GeometryLimits { occurrences: 2000, points: 16000 });
        json!({"rider":start.rider.value().to_string(),"action":start.original.action_id.value().to_string(),
            "original":start.original,"traffic":start.traffic,
            "geometry":geometry.as_ref().ok(),"geometry_error":geometry.err().map(|e|e.to_string())})
    }).collect();
    let frames: Vec<_> = result.scene_frames.as_ref().ok_or_else(ApiError::internal)?.iter().map(|frame| {
        let w = &frame.world;
        let riders: Result<Vec<_>, ApiError> = w.riders.iter().map(|(id, rider)| {
            let stops: Vec<_> = w.plans[id].stops.iter().map(|s| json!({"kind": match s {Stop::Pickup(_) => "pickup", Stop::Dropoff(_) => "delivery"}, "order":s.order().value().to_string()})).collect();
            Ok(json!({"id":id.value().to_string(),"coordinate":rider.coordinate,"availability":rider.availability,
                "capacity":w.profiles[id].max_capacity.value().to_string(),"load":onboard_load(w,*id).map_err(ApiError::invalid)?.value().to_string(),
                "assigned":w.assignments.values().filter(|a|a.rider==*id).map(|a|a.order.value().to_string()).collect::<Vec<_>>(),
                "stops":stops,"plan_revision":frame.plan_revisions.get(id).map(|r|r.value().to_string()),"execution":frame.execution.get(id)}))
        }).collect();
        let orders: Vec<_> = w.orders.iter().map(|(id,order)| {
            let (status, owner) = match w.fulfillment[id] {
                FulfillmentState::AwaitingPickup => ("awaiting_pickup",w.assignments.get(id).map(|a|a.rider)),
                FulfillmentState::PickedUp{rider,..} => ("onboard",Some(rider)),
                FulfillmentState::Delivered{rider,..} => ("delivered",Some(rider)),
                FulfillmentState::Cancelled{..} => ("cancelled",None),
            };
            json!({"id":id.value().to_string(),"pickup":order.pickup,"dropoff":order.dropoff,"demand":order.demand.value().to_string(),"status":status,"rider":owner.map(|r|r.value().to_string()),"readiness":w.readiness[id]})
        }).collect();
        Ok(json!({"at":frame.at,"revision":frame.revision.number.to_string(),"riders":riders?,"orders":orders,
            "traffic":frame.traffic,"traffic_digest":frame.traffic_digest}))
    }).collect::<Result<_,ApiError>>()?;
    let roads = graph_debug::export(
        api,
        &graph_debug::GraphQuery {
            graph_digest: graph.metadata().snapshot_digest().to_owned(),
            node_offset: 0,
            edge_offset: 0,
            objective: Objective::TravelTime,
        },
        None,
        Seconds::ZERO,
    )?;
    Ok(
        json!({"schema_version":1,"semantics":"synthetic-delivery-scene/v1","motion_model":"causal-uniform-distance/v1","traffic_semantics":"synthetic-static-edge-factors/v1",
        "run_identity":run,"graph_digest":graph.metadata().snapshot_digest(),"time_domain":"simulation_logical_seconds_no_civil_epoch",
        "start":result.started_at,"end":result.ended_at,"frames":frames,"routes":routes,
        "schedules":result.trace.schedules,"effects":result.trace.effects,"roads":roads,
        "summary":result.summary,"scenario":scenario,
        "limitations":["interpolated_not_measured_gps","no_physical_heading","road_page_not_city_coverage","no_ambient_vehicles","no_retained_turn_cost_breakdown"]}),
    )
}
