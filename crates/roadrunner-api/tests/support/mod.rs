//! HTTP contract, volatile command recognition, authorization and execution integration.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use roadrunner_api::{Api, ApiConfig, Credential};
use roadrunner_core::{
    geo::{CanonicalCoordinate, KilometersPerHour, Seconds},
    graph::*,
};
use roadrunner_dispatch::*;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tower::ServiceExt;
pub(crate) const TOKEN: &str = "test-operator-token-1234";
pub(crate) struct Clock(AtomicU64);
impl Clock {
    pub(crate) fn set(&self, t: u64) {
        self.0.store(t, Ordering::SeqCst);
    }
}
impl OperationalClock for Clock {
    fn time_domain(&self) -> &'static str {
        "test-monotonic/v1"
    }
    #[allow(clippy::cast_precision_loss)]
    fn now(&self) -> Result<DispatchInstant, DispatchTimeError> {
        DispatchInstant::new(self.0.load(Ordering::SeqCst) as f64)
    }
}
pub(crate) fn fixture() -> (Api, Arc<Clock>, String) {
    let clock = Arc::new(Clock(AtomicU64::new(100)));
    let (api, digest) = fixture_with_clock(clock.clone());
    (api, clock, digest)
}
pub(crate) fn fixture_with_clock(clock: Arc<dyn OperationalClock + Send + Sync>) -> (Api, String) {
    let mut b = GraphBuilder::new(
        GraphSnapshotId::new(1),
        GraphMetadata::new(
            "test/v1",
            "synthetic",
            GraphBuildIdentity::new("api-test", "v1", "v1", "synthetic"),
        ),
    );
    let points = [
        CanonicalCoordinate::new(0, 0).unwrap(),
        CanonicalCoordinate::new(0, 10_000).unwrap(),
        CanonicalCoordinate::new(0, 20_000).unwrap(),
        CanonicalCoordinate::new(10_000, 20_000).unwrap(),
    ];
    for (id, p) in (0_u64..).zip(points) {
        b.add_node(BuilderNodeId::new(id), p).unwrap();
    }
    let prop = EdgeProperties::new(
        KilometersPerHour::new(36.0).unwrap(),
        None,
        AccessClass::General,
    );
    // Geometry has an intermediate point, so the HTTP response must preserve more than endpoints.
    b.add_segment(
        BuilderSegmentId::new(0),
        BuilderNodeId::new(0),
        BuilderNodeId::new(1),
        vec![
            points[0],
            CanonicalCoordinate::new(1_000, 5_000).unwrap(),
            points[1],
        ],
        Some(prop),
        Some(prop),
    )
    .unwrap();
    b.add_segment(
        BuilderSegmentId::new(1),
        BuilderNodeId::new(1),
        BuilderNodeId::new(2),
        vec![points[1], points[2]],
        Some(prop),
        Some(prop),
    )
    .unwrap();
    let graph = b.finalize().unwrap();
    let digest = graph.metadata().snapshot_digest().into();
    let operator = Credential {
        principal: "operator".into(),
        read: true,
        write: true,
        simulate: true,
        observation_source: Some("test-source".into()),
        resources: None,
    };
    let reader = Credential {
        principal: "reader".into(),
        read: true,
        write: false,
        simulate: false,
        observation_source: None,
        resources: None,
    };
    let config = ApiConfig {
        credentials: BTreeMap::from([
            (TOKEN.into(), operator),
            ("test-reader-token-1234".into(), reader),
        ]),
        policies: BTreeMap::from([(
            "test".into(),
            OrderPolicy {
                id: "api-test/v1".into(),
                version: 1,
                deadline: AdmissionDeadline::SoftObserved,
                max_completion_delay: None,
                pickup_service: Seconds::ZERO,
                dropoff_service: Seconds::ZERO,
                readiness: ReadinessRule::ValidForecastV1,
            },
        )]),
        recovery: RecoveryPolicy {
            version: 1,
            reroute_penalty: Seconds::ZERO,
            assignment_stability_penalty: Seconds::ZERO,
            minimum_improvement: Seconds::ZERO,
            cooldown: Seconds::ZERO,
        },
        work_budget: 1000,
        certification_window: Seconds::new(2.0).unwrap(),
        workers: 4,
    };
    let api = Api::with_clock(
        graph,
        config,
        OperationalWorldNamespace::from_bytes([1; 16]),
        clock,
    )
    .unwrap();
    (api, digest)
}
pub(crate) fn location(digest: &str, node: u32) -> Value {
    let (latitude, longitude) = match node {
        0 => (0.0, 0.0),
        1 => (0.0, 0.001),
        2 => (0.0, 0.002),
        _ => (0.001, 0.002),
    };
    json!({"graph_digest":digest,"node":node,"coordinate":{"latitude":latitude,"longitude":longitude}})
}
pub(crate) async fn request(
    api: &Api,
    method: &str,
    path: &str,
    key: Option<&str>,
    payload: Value,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if path.starts_with("/v1/commands/") {
        builder = builder.header("x-roadrunner-namespace", api.namespace());
    }
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if let Some(key) = key {
        builder = builder.header("idempotency-key", key);
    }
    let response = api
        .router()
        .oneshot(builder.body(Body::from(payload.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({"raw":String::from_utf8_lossy(&bytes)})),
    )
}
pub(crate) async fn call(
    api: &Api,
    method: &str,
    path: &str,
    key: &str,
    payload: Value,
) -> (StatusCode, Value) {
    request(api, method, path, Some(key), payload, Some(TOKEN)).await
}
pub(crate) async fn create_order(api: &Api, digest: &str, key: &str) -> String {
    let (status,response)=call(api,"POST","/v1/orders",key,json!({"namespace":api.namespace(),"pickup":location(digest,1),"dropoff":location(digest,2),"demand":1,"policy":"test","expected_ready_seconds":100,"forecast_valid_until_seconds":1000})).await;
    assert_eq!(status, StatusCode::OK, "{response}");
    response["result"]["order"].as_str().unwrap().into()
}
pub(crate) async fn create_rider(api: &Api, digest: &str) -> String {
    let (status,response)=call(api,"POST","/v1/riders","rider",json!({"namespace":api.namespace(),"location":location(digest,0),"capacity":3,"available":true})).await;
    assert_eq!(status, StatusCode::OK, "{response}");
    response["result"]["rider"].as_str().unwrap().into()
}
pub(crate) fn observation(id: &str, sequence: u64, time: u64) -> Value {
    json!({"source":"test-source","observation_id":id,"sequence":sequence,"observed_at_seconds":time})
}
