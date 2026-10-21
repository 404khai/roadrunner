//! Reproduce the bounded fleet showcase through the actual HTTP scene adapter.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use roadrunner_api::{Api, ApiConfig, Credential};
use roadrunner_core::geo::Seconds;
use roadrunner_dispatch::{
    AdmissionDeadline, DispatchInstant, DispatchTimeError, OperationalClock,
    OperationalWorldNamespace, OrderPolicy, ReadinessRule, RecoveryPolicy,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, error::Error, path::Path, sync::Arc};
use tower::ServiceExt;

struct Clock;
impl OperationalClock for Clock {
    fn time_domain(&self) -> &'static str {
        "demo-logical/v1"
    }
    fn now(&self) -> Result<DispatchInstant, DispatchTimeError> {
        DispatchInstant::new(0.0)
    }
}
const TOKEN: &str = "isolated-demo-fixture-token";
async fn request(
    api: &Api,
    method: &str,
    path: &str,
    body: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let response = api
        .router()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("Authorization", format!("Bearer {TOKEN}"))
                .header("Content-Type", "application/json")
                .body(Body::from(body))?,
        )
        .await?;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes().to_vec();
    if status != StatusCode::OK {
        return Err(format!(
            "scene adapter {status}: {}",
            String::from_utf8_lossy(&bytes)
        )
        .into());
    }
    Ok(bytes)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(args.len() == 2 || (args.len() == 3 && args[2] == "--check")) {
        return Err(
            "usage: fleet_demo <12|24> <scene.json> [--check]; run from repository root".into(),
        );
    }
    let loaded = load_graph()?;
    let api = Api::with_clock(
        loaded,
        ApiConfig {
            credentials: BTreeMap::from([(
                TOKEN.into(),
                Credential {
                    principal: "demo-fixture".into(),
                    read: true,
                    write: false,
                    simulate: true,
                    observation_source: None,
                    resources: None,
                },
            )]),
            policies: BTreeMap::from([(
                "demo".into(),
                OrderPolicy {
                    id: "demo-soft/v1".into(),
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
            work_budget: 1024,
            certification_window: Seconds::new(2.0)?,
            workers: 1,
        },
        OperationalWorldNamespace::from_bytes([38; 16]),
        Arc::new(Clock),
    )?;
    let before: Value =
        serde_json::from_slice(&request(&api, "GET", "/v1/state", Vec::new()).await?)?;
    let scenarios: Value =
        serde_json::from_str(include_str!("../../../web/src/fleet-demo-scenarios.json"))?;
    if !matches!(args[0].as_str(), "12" | "24") {
        return Err("choose 12 or 24 riders".into());
    }
    let input = serde_json::to_vec(&scenarios[&args[0]])?;
    let bytes = request(&api, "POST", "/v1/visualization/deliveries", input.clone()).await?;
    if bytes != request(&api, "POST", "/v1/visualization/deliveries", input).await? {
        return Err("scene replay differs".into());
    }
    let after: Value =
        serde_json::from_slice(&request(&api, "GET", "/v1/state", Vec::new()).await?)?;
    if before != after {
        return Err("demo mutated operational authority".into());
    }
    if args.len() == 3 {
        if std::fs::read(&args[1])? != bytes {
            return Err("retained fleet scene is stale".into());
        }
    } else {
        use std::io::Write;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(Path::new(&args[1]))?
            .write_all(&bytes)?;
    }
    let scene: Value = serde_json::from_slice(&bytes)?;
    println!(
        "{}",
        json!({"scene":args[1],"bytes":bytes.len(),"riders":scene["scenario"]["riders"].as_array().map(Vec::len),"frames":scene["frames"].as_array().map(Vec::len),"summary":scene["summary"],"replay_identical":true,"operational_authority_unchanged":true})
    );
    Ok(())
}
fn load_graph() -> Result<roadrunner_core::graph::FrozenGraph, Box<dyn Error>> {
    // Core decoding checks artifact integrity; the existing deep verifier checks semantics.
    let graph = roadrunner_core::graph::decode_graph_artifact(&std::fs::read(
        "data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph",
    )?)?;
    roadrunner_core::graph::verify_graph_deep(&graph)?;
    Ok(graph)
}
