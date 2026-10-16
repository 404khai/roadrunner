//! Phase 26 actual producer conformance, against source fixture coordinates and captured inputs.
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[allow(dead_code)]
mod support;
use axum::http::StatusCode;
use serde_json::{Value, json};
use support::*;
fn query(d: &str) -> Value {
    json!({"origin":location(d,2),"destination":location(d,0),"departure_time":"2026-01-01T00:00:00Z","objective":"distance"})
}
#[tokio::test]
async fn geometry_provenance_units_parity_and_noninterference() {
    let (api, _, digest) = fixture();
    let before = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    let q = query(&digest);
    let (status, single) = request(
        &api,
        "POST",
        "/v1/visualization/routes",
        None,
        q.clone(),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{single}");
    let r = &single["routes"][0];
    let points: Vec<_> = r["geometry"]["points_e7"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["latitude_e7"].as_i64().unwrap(),
                p["longitude_e7"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        points,
        vec![(0, 20000), (0, 10000), (0, 10000), (1000, 5000), (0, 0)]
    );
    assert_eq!(r["geometry"]["occurrences"][0]["point_end"], 2);
    assert_eq!(r["geometry"]["occurrences"][1]["point_start"], 2);
    assert_eq!(r["geometry"]["occurrences"][1]["orientation"], "reverse");
    assert_eq!(r["objective"]["kind"], "distance");
    assert_eq!(r["objective_unit"], "meters");
    assert_eq!(r["objective"]["value"], r["distance_meters"]);
    assert_ne!(r["predicted_elapsed_seconds"], r["distance_meters"]);
    assert_eq!(r["provenance"]["departure_seconds"], 1_767_225_600.0);
    assert_eq!(r["provenance"]["time_domain"], "unix_seconds");
    assert_eq!(r["cost_breakdown"], "unavailable_not_retained");
    let (status, alt) = request(
        &api,
        "POST",
        "/v1/visualization/routes/alternatives",
        None,
        q.clone(),
        Some(TOKEN),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{alt}");
    let a = &alt["routes"][0]["route"];
    assert_eq!(r["provenance"], a["provenance"]);
    assert_eq!(r["routing_profile"], "test/v1");
    assert_eq!(r["geometry"]["path_id"], a["geometry"]["path_id"]);
    assert_ne!(r["evaluation_id"], a["evaluation_id"]); // actual algorithms differ
    assert_eq!(alt["routes"][0]["rank"], 1);
    assert_ne!(alt["routes"][0]["member_id"], json!(1));
    assert_eq!(alt["summary"]["termination"], "exhausted");
    assert_eq!(alt["summary"]["ranked_paths"], 1);
    let mut time = q.clone();
    time["objective"] = json!("travel_time");
    let timed = request(
        &api,
        "POST",
        "/v1/visualization/routes",
        None,
        time,
        Some(TOKEN),
    )
    .await
    .1;
    assert_eq!(
        r["geometry"]["path_id"],
        timed["routes"][0]["geometry"]["path_id"]
    );
    assert_ne!(r["evaluation_id"], timed["routes"][0]["evaluation_id"]);
    assert_eq!(timed["routes"][0]["objective_unit"], "seconds");
    let old = request(&api, "POST", "/v1/routes", None, q, Some(TOKEN))
        .await
        .1;
    assert_eq!(old["routes"][0]["distance_meters"], r["distance_meters"]);
    assert_eq!(
        old["routes"][0]["eta_seconds"],
        r["predicted_elapsed_seconds"]
    );
    assert_eq!(
        old["routes"][0]["expanded_states"],
        r["search"]["expanded_states"]
    );
    let after = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    assert_eq!(before, after);
}
#[tokio::test]
async fn no_travel_auth_unsupported_snapping_and_graph_conflict() {
    let (api, _, d) = fixture();
    let mut q = query(&d);
    q["destination"] = q["origin"].clone();
    assert_eq!(
        request(
            &api,
            "POST",
            "/v1/visualization/routes",
            None,
            q.clone(),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let r = request(
        &api,
        "POST",
        "/v1/visualization/routes",
        None,
        q.clone(),
        Some("test-reader-token-1234"),
    )
    .await
    .1;
    assert_eq!(r["routes"][0]["geometry"]["no_travel"], true);
    assert_eq!(
        r["routes"][0]["geometry"]["points_e7"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(r["routes"][0]["geometry"]["occurrences"], json!([]));
    assert_eq!(r["routes"][0]["search"]["expanded_states"], 1);
    q["origin"]["graph_digest"] = json!("wrong");
    assert_eq!(
        request(
            &api,
            "POST",
            "/v1/visualization/routes",
            None,
            q,
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let mut q = query(&d);
    q["origin"]["coordinate"]["latitude"] = json!(1);
    assert_eq!(
        request(
            &api,
            "POST",
            "/v1/visualization/routes",
            None,
            q,
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn captured_traffic_context_is_identical_in_both_producers() {
    let (api, _, d) = fixture();
    let adopted=call(&api,"POST","/v1/traffic","visualization-traffic",json!({"namespace":api.namespace(),"graph_digest":d,"overrides":[{"edge":0,"multiplier":2.0}]})).await;
    assert_eq!(adopted.0, StatusCode::OK);
    let q = json!({"origin":location(&d,0),"destination":location(&d,1),"departure_time":"2026-01-01T00:00:00Z","objective":"traffic_aware"});
    let single = request(
        &api,
        "POST",
        "/v1/visualization/routes",
        None,
        q.clone(),
        Some(TOKEN),
    )
    .await
    .1;
    let multiple = request(
        &api,
        "POST",
        "/v1/visualization/routes/alternatives",
        None,
        q.clone(),
        Some(TOKEN),
    )
    .await
    .1;
    assert!(single["provenance"]["traffic_digest"].is_string());
    assert_eq!(single["provenance"], multiple["provenance"]);
    let s = &single["routes"][0];
    let a = &multiple["routes"][0]["route"];
    assert_eq!(s["objective"]["value"], s["predicted_elapsed_seconds"]);
    assert_eq!(s["objective"], a["objective"]);
    let legacy = request(&api, "POST", "/v1/routes", None, q.clone(), Some(TOKEN))
        .await
        .1;
    let legacy_multiple = request(
        &api,
        "POST",
        "/v1/routes/alternatives",
        None,
        q,
        Some(TOKEN),
    )
    .await
    .1;
    assert_eq!(
        legacy["traffic_digest"],
        single["provenance"]["traffic_digest"]
    );
    assert_eq!(legacy["traffic_digest"], legacy_multiple["traffic_digest"]);
    assert_eq!(legacy["objective"], legacy_multiple["objective"]);
}

#[tokio::test]
async fn decision_recommendation_and_actual_publication_are_separate() {
    let (api, _, d) = fixture();
    let _rider = create_rider(&api, &d).await;
    let order = create_order(&api, &d, "visualization-order").await;
    let (status, response) = call(
        &api,
        "POST",
        "/v1/dispatch",
        "visualization-decision",
        json!({"namespace":api.namespace(),"order":order}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{response}");
    assert_eq!(response["published"], true);
    assert!(response["result"]["publication"].is_object());
    let inspection = &response["result"]["decision"]["inspection"];
    assert_eq!(inspection["schema_version"], 1);
    assert_eq!(inspection["family"], "insertion");
    assert_eq!(
        inspection["authority"],
        "recommended_proposal_not_committed"
    );
    assert_eq!(inspection["proposal_available"], true);
    assert_eq!(
        inspection["retention"],
        "aggregate_counts_and_selected_proposal;not_candidate_history"
    );
    assert_eq!(
        inspection["candidate_leg_geometry"],
        "unavailable_not_retained"
    );
    let replay = request(
        &api,
        "GET",
        "/v1/commands/visualization-decision",
        None,
        json!(null),
        Some(TOKEN),
    )
    .await;
    assert_eq!(replay, (status, response));
    assert_eq!(
        request(
            &api,
            "GET",
            "/v1/commands/visualization-decision",
            None,
            json!(null),
            Some("test-reader-token-1234")
        )
        .await
        .0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn anchor_catalog_is_bounded_exact_read_only_and_authenticated() {
    let (api, _, digest) = fixture();
    let before = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    assert_eq!(
        request(
            &api,
            "GET",
            "/v1/visualization/anchors",
            None,
            json!(null),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, page) = request(
        &api,
        "GET",
        "/v1/visualization/anchors",
        None,
        json!(null),
        Some("test-reader-token-1234"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["graph"]["snapshot_digest"], digest);
    assert_eq!(page["node_count"], 4);
    assert_eq!(page["anchors"].as_array().unwrap().len(), 4);
    assert_eq!(
        page["anchors"][1],
        json!({"node":1,"coordinate":{"latitude":0.0,"longitude":0.001}})
    );
    assert_eq!(page["next_offset"], json!(null));
    let tail = request(
        &api,
        "GET",
        "/v1/visualization/anchors?offset=3",
        None,
        json!(null),
        Some(TOKEN),
    )
    .await
    .1;
    assert_eq!(tail["anchors"].as_array().unwrap().len(), 1);
    assert_eq!(tail["anchors"][0]["node"], 3);
    assert_eq!(
        request(
            &api,
            "GET",
            "/v1/visualization/anchors?offset=5",
            None,
            json!(null),
            Some(TOKEN)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let empty = request(
        &api,
        "GET",
        "/v1/visualization/anchors?offset=4",
        None,
        json!(null),
        Some(TOKEN),
    )
    .await
    .1;
    assert_eq!(empty["anchors"], json!([]));
    let after = request(&api, "GET", "/v1/state", None, json!(null), Some(TOKEN))
        .await
        .1;
    assert_eq!(before, after);
    // Every emitted anchor is accepted by the existing strict routing boundary.
    for a in page["anchors"].as_array().unwrap() {
        let mut location = a.clone();
        location["graph_digest"] = json!(digest);
        let (status, route) = request(
            &api,
            "POST",
            "/v1/visualization/routes",
            None,
            json!({"origin":location,"destination":location}),
            Some(TOKEN),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{route}");
        assert_eq!(route["routes"][0]["geometry"]["no_travel"], true);
    }
}
