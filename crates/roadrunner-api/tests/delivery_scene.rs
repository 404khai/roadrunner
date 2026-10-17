//! Real engine captures: directed geometry, lifecycle, resource/auth boundaries and noninterference.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[allow(dead_code)]
mod support;
use axum::http::StatusCode;
use serde_json::{Value, json};
use support::*;
fn scenario(digest: &str) -> Value {
    let policy = json!({"id":"scene-test/v1","version":1,"deadline":"SoftObserved","max_completion_delay":null,"pickup_service":10,"dropoff_service":5,"readiness":"CreatedAtFallbackV1"});
    json!({"schema_version":4,"scenario_id":"scene-test/v1","seed":30,"graph_snapshot_digest":digest,"start_seconds":0,"end_seconds":300,"routing_epoch_seconds":0,
      "dispatch":{"kind":"dynamic","work_budget":256,"forecast_validity_seconds":1000,"algorithm":"Greedy","recovery":{"version":1,"reroute_penalty":10,"assignment_stability_penalty":10,"minimum_improvement":1,"cooldown":5}},
      "riders":[{"id":1,"node":0,"capacity":1,"available":true},{"id":2,"node":2,"capacity":1,"available":true}],
      "orders":[{"id":1,"pickup_node":1,"dropoff_node":2,"created_at_seconds":0,"expected_ready_at_seconds":80,"actual_readiness":{"kind":"fixed","at_seconds":80},"demand":1,"admission":policy},
      {"id":2,"pickup_node":2,"dropoff_node":0,"created_at_seconds":0,"expected_ready_at_seconds":0,"actual_readiness":{"kind":"fixed","at_seconds":0},"demand":1,"admission":policy},
      {"id":3,"pickup_node":1,"dropoff_node":0,"created_at_seconds":5,"expected_ready_at_seconds":90,"actual_readiness":{"kind":"fixed","at_seconds":90},"demand":1,"admission":policy}],
      "initial_traffic":[{"edge_id":0,"multiplier":1.5}],"traffic_changes":[{"at_seconds":20,"overrides":[{"edge_id":0,"multiplier":3}]}],"dynamic_events":[{"at_seconds":2,"change":{"kind":"road_delay","rider":1,"additional_seconds":30}},{"at_seconds":2,"change":{"kind":"road_delay","rider":2,"additional_seconds":30}}]})
}
#[tokio::test]
async fn engine_scene_preserves_actions_geometry_stages_and_live_world() {
    let (api, _, digest) = fixture();
    let before = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    let input = scenario(&digest);
    let (status, s) = request(
        &api,
        "POST",
        "/v1/visualization/deliveries",
        None,
        input.clone(),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{s}");
    assert_eq!(s["semantics"], "synthetic-delivery-scene/v1");
    let frames = s["frames"].as_array().unwrap();
    assert!(
        frames
            .iter()
            .all(|f| f["riders"].as_array().unwrap().len() == 2)
    );
    for stage in ["Travelling", "Waiting", "Servicing"] {
        assert!(
            frames.iter().any(|f| f["riders"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["execution"]["stage"] == stage)),
            "missing {stage}"
        );
    }
    assert!(frames.iter().any(|f| {
        f["riders"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["execution"]["stage"] == "Travelling")
            .count()
            == 2
    }));
    let schedules = s["schedules"].as_array().unwrap();
    assert_eq!(schedules.len(), 1);
    let changed = &schedules[0]["execution"];
    let original = s["routes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["original"]["action_id"] == changed["action_id"])
        .unwrap();
    assert_eq!(original["original"]["route"], changed["route"]);
    assert_ne!(original["original"]["generation"], changed["generation"]);
    let routes = s["routes"].as_array().unwrap();
    assert!(routes.iter().any(|r| {
        r["geometry"]["occurrences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["orientation"] == "reverse")
    }));
    assert!(routes.iter().any(|r| {
        r["geometry"]["points_e7"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["latitude_e7"] == 1000)
    }));
    let effects = s["effects"].as_array().unwrap();
    for f in frames {
        for o in f["orders"].as_array().unwrap() {
            if o["status"] == "delivered" {
                assert!(effects.iter().any(|e| e["effect"] == "Completion"
                    && e["at"].as_f64().unwrap() <= f["at"].as_f64().unwrap()));
            }
        }
    }
    assert_eq!(
        s,
        request(
            &api,
            "POST",
            "/v1/visualization/deliveries",
            None,
            input,
            Some(TOKEN)
        )
        .await
        .1
    );
    assert_eq!(
        before,
        request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
            .await
            .1
    );
}
#[tokio::test]
async fn scene_auth_graph_binding_limits_and_empty_world() {
    let (api, _, digest) = fixture();
    for (token, status) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some("test-reader-token-1234"), StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            request(
                &api,
                "POST",
                "/v1/visualization/deliveries",
                None,
                scenario(&digest),
                token
            )
            .await
            .0,
            status
        );
    }
    assert_eq!(
        request(
            &api,
            "POST",
            "/v1/visualization/deliveries",
            None,
            scenario("wrong"),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut input = scenario(&digest);
    input["end_seconds"] = json!(100_000);
    assert_eq!(
        request(
            &api,
            "POST",
            "/v1/visualization/deliveries",
            None,
            input,
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut input = scenario(&digest);
    input["orders"] = json!([]);
    input["dynamic_events"] = json!([]);
    let (status, s) = request(
        &api,
        "POST",
        "/v1/visualization/deliveries",
        None,
        input,
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(s["routes"], json!([]));
    assert_eq!(s["frames"][0]["orders"], json!([]));
}
