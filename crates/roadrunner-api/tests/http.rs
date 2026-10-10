//! HTTP contract integration tests.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use support::*;
use tower::ServiceExt;
#[tokio::test]
async fn authorization_namespace_and_validation_fail_closed() {
    let (api, _, digest) = fixture();
    assert_eq!(
        request(&api, "GET", "/v1/state", None, json!(null), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    let body = json!({"namespace":api.namespace(),"location":location(&digest,0),"capacity":1,"available":true});
    assert_eq!(
        request(
            &api,
            "POST",
            "/v1/riders",
            Some("key"),
            body.clone(),
            Some("test-reader-token-1234")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(&api, "POST", "/v1/riders", None, body.clone(), Some(TOKEN))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let mut wrong = body.clone();
    wrong["namespace"] = json!("old-discarded-world");
    assert_eq!(
        call(&api, "POST", "/v1/riders", "wrong", wrong).await.0,
        StatusCode::CONFLICT
    );
    let mut invalid = body;
    invalid["location"]["coordinate"]["latitude"] = json!(91);
    assert_eq!(
        call(&api, "POST", "/v1/riders", "invalid", invalid).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let state = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    assert!(state["world"]["riders"].as_object().unwrap().is_empty());
}
#[tokio::test]
async fn retries_and_lost_response_lookup_return_original_outcome() {
    let (api, _, digest) = fixture();
    let body = json!({"namespace":api.namespace(),"location":location(&digest,0),"capacity":1,"available":true});
    let first = call(&api, "POST", "/v1/riders", "retry", body.clone()).await;
    let second = call(&api, "POST", "/v1/riders", "retry", body.clone()).await;
    assert_eq!(first, second);
    assert_eq!(
        request(
            &api,
            "GET",
            "/v1/commands/retry",
            None,
            json!(null),
            Some(TOKEN)
        )
        .await,
        first
    );
    assert_eq!(
        request(
            &api,
            "GET",
            "/v1/commands/retry",
            None,
            json!(null),
            Some("test-reader-token-1234")
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
    let mut changed = body;
    changed["capacity"] = json!(2);
    assert_eq!(
        call(&api, "POST", "/v1/riders", "retry", changed).await.0,
        StatusCode::CONFLICT
    );
    let state = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    assert_eq!(state["world"]["riders"].as_object().unwrap().len(), 1);
    assert_eq!(state["revision"], "1");
}
#[tokio::test]
async fn simultaneous_same_key_reserves_one_command_and_one_effect() {
    let (api, _, digest) = fixture();
    let body = json!({"namespace":api.namespace(),"location":location(&digest,0),"capacity":1,"available":true});
    let (a, b) = tokio::join!(
        call(&api, "POST", "/v1/riders", "race", body.clone()),
        call(&api, "POST", "/v1/riders", "race", body)
    );
    assert!(a.0 == StatusCode::OK || a.0 == StatusCode::ACCEPTED);
    assert!(b.0 == StatusCode::OK || b.0 == StatusCode::ACCEPTED);
    assert_eq!(a.1["command"], b.1["command"]);
    let state = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    assert_eq!(state["world"]["riders"].as_object().unwrap().len(), 1);
    assert_eq!(state["history"].as_array().unwrap().len(), 1);
}
#[tokio::test]
async fn routing_preserves_reverse_segment_geometry_and_algorithm_cost() {
    let (api, _, digest) = fixture();
    let body = json!({"origin":location(&digest,2),"destination":location(&digest,0),"departure_time":"1970-01-01T00:01:40Z","algorithm":"dijkstra","objective":"travel_time"});
    let (status, d) = call(&api, "POST", "/v1/routes", "ignored", body.clone()).await;
    assert_eq!(status, StatusCode::OK, "{d}");
    let geometry = d["routes"][0]["geometry"]["coordinates"]
        .as_array()
        .unwrap();
    assert_eq!(geometry.len(), 4);
    assert_eq!(geometry.first().unwrap(), &json!([0.002, 0.0]));
    assert_eq!(geometry.last().unwrap(), &json!([0.0, 0.0]));
    let mut astar = body.clone();
    astar["algorithm"] = json!("astar");
    let a = call(&api, "POST", "/v1/routes", "astar", astar).await.1;
    assert_eq!(a["routes"][0]["cost"], d["routes"][0]["cost"]);
    let mut alternatives = body.clone();
    alternatives["alternatives"] = json!(3);
    let (status, a) = call(
        &api,
        "POST",
        "/v1/routes/alternatives",
        "alternatives",
        alternatives,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{a}");
    assert_eq!(a["routes"].as_array().unwrap().len(), 1);
    let mut disconnected = body.clone();
    disconnected["destination"] = location(&digest, 3);
    assert_eq!(
        call(&api, "POST", "/v1/routes", "none", disconnected)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    let mut mismatch = body;
    mismatch["origin"]["graph_digest"] = json!("old");
    assert_eq!(
        call(&api, "POST", "/v1/routes", "mismatch", mismatch)
            .await
            .0,
        StatusCode::CONFLICT
    );
}
#[tokio::test]
#[allow(clippy::too_many_lines)] // Complete pickup/dropoff lifecycle in one integration scenario.
async fn creation_dispatch_and_observed_pickup_dropoff_use_shared_authority() {
    let (api, clock, digest) = fixture();
    let rider = create_rider(&api, &digest).await;
    let order = create_order(&api, &digest, "order").await;
    let path = format!("/v1/orders/{order}");
    let created = request(&api, "GET", &path, None, json!(null), Some(TOKEN))
        .await
        .1;
    assert!(created["assignment"].is_null());
    let (status, decision) = call(
        &api,
        "POST",
        "/v1/dispatch",
        "dispatch",
        json!({"namespace":api.namespace(),"order":order}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{decision}");
    assert_eq!(decision["published"], true);
    assert_eq!(
        decision["result"]["publication"]["validity_policy"],
        "static-road-monotone/v1"
    );
    assert_eq!(
        request(
            &api,
            "GET",
            &format!("/v1/deliveries/{order}"),
            None,
            json!(null),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::OK
    );
    let ready = json!({"namespace":api.namespace(),"observation":observation("ready",1,100)});
    assert_eq!(
        call(
            &api,
            "POST",
            &format!("{path}/ready"),
            "ready",
            ready.clone()
        )
        .await
        .0,
        StatusCode::OK
    );
    let repeat = call(
        &api,
        "POST",
        &format!("{path}/ready"),
        "ready-other-key",
        ready,
    )
    .await
    .1;
    assert_eq!(repeat["published"], false);
    let mut last_action = serde_json::Value::Null;
    for (step, t) in [(0, 120), (1, 140)] {
        let data = request(
            &api,
            "GET",
            &format!("/v1/riders/{rider}"),
            None,
            json!(null),
            Some(TOKEN),
        )
        .await
        .1;
        let (status, start) = call(
            &api,
            "POST",
            &format!("/v1/riders/{rider}/actions/start"),
            &format!("start-{step}"),
            json!({"namespace":api.namespace(),"plan_revision":data["plan_revision"]}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{start}");
        last_action = start["result"]["action"].clone();
        clock.set(t);
        let effect = api
            .namespace()
            .replace("world:0", &format!("effect:{}", step * 2));
        let observation_sequence = step * 2 + 1;
        let arrival = json!({"namespace":api.namespace(),"action":start["result"]["action"],"generation":"0","effect_id":effect,"observation":observation(&format!("arrival-{step}"),observation_sequence,t)});
        let (status, result) = call(
            &api,
            "POST",
            &format!("/v1/riders/{rider}/actions/arrival"),
            &format!("arrival-{step}"),
            arrival,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{result}");
        assert_eq!(
            call(
                &api,
                "POST",
                &format!("/v1/riders/{rider}/actions/service"),
                &format!("service-{step}"),
                json!({"namespace":api.namespace()})
            )
            .await
            .0,
            StatusCode::OK
        );
        let effect = api
            .namespace()
            .replace("world:0", &format!("effect:{}", step * 2 + 1));
        let completion = json!({"namespace":api.namespace(),"action":start["result"]["action"],"generation":"0","effect_id":effect,"observation":observation(&format!("complete-{step}"),observation_sequence+1,t)});
        let (status, result) = call(
            &api,
            "POST",
            &format!("/v1/riders/{rider}/actions/completion"),
            &format!("complete-{step}"),
            completion,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{result}");
    }
    let alias = json!({"namespace":api.namespace(),"action":last_action,"generation":"0","effect_id":api.namespace().replace("world:0","effect:4"),"observation":observation("completion-alias",5,140)});
    let (status, alias) = call(
        &api,
        "POST",
        &format!("/v1/riders/{rider}/actions/completion"),
        "completion-alias",
        alias,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{alias}");
    assert_eq!(alias["published"], false);
    assert_eq!(alias["result"]["effect"]["effect"], "already_applied");
    let result = request(&api, "GET", &path, None, json!(null), Some(TOKEN))
        .await
        .1;
    assert!(result["fulfillment"]["Delivered"].is_object());
    assert!(result["assignment"].is_null());
}
#[tokio::test]
async fn observations_reject_untrusted_sources_gaps_and_future_times() {
    let (api, _, digest) = fixture();
    let rider = create_rider(&api, &digest).await;
    let mut body = json!({"namespace":api.namespace(),"location":location(&digest,1),"available":true,"observation":observation("location",2,100)});
    let path = format!("/v1/riders/{rider}/location");
    assert_eq!(
        call(&api, "PATCH", &path, "gap", body.clone()).await.0,
        StatusCode::CONFLICT
    );
    body["observation"]["sequence"] = json!(1);
    body["observation"]["observed_at_seconds"] = json!(101);
    assert_eq!(
        call(&api, "PATCH", &path, "future", body.clone()).await.0,
        StatusCode::CONFLICT
    );
    body["observation"]["observed_at_seconds"] = json!(100);
    body["observation"]["source"] = json!("untrusted");
    assert_eq!(
        call(&api, "PATCH", &path, "untrusted", body.clone())
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    body["observation"]["source"] = json!("test-source");
    assert_eq!(
        call(&api, "PATCH", &path, "good", body.clone()).await.0,
        StatusCode::OK
    );
    body["location"] = location(&digest, 2);
    assert_eq!(
        call(&api, "PATCH", &path, "reuse", body).await.0,
        StatusCode::CONFLICT
    );
}
#[tokio::test]
async fn simulations_are_isolated_and_openapi_and_swagger_are_served() {
    let (api, _, digest) = fixture();
    let before = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    let scenario = json!({"schema_version":1,"seed":7,"start_seconds":0,"end_seconds":100,"routing_epoch_seconds":0,"graph_snapshot_digest":digest,"dispatch":{"kind":"basic"},"riders":[{"id":1,"node":0,"capacity":1,"available":true}],"orders":[{"id":1,"pickup_node":1,"dropoff_node":2,"created_at_seconds":0,"actual_readiness":{"kind":"fixed","at_seconds":0},"demand":1}]});
    let (status, result) = call(
        &api,
        "POST",
        "/v1/simulations",
        "simulation",
        json!({"namespace":api.namespace(),"scenario":scenario}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["published"], false);
    assert_eq!(result["result"]["summary"]["delivered_orders"], 1);
    let after = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    assert_eq!(before, after);
    let doc = request(&api, "GET", "/openapi.json", None, json!(null), None).await;
    assert_eq!(doc.0, StatusCode::OK);
    assert_eq!(doc.1["paths"].as_object().unwrap().len(), 24);
    let mut schemas = vec![&doc.1];
    while let Some(node) = schemas.pop() {
        match node {
            serde_json::Value::Object(object) => {
                if let Some(reference) = object.get("$ref").and_then(serde_json::Value::as_str) {
                    assert!(
                        doc.1.pointer(&reference[1..]).is_some(),
                        "unresolved OpenAPI reference: {reference}"
                    );
                }
                schemas.extend(object.values());
            }
            serde_json::Value::Array(array) => schemas.extend(array.iter()),
            _ => {}
        }
    }
    assert!(doc.1["components"]["schemas"]["CreateOrder"].is_object());
    assert!(doc.1["components"]["securitySchemes"]["bearer"].is_object());
    let response = api
        .router()
        .oneshot(
            Request::builder()
                .uri("/swagger-ui/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        String::from_utf8_lossy(&response.into_body().collect().await.unwrap().to_bytes())
            .contains("Swagger UI")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disconnect_after_reservation_does_not_cancel_or_repeat_publication() {
    use roadrunner_dispatch::{DispatchInstant, DispatchTimeError, OperationalClock};
    use std::sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    struct GateClock {
        armed: AtomicBool,
        gate: Mutex<(bool, bool)>,
        changed: Condvar,
    }
    impl OperationalClock for GateClock {
        fn time_domain(&self) -> &'static str {
            "test-disconnect/v1"
        }
        fn now(&self) -> Result<DispatchInstant, DispatchTimeError> {
            if self.armed.swap(false, Ordering::SeqCst) {
                let mut gate = self.gate.lock().unwrap();
                gate.0 = true;
                self.changed.notify_all();
                while !gate.1 {
                    gate = self.changed.wait(gate).unwrap();
                }
            }
            DispatchInstant::new(100.0)
        }
    }
    let clock = Arc::new(GateClock {
        armed: AtomicBool::new(false),
        gate: Mutex::new((false, false)),
        changed: Condvar::new(),
    });
    let (api, digest) = fixture_with_clock(clock.clone());
    clock.armed.store(true, Ordering::SeqCst);
    let worker_api = api.clone();
    let intent = json!({"namespace":api.namespace(),"location":location(&digest,0),"capacity":1,"available":true});
    let payload = intent.clone();
    let waiting = tokio::spawn(async move {
        call(&worker_api, "POST", "/v1/riders", "disconnect", payload).await
    });
    let waiter_clock = clock.clone();
    tokio::task::spawn_blocking(move || {
        let mut gate = waiter_clock.gate.lock().unwrap();
        while !gate.0 {
            gate = waiter_clock.changed.wait(gate).unwrap();
        }
    })
    .await
    .unwrap();
    waiting.abort(); // Fault point: response waiter disappears after command reservation.
    {
        let mut gate = clock.gate.lock().unwrap();
        gate.1 = true;
        clock.changed.notify_all();
    }
    let result = call(&api, "POST", "/v1/riders", "disconnect", intent).await;
    let mut resolved = result;
    for _ in 0..100 {
        if resolved.0 != StatusCode::ACCEPTED {
            break;
        }
        tokio::task::yield_now().await;
        resolved = request(
            &api,
            "GET",
            "/v1/commands/disconnect",
            None,
            json!(null),
            Some(TOKEN),
        )
        .await;
    }
    assert_eq!(resolved.0, StatusCode::OK, "{resolved:?}");
    assert_eq!(resolved.1["published"], true);
    let state = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    assert_eq!(state["world"]["riders"].as_object().unwrap().len(), 1);
    assert_eq!(state["history"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn fleet_recovery_cancellation_and_traffic_have_recorded_outcomes() {
    let (api, _, digest) = fixture();
    create_rider(&api, &digest).await;
    let first = create_order(&api, &digest, "first").await;
    let second = create_order(&api, &digest, "second").await;
    let (status, result) = call(
        &api,
        "POST",
        "/v1/dispatch/fleet",
        "fleet",
        json!({"namespace":api.namespace(),"orders":[first,second]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["published"], true);
    let (status, result) = call(
        &api,
        "POST",
        "/v1/dispatch/recovery",
        "recovery",
        json!({"namespace":api.namespace(),"trigger":"operator-test"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["published"], false);
    let (status, result) = call(
        &api,
        "POST",
        &format!("/v1/orders/{second}/cancel"),
        "cancel",
        json!({"namespace":api.namespace()}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{result}");
    assert_eq!(result["published"], true);
    let traffic = json!({"namespace":api.namespace(),"graph_digest":digest,"overrides":[{"edge":0,"multiplier":2.0}]});
    assert_eq!(
        call(&api, "POST", "/v1/traffic", "traffic", traffic)
            .await
            .0,
        StatusCode::OK
    );
    let request = json!({"origin":location(&digest,0),"destination":location(&digest,1),"objective":"traffic_aware"});
    let (_, result) = call(&api, "POST", "/v1/routes", "route", request).await;
    assert!(result["traffic_digest"].is_string());
}
