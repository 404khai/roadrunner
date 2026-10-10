//! Local HTTP entry point. Explicit trusted-local mode has no physical device integration.
use roadrunner_api::{Api, ApiConfig, Credential};
use roadrunner_core::{geo::Seconds, graph::decode_graph_artifact};
use roadrunner_dispatch::{AdmissionDeadline, OrderPolicy, ReadinessRule, RecoveryPolicy};
use std::{collections::BTreeMap, error::Error, net::SocketAddr};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments == ["--openapi"] {
        println!("{}", serde_json::to_string_pretty(&Api::openapi())?);
        return Ok(());
    }
    if arguments.len() != 2 || arguments[0] != "--trusted-local" {
        return Err("usage: roadrunner-api --trusted-local <graph.rr-graph>; ROADRUNNER_API_TOKEN (>=16 bytes) required; ROADRUNNER_API_BIND defaults to 127.0.0.1:3000".into());
    }
    let bind: SocketAddr = std::env::var("ROADRUNNER_API_BIND")
        .unwrap_or_else(|_| "127.0.0.1:3000".into())
        .parse()?;
    if !bind.ip().is_loopback() {
        return Err("trusted-local entry point only accepts loopback bind addresses".into());
    }
    let token =
        std::env::var("ROADRUNNER_API_TOKEN").map_err(|_| "ROADRUNNER_API_TOKEN is required")?;
    let graph = decode_graph_artifact(&std::fs::read(&arguments[1])?)?;
    // Explicit local experiment policy; no hard/cumulative SLA is invented.
    let policy = OrderPolicy {
        id: "local-soft-forecast/v1".into(),
        version: 1,
        deadline: AdmissionDeadline::SoftObserved,
        max_completion_delay: None,
        pickup_service: Seconds::ZERO,
        dropoff_service: Seconds::ZERO,
        readiness: ReadinessRule::ValidForecastV1,
    };
    let config = ApiConfig {
        credentials: BTreeMap::from([(
            token,
            Credential {
                principal: "local-operator".into(),
                read: true,
                write: true,
                simulate: true,
                observation_source: Some("trusted-local".into()),
                resources: None,
            },
        )]),
        policies: BTreeMap::from([("local".into(), policy)]),
        recovery: RecoveryPolicy {
            version: 1,
            reroute_penalty: Seconds::new(30.0)?,
            assignment_stability_penalty: Seconds::new(60.0)?,
            minimum_improvement: Seconds::new(1.0)?,
            cooldown: Seconds::new(30.0)?,
        },
        work_budget: 50_000,
        certification_window: Seconds::new(2.0)?,
        workers: 4,
    };
    let api = Api::new(graph, config)?;
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(address=%listener.local_addr()?,namespace=%api.namespace(),"Roadrunner trusted-local API listening; Swagger UI at /swagger-ui/");
    axum::serve(listener, api.router())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
