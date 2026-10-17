//! Isolated, bounded dispatch evaluation with routes retained at evaluation time.
use crate::{Api, error::ApiError};
use roadrunner_core::{
    geo::{Meters, Seconds},
    graph::{FrozenGraph, NodeId},
    routing::{GeometryLimits, checked_route_geometry},
};
use roadrunner_dispatch::{
    Availability, CandidatePolicy, CandidateResult, CapacityUnits, CoreRouteProvider, DecisionId,
    DispatchDecisionOutcome, DispatchEvaluationError, DispatchInstant, DispatchSnapshot,
    FulfillmentState, Order, OrderId, OrderReadiness, PreparationAwareStrategy, RiderId, RiderPlan,
    RiderProfile, RiderState, RouteOutcome, RouteProvider, RoutedLeg, RoutingAnchor,
    RoutingAnchors, RoutingEpoch, RoutingProvenance, TrafficContext, World, WorldData,
    basic_dispatch, preparation_aware_dispatch,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cell::RefCell;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DispatchSceneRequest {
    graph_digest: String,
    pickup_node: u32,
    customer_node: u32,
    riders: Vec<DemoRider>,
    radius_meters: Option<f64>,
    limit: usize,
    ready_in_seconds: f64,
    preparation_aware: bool,
    publish_demo: bool,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DemoRider {
    node: u32,
    capacity: u32,
    available: bool,
}
struct RecordingProvider<'a> {
    inner: CoreRouteProvider<'a>,
    calls: RefCell<Vec<(NodeId, NodeId, Seconds, RoutedLeg)>>,
}
impl RouteProvider for RecordingProvider<'_> {
    fn graph(&self) -> &FrozenGraph {
        self.inner.graph()
    }
    fn provenance(&self) -> RoutingProvenance {
        self.inner.provenance()
    }
    fn route(
        &self,
        from: NodeId,
        to: NodeId,
        departure: Seconds,
    ) -> Result<RouteOutcome, DispatchEvaluationError> {
        let result = self.inner.route(from, to, departure)?;
        if let RouteOutcome::RouteFound(leg) = &result {
            self.calls
                .borrow_mut()
                .push((from, to, departure, (**leg).clone()));
        }
        Ok(result)
    }
}
#[allow(clippy::too_many_lines)]
pub(crate) fn export(api: &Api, input: &DispatchSceneRequest) -> Result<Value, ApiError> {
    let graph = &api.inner.graph;
    if input.graph_digest != graph.metadata().snapshot_digest() {
        return Err(ApiError::conflict("dispatch graph snapshot mismatch"));
    }
    if input.riders.len() > 16
        || input.limit > 16
        || !input.ready_in_seconds.is_finite()
        || !(0.0..=3600.0).contains(&input.ready_in_seconds)
    {
        return Err(ApiError::invalid(
            "dispatch demo supports 0..16 riders/limit and 0..3600 ready seconds",
        ));
    }
    if input
        .radius_meters
        .is_some_and(|v| !v.is_finite() || !(0.0..=5000.0).contains(&v))
    {
        return Err(ApiError::invalid("radius must be 0..5000 metres"));
    }
    let anchor = |node: u32| -> Result<RoutingAnchor, ApiError> {
        let node = NodeId::new(node);
        Ok(RoutingAnchor {
            coordinate: graph
                .node(node)
                .ok_or_else(|| ApiError::invalid("unknown graph node"))?
                .coordinate(),
            graph_digest: input.graph_digest.clone(),
            node,
        })
    };
    let at = DispatchInstant::new(100.0).map_err(ApiError::invalid)?;
    let pickup = anchor(input.pickup_node)?;
    let customer = anchor(input.customer_node)?;
    let order = OrderId::new(1);
    let mut data = WorldData::default();
    let mut anchors = RoutingAnchors::default();
    data.orders.insert(
        order,
        Order {
            id: order,
            pickup: pickup.coordinate,
            dropoff: customer.coordinate,
            created_at: at,
            deadline: None,
            demand: CapacityUnits::new(1),
        },
    );
    data.readiness.insert(
        order,
        OrderReadiness {
            expected_at: Some(
                DispatchInstant::new(100.0 + input.ready_in_seconds).map_err(ApiError::invalid)?,
            ),
            observed_at: None,
        },
    );
    data.fulfillment
        .insert(order, FulfillmentState::AwaitingPickup);
    anchors.pickups.insert(order, pickup.clone());
    anchors.dropoffs.insert(order, customer.clone());
    let mut rider_views = Vec::new();
    for (index, r) in input.riders.iter().enumerate() {
        if r.capacity > 8 {
            return Err(ApiError::invalid("capacity must be 0..8"));
        }
        let id = RiderId::new(u64::try_from(index + 1).map_err(ApiError::invalid)?);
        let origin = anchor(r.node)?;
        data.profiles.insert(
            id,
            RiderProfile {
                id,
                routing_profile: graph.metadata().routing_profile().to_owned(),
                max_capacity: CapacityUnits::new(u64::from(r.capacity)),
            },
        );
        data.riders.insert(
            id,
            RiderState {
                coordinate: origin.coordinate,
                availability: if r.available {
                    Availability::Available
                } else {
                    Availability::Unavailable
                },
            },
        );
        data.plans.insert(id, RiderPlan::default());
        anchors.riders.insert(id, origin.clone());
        rider_views.push(json!({"id":id.value().to_string(),"coordinate":origin.coordinate,"capacity":r.capacity,"available":r.available}));
    }
    let mut world = World::new(31, data).map_err(ApiError::invalid)?;
    let provider = RecordingProvider {
        inner: CoreRouteProvider::new(graph, TrafficContext::FreeFlow)
            .map_err(ApiError::invalid)?,
        calls: RefCell::new(Vec::new()),
    };
    let policy = match input.radius_meters {
        Some(radius) => CandidatePolicy::Spatial {
            radius: Meters::new(radius).map_err(ApiError::invalid)?,
            limit: input.limit,
        },
        None => CandidatePolicy::Exhaustive,
    };
    let snapshot = DispatchSnapshot::new(
        &world,
        at,
        RoutingEpoch(DispatchInstant::new(0.0).map_err(ApiError::invalid)?),
        &provider,
        &anchors,
    )
    .map_err(ApiError::invalid)?;
    let decision = if input.preparation_aware {
        preparation_aware_dispatch(
            &snapshot,
            order,
            DecisionId::new(1),
            policy,
            PreparationAwareStrategy::default(),
        )
    } else {
        basic_dispatch(&snapshot, order, DecisionId::new(1), policy)
    }
    .map_err(ApiError::invalid)?;
    let geometry = |from: NodeId, to: NodeId, departure: f64| -> Value {
        let calls = provider.calls.borrow();
        let leg = calls.iter().find(|(f, t, d, _)| {
            *f == from && *t == to && d.value().to_bits() == departure.to_bits()
        });
        match leg {
            Some((_, _, _, leg)) => {
                let result = checked_route_geometry(
                    Some(graph),
                    &input.graph_digest,
                    leg.route.path(),
                    leg.route.edges(),
                    GeometryLimits {
                        occurrences: 2000,
                        points: 16000,
                    },
                );
                json!({"departure_seconds":departure,"routing":leg.provenance,"geometry":result.as_ref().ok(),"unavailable":result.err().map(|e|e.to_string())})
            }
            None => json!({"geometry":null,"unavailable":"no_retained_evaluation_leg"}),
        }
    };
    let candidates:Vec<_>=decision.evidence().candidates.iter().map(|c|match &c.result {
        CandidateResult::Feasible{evaluation,score}=>json!({"rider":c.rider.value().to_string(),"feasible":true,"pickup_seconds":evaluation.pickup_travel,"delivery_seconds":evaluation.delivery_travel,"waiting_seconds":evaluation.waiting,"score_seconds":score.total,"completion_seconds":evaluation.completion_time,"legs":[geometry(anchors.riders[&c.rider].node,pickup.node,100.0),geometry(pickup.node,customer.node,evaluation.pickup_departure.value())]}),
        CandidateResult::Rejected(reason)=>json!({"rider":c.rider.value().to_string(),"feasible":false,"rejection":reason,"legs":[]})
    }).collect();
    let selected = match decision.outcome() {
        DispatchDecisionOutcome::Assigned(p) => Some(p.rider.value().to_string()),
        DispatchDecisionOutcome::Unassigned { .. } => None,
    };
    let committed = input.publish_demo && selected.is_some();
    if committed {
        world.commit(&decision).map_err(ApiError::invalid)?;
    }
    let identity = format!(
        "dispatch-demo-v1:{:x}",
        Sha256::digest(serde_json::to_vec(input).map_err(ApiError::invalid)?)
    );
    Ok(
        json!({"schema_version":1,"semantics":"isolated-dispatch-evaluation/v1","evaluation_id":identity,"graph_digest":input.graph_digest,"evaluated_at":at,"time_domain":"simulation_logical_seconds_no_civil_epoch","pickup":pickup.coordinate,"customer":customer.coordinate,"riders":rider_views,"candidates":candidates,"selected":selected,"committed":committed,"active_execution":false,"decision":decision,"explanation":decision.explanation(),"request":input,"publication_scope":"isolated_demo_world_only","geometry_source":"captured_during_original_evaluation","eligible_population":decision.evidence().eligibility_policy,"distance_model":"haversine_metres_inclusive_boundary","discovery_order_retained":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(digest: &str) -> DispatchSceneRequest {
        serde_json::from_value(json!({"graph_digest":digest,"pickup_node":0,"customer_node":1,"riders":[{"node":0,"capacity":2,"available":true},{"node":1,"capacity":0,"available":true},{"node":0,"capacity":2,"available":false}],"radius_meters":1000,"limit":2,"ready_in_seconds":60,"preparation_aware":true,"publish_demo":false})).unwrap_or_else(|e|panic!("{e}"))
    }
    #[test]
    fn captures_original_candidate_geometry_and_keeps_publication_separate() {
        let (api, _, digest) = crate::test_support::fixture();
        let mut request = input(&digest);
        let result = export(&api, &request).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(
            result["decision"]["evidence"]["coverage"],
            "PotentiallyIncomplete"
        );
        assert_eq!(result["candidates"].as_array().map(Vec::len), Some(2));
        assert_eq!(
            result["candidates"][0]["legs"][0]["geometry"]["no_travel"],
            true
        );
        assert!(
            result["candidates"][0]["waiting_seconds"]
                .as_f64()
                .is_some_and(|n| n > 0.0)
        );
        assert_eq!(result["committed"], false);
        assert_eq!(result["active_execution"], false);
        request.publish_demo = true;
        assert_eq!(
            export(&api, &request).unwrap_or_else(|e| panic!("{e:?}"))["committed"],
            true
        );
        request.limit = 0;
        let empty = export(&api, &request).unwrap_or_else(|e| panic!("{e:?}"));
        assert!(empty["selected"].is_null());
        assert_eq!(empty["committed"], false);
    }
    #[test]
    fn validates_bounds_and_graph_before_resolving_ids() {
        let (api, _, digest) = crate::test_support::fixture();
        let mut request = input(&digest);
        request.graph_digest = "wrong".into();
        assert!(export(&api, &request).is_err());
        request.graph_digest = digest;
        request.riders = std::iter::repeat_with(|| DemoRider {
            node: 0,
            capacity: 1,
            available: true,
        })
        .take(17)
        .collect();
        assert!(export(&api, &request).is_err());
    }
}

#[cfg(test)]
mod http_tests {
    use serde_json::json;
    #[tokio::test]
    async fn requires_authorization_and_does_not_publish_live_state() {
        let (api, _, digest) = crate::test_support::fixture();
        let body = json!({"graph_digest":digest,"pickup_node":0,"customer_node":1,"riders":[{"node":0,"capacity":1,"available":true}],"radius_meters":1000,"limit":1,"ready_in_seconds":0,"preparation_aware":true,"publish_demo":true});
        let denied = crate::test_support::request(
            &api,
            "POST",
            "/v1/visualization/dispatch",
            None,
            body.clone(),
            None,
        )
        .await;
        assert_eq!(denied.0, axum::http::StatusCode::UNAUTHORIZED);
        let result = crate::test_support::request(
            &api,
            "POST",
            "/v1/visualization/dispatch",
            None,
            body,
            Some(crate::test_support::TOKEN),
        )
        .await;
        assert_eq!(result.0, axum::http::StatusCode::OK);
        assert_eq!(result.1["committed"], true);
        let state = api
            .inner
            .coordinator
            .lock()
            .unwrap_or_else(|e| panic!("{e}"));
        assert!(state.state.data().assignments.is_empty());
    }
}
