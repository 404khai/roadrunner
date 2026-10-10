//! Phase 20 HTTP adapter over one volatile, serialized operational authority.
//! Domain publication remains in dispatch; simulations never mutate live state.
mod error;
mod http;
mod model;
mod routing;
mod runtime;

use roadrunner_core::{geo::Seconds, graph::FrozenGraph};
use roadrunner_dispatch::{
    DispatchInstant, DispatchTimeError, OperationalClock, OperationalState,
    OperationalWorldNamespace, OrderPolicy, RecoveryPolicy, World, WorldData,
};
use runtime::Coordinator;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

/// Explicit authorization for a credential in this server's sole namespace.
#[derive(Clone)]
pub struct Credential {
    /// Stable principal identity; rotating tokens should retain this value.
    pub principal: String,
    /// Read routing and operational state.
    pub read: bool,
    /// Create and issue lifecycle/planning commands.
    pub write: bool,
    /// Execute isolated synthetic simulations.
    pub simulate: bool,
    /// Trusted observation source; None cannot submit physical observations.
    pub observation_source: Option<String>,
    /// Optional allowlist of opaque resource references; None grants namespace-wide access.
    pub resources: Option<Vec<String>>,
}
/// Explicit server-owned policy and resource limits. No client snapshot is authoritative.
#[derive(Clone)]
pub struct ApiConfig {
    /// Bearer token -> stable principal/capabilities. Tokens are never logged.
    pub credentials: BTreeMap<String, Credential>,
    /// Named, validated admission policies selected at order creation.
    pub policies: BTreeMap<String, OrderPolicy>,
    /// Configured deterministic recovery policy.
    pub recovery: RecoveryPolicy,
    /// Dispatch search budget; incomplete search cannot publish.
    pub work_budget: u64,
    /// Explicit endpoint offset checked by the static monotone proof, not an unconditional TTL.
    pub certification_window: Seconds,
    /// Concurrent blocking queries/planning/simulation workers.
    pub workers: usize,
}
/// Monotonic live clock: one UNIX wall-time origin plus elapsed monotonic time.
/// Wall-clock corrections during this namespace lifetime cannot move it backwards.
pub struct HostClock {
    origin: f64,
    started: Instant,
    domain: String,
}
impl HostClock {
    /// Samples the live origin once; no clock is recovered across discarded lifetimes.
    /// # Errors
    /// Rejects an unavailable/pre-epoch host clock.
    pub fn new(namespace: OperationalWorldNamespace) -> Result<Self, DispatchTimeError> {
        let origin = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| DispatchTimeError)?
            .as_secs_f64();
        DispatchInstant::new(origin)?;
        Ok(Self {
            origin,
            started: Instant::now(),
            domain: format!(
                "unix-origin-monotonic/v1/{}",
                namespace.reference("world", 0)
            ),
        })
    }
}
impl OperationalClock for HostClock {
    fn time_domain(&self) -> &str {
        &self.domain
    }
    fn now(&self) -> Result<DispatchInstant, DispatchTimeError> {
        DispatchInstant::new(self.origin + self.started.elapsed().as_secs_f64())
    }
}
/// Application handle shared by all requests. Clone retains the same authority/ledger.
#[derive(Clone)]
pub struct Api {
    pub(crate) inner: Arc<Inner>,
}
pub(crate) struct Inner {
    pub graph: Arc<FrozenGraph>,
    pub config: ApiConfig,
    pub clock: Arc<dyn OperationalClock + Send + Sync>,
    pub coordinator: Mutex<Coordinator>,
    pub workers: Arc<tokio::sync::Semaphore>,
    pub namespace: OperationalWorldNamespace,
}
impl Api {
    /// Establishes a fresh volatile authority. Discarding it discards command recognition.
    /// # Errors
    /// Rejects invalid configuration, entropy, initial state, or clock.
    pub fn new(graph: FrozenGraph, config: ApiConfig) -> Result<Self, String> {
        let namespace = OperationalWorldNamespace::fresh().map_err(|e| e.to_string())?;
        let clock = Arc::new(HostClock::new(namespace).map_err(|e| e.to_string())?);
        Self::with_clock(graph, config, namespace, clock)
    }
    /// Establishes an empty authority with explicit clock/namespace for embedded drivers/tests.
    /// Live callers must use an independently fresh namespace; this is not durable restore.
    /// # Errors
    /// Rejects empty credentials/policies, invalid limits/policies or clock.
    pub fn with_clock(
        graph: FrozenGraph,
        config: ApiConfig,
        namespace: OperationalWorldNamespace,
        clock: Arc<dyn OperationalClock + Send + Sync>,
    ) -> Result<Self, String> {
        if config.workers == 0
            || config.work_budget == 0
            || config.certification_window == Seconds::ZERO
            || config.credentials.is_empty()
            || config.policies.is_empty()
            || config.recovery.version != 1
        {
            return Err("nonempty credentials/policies and positive limits are required".into());
        }
        for (token, credential) in &config.credentials {
            if token.len() < 16
                || credential.principal.is_empty()
                || credential
                    .observation_source
                    .as_ref()
                    .is_some_and(String::is_empty)
            {
                return Err("credentials require a token of at least 16 bytes, a principal and nonempty source".into());
            }
        }
        for (name, policy) in &config.policies {
            if name.is_empty() {
                return Err("empty policy name".into());
            }
            policy.validate().map_err(|e| e.to_string())?;
        }
        clock.now().map_err(|e| e.to_string())?;
        let state = OperationalState::new(
            namespace,
            World::new(1, WorldData::default()).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            inner: Arc::new(Inner {
                graph: Arc::new(graph),
                workers: Arc::new(tokio::sync::Semaphore::new(config.workers)),
                config,
                clock,
                namespace,
                coordinator: Mutex::new(Coordinator::new(state)),
            }),
        })
    }
    /// Returns the opaque namespace-qualified world reference required by commands.
    #[must_use]
    pub fn namespace(&self) -> String {
        self.inner.namespace.reference("world", 0)
    }
    /// Builds the Axum router, including generated `OpenAPI` and vendored Swagger UI.
    pub fn router(&self) -> axum::Router {
        http::router(self.clone())
    }
    /// Generated `OpenAPI` contract; independently accessible without starting a listener.
    #[must_use]
    pub fn openapi() -> utoipa::openapi::OpenApi {
        http::openapi()
    }
}

#[cfg(test)]
extern crate self as roadrunner_api;
#[cfg(test)]
#[path = "../tests/support/mod.rs"]
mod test_support;
