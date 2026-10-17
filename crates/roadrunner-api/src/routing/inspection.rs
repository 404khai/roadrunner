//! Bounded re-evaluation of returned paths under the original static request inputs.
//! This is not a retained search trace or a change to portable route identity.
use crate::error::ApiError;
use roadrunner_core::{
    cost::{
        RoutingContext, SearchCapability, TrafficSnapshot, TraversalEvaluation, TraversalEvaluator,
        TraversalState,
    },
    geo::Seconds,
    graph::FrozenGraph,
    routing::RouteResult,
};
use serde_json::{Value, json};

pub(super) fn export(
    graph: &FrozenGraph,
    route: &RouteResult,
    evaluator: &dyn TraversalEvaluator,
    context: &RoutingContext,
    traffic: Option<&TrafficSnapshot>,
    evaluation_id: &str,
) -> Result<Value, ApiError> {
    if !matches!(evaluator.capability(), SearchCapability::StaticNonNegative) {
        return Err(ApiError::internal());
    }
    let mut elapsed = Seconds::ZERO;
    let mut objective = 0.0;
    let mut base = 0.0;
    // Called only after bounded portable geometry export succeeds (<=20,000 occurrences).
    let mut segments = Vec::with_capacity(route.edges().len());
    for (ordinal, id) in route.edges().iter().enumerate() {
        let edge = graph.edge(*id).ok_or_else(ApiError::internal)?;
        let segment = graph
            .segment(edge.segment())
            .ok_or_else(ApiError::internal)?;
        let TraversalEvaluation::Traversable {
            objective_cost,
            travel_time,
        } = evaluator
            .evaluate(edge, segment, TraversalState::new(elapsed), context)
            .map_err(ApiError::invalid)?
        else {
            return Err(ApiError::internal());
        };
        let free_flow = edge.free_flow_travel_time().value();
        let factor = traffic
            .map(|t| t.multiplier(*id).ok_or_else(ApiError::internal))
            .transpose()?
            .map(roadrunner_core::cost::TrafficMultiplier::value);
        segments.push(json!({"ordinal":ordinal,"edge":id,"objective_cost":objective_cost.value(),"predicted_elapsed_seconds":travel_time.value(),"base_seconds":free_flow,"traffic_factor":factor}));
        elapsed = elapsed
            .checked_add(travel_time)
            .map_err(ApiError::invalid)?;
        objective += objective_cost.value();
        base += free_flow;
    }
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-8 * a.abs().max(b.abs()).max(1.0);
    if !close(elapsed.value(), route.elapsed_travel_time().value())
        || !close(objective, route.total_cost().value())
    {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"schema_version":1,"semantics":"pinned_static_path_evaluation/v1","evaluation_id":evaluation_id,"graph_digest":graph.metadata().snapshot_digest(),"traffic_digest":traffic.map(TrafficSnapshot::traffic_snapshot_digest),"base_seconds":base,"traffic_delay_seconds":traffic.map(|_| elapsed.value()-base),"segments":segments}),
    )
}
