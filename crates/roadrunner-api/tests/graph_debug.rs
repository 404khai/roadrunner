//! Actual bounded graph producer, with independently asserted source geometry and costs.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[allow(dead_code)]
mod support;
use axum::http::StatusCode;
use serde_json::{Value, json};
use support::*;
fn path(digest: &str, extra: &str) -> String {
    format!("/v1/visualization/graph?graph_digest={digest}{extra}")
}
async fn graph(api: &roadrunner_api::Api, digest: &str, extra: &str) -> Value {
    let (status, v) = request(
        api,
        "GET",
        &path(digest, extra),
        None,
        json!(null),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{v}");
    v
}
#[tokio::test]
async fn exact_reverse_geometry_standalone_costs_and_noninterference() {
    let (api, _, d) = fixture();
    let before = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    let g = graph(&api, &d, "").await;
    assert_eq!(g["schema_version"], 1);
    assert_eq!(g["node_count"], 4);
    assert_eq!(g["edges"].as_array().unwrap().len(), 4);
    assert_eq!(g["nodes"].as_array().unwrap().len(), 4); // includes isolated source node
    let edges = g["edges"].as_array().unwrap();
    let f = edges
        .iter()
        .find(|e| e["from"] == 0 && e["to"] == 1)
        .unwrap();
    let r = edges
        .iter()
        .find(|e| e["from"] == 1 && e["to"] == 0)
        .unwrap();
    let forward = f["points_e7"].as_array().unwrap();
    assert_eq!(forward.len(), 3);
    assert_eq!(forward[1], json!({"latitude_e7":1000,"longitude_e7":5000}));
    let reverse = r["points_e7"].as_array().unwrap();
    assert_eq!(*reverse, forward.iter().rev().cloned().collect::<Vec<_>>());
    assert_eq!(r["orientation"], "reverse");
    assert_eq!(f["objective_cost"], f["base_seconds"]);
    assert_eq!(f["traffic_multiplier"], json!(null));
    assert_eq!(f["road_class"], json!(null));
    assert_eq!(
        g["capabilities"]["explored_node_membership"],
        "unavailable_not_retained"
    );
    let distance = graph(&api, &d, "&objective=distance").await;
    assert_eq!(
        distance["edges"][0]["objective_cost"],
        distance["edges"][0]["distance_meters"]
    );
    let unavailable = graph(&api, &d, "&objective=traffic_aware").await;
    assert_eq!(
        unavailable["edges"][0]["cost_status"],
        "unavailable_no_traffic_snapshot"
    );
    assert_eq!(unavailable["edges"][0]["objective_cost"], json!(null));
    let after = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    assert_eq!(before, after);
}
#[tokio::test]
async fn graph_identity_auth_offset_validation_and_empty_tail() {
    let (api, _, d) = fixture();
    assert_eq!(
        request(&api, "GET", &path(&d, ""), None, json!(null), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &api,
            "GET",
            &path(&d, ""),
            None,
            json!(null),
            Some("test-reader-token-1234")
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &api,
            "GET",
            &path("wrong", "&edge_offset=9999"),
            None,
            json!(null),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &api,
            "GET",
            &path(&d, "&edge_offset=9999"),
            None,
            json!(null),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        request(
            &api,
            "GET",
            &path(&d, "&invented=1"),
            None,
            json!(null),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let tail = graph(&api, &d, "&node_offset=4&edge_offset=4").await;
    assert_eq!(tail["nodes"], json!([]));
    assert_eq!(tail["edges"], json!([]));
    assert_eq!(tail["edge_page"]["next_offset"], json!(null));
}
#[tokio::test]
async fn pinned_overlay_factors_are_distinct_from_nontraffic_objective() {
    let (api, _, d) = fixture();
    let (status,_)=call(&api,"POST","/v1/traffic","graph-debug-traffic",json!({"namespace":api.namespace(),"graph_digest":d,"overrides":[{"edge":0,"multiplier":2.0}]})).await;
    assert_eq!(status, StatusCode::OK);
    let static_view = graph(&api, &d, "").await;
    assert_eq!(static_view["edges"][0]["traffic_multiplier"], 2.0);
    assert_eq!(static_view["evaluation"]["traffic_digest"], json!(null));
    assert_eq!(
        static_view["edges"][0]["objective_cost"],
        static_view["edges"][0]["base_seconds"]
    );
    let timed = graph(&api, &d, "&objective=traffic_aware").await;
    assert_eq!(
        timed["evaluation"]["traffic_digest"],
        timed["traffic_snapshot_digest"]
    );
    let e = &timed["edges"][0];
    assert!(
        (e["objective_cost"].as_f64().unwrap() - 2.0 * e["base_seconds"].as_f64().unwrap()).abs()
            < 1e-8
    );
    let route = request(
        &api,
        "POST",
        "/v1/visualization/routes",
        None,
        json!({"origin":location(&d,0),"destination":location(&d,1),"objective":"traffic_aware"}),
        Some(TOKEN),
    )
    .await
    .1;
    assert_eq!(
        route["routes"][0]["objective"]["value"],
        e["objective_cost"]
    );
    let (status,_)=call(&api,"POST","/v1/traffic","graph-debug-new-traffic",json!({"namespace":api.namespace(),"graph_digest":d,"overrides":[{"edge":0,"multiplier":3.0}]})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(timed["edges"][0]["traffic_multiplier"], 2.0);
    let newer = graph(&api, &d, "&objective=traffic_aware").await;
    assert_eq!(newer["edges"][0]["traffic_multiplier"], 3.0);
    assert_ne!(timed["graph_view_id"], newer["graph_view_id"]);
}
