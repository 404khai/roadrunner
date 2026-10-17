use crate::{
    Api, Credential,
    error::{ApiError, ErrorBody},
    model::{
        CommandFailure, CreateOrder, CreateRider, DelayRequest, DispatchRequest, EffectRequest,
        Envelope, FleetRequest, ForecastRequest, NamespaceRequest, ObserveRequest, RecoveryRequest,
        RouteRequest, RoutesResponse, SimulationRequest, StartRequest, TrafficRequest,
        UpdateLocation,
    },
    runtime::{self, Command},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, patch, post},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use utoipa::{
    OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

fn credential(api: &Api, headers: &HeaderMap) -> Result<Credential, ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    token
        .and_then(|t| api.inner.config.credentials.get(t))
        .cloned()
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "valid bearer credential required",
            )
        })
}
fn read(api: &Api, headers: &HeaderMap) -> Result<Credential, ApiError> {
    let c = credential(api, headers)?;
    if !c.read {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "read capability required",
        ));
    }
    Ok(c)
}
fn body<T: DeserializeOwned>(body: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    body.map(|Json(v)| v).map_err(|e| {
        ApiError::new(
            e.status(),
            "invalid_json",
            "JSON body does not match the documented request schema",
        )
    })
}
async fn command(
    api: Api,
    headers: HeaderMap,
    intent: Command,
) -> Result<impl IntoResponse, ApiError> {
    let principal = credential(&api, &headers)?;
    let key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "missing_idempotency_key",
                "Idempotency-Key required",
            )
        })?
        .to_owned();
    runtime::submit(api, principal, key, intent).await
}

#[utoipa::path(get,path="/v1/state",responses((status=200,body=Object),(status=401,body=ErrorBody)),security(("bearer"=[])))]
async fn state(State(api): State<Api>, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    crate::persistence::check_async(&api).await?;
    let principal = read(&api, &headers)?;
    if principal.resources.is_some() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "snapshot requires namespace-wide read capability",
        ));
    }
    let c = api
        .inner
        .coordinator
        .lock()
        .map_err(|_| ApiError::internal())?;
    let references = json!({"orders":c.state.data().orders.keys().map(|id|api.inner.namespace.reference("order",id.value())).collect::<Vec<_>>(),"riders":c.state.data().riders.keys().map(|id|api.inner.namespace.reference("rider",id.value())).collect::<Vec<_>>()});
    Ok(Json(
        json!({"schema_version":1,"namespace":api.namespace(),"revision":c.state.revision().number.to_string(),"time_domain":api.inner.clock.time_domain(),"now_seconds":api.inner.clock.now().map_err(ApiError::invalid)?,"graph":api.inner.graph.metadata(),"references":references,"world":c.state.data(),"plan_revisions":c.state.plan_revisions(),"execution":c.state.execution(),"contexts":c.state.contexts(),"history":c.state.history(),"publications":c.state.publications()}),
    ))
}
#[utoipa::path(get,path="/v1/visualization/graph",params(crate::graph_debug::GraphQuery),responses((status=200,body=Object),(status=401,body=ErrorBody),(status=413,body=ErrorBody),(status=503,body=ErrorBody)),security(("bearer"=[])))]
async fn debug_graph(
    State(api): State<Api>,
    headers: HeaderMap,
    Query(query): Query<crate::graph_debug::GraphQuery>,
) -> Result<axum::response::Response, ApiError> {
    crate::persistence::check_async(&api).await?;
    read(&api, &headers)?;
    let busy = || {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "visualization_busy",
            "graph inspection capacity unavailable; retry later",
        )
    };
    let export_permit = api
        .inner
        .export_admission
        .clone()
        .try_acquire_owned()
        .map_err(|_| busy())?;
    let worker = api
        .inner
        .workers
        .clone()
        .try_acquire_owned()
        .map_err(|_| busy())?;
    let (traffic, departure) = {
        let c = api
            .inner
            .coordinator
            .lock()
            .map_err(|_| ApiError::internal())?;
        (
            c.traffic.clone(),
            roadrunner_core::geo::Seconds::new(
                api.inner.clock.now().map_err(ApiError::invalid)?.value(),
            )
            .map_err(ApiError::invalid)?,
        )
    };
    tokio::task::spawn_blocking(move || {
        let _worker = worker;
        let response = crate::graph_debug::export(&api, &query, traffic.as_ref(), departure)?;
        let bytes = encode_export(&response, 4 * 1024 * 1024)?;
        axum::response::Response::builder()
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::new(HeldExportBody {
                bytes: bytes.into(),
                _permit: export_permit,
            }))
            .map_err(|_| ApiError::internal())
    })
    .await
    .map_err(|_| ApiError::internal())?
}

#[utoipa::path(post,path="/v1/visualization/deliveries",request_body=Object,responses((status=200,body=Object),(status=403,body=ErrorBody),(status=413,body=ErrorBody),(status=503,body=ErrorBody)),security(("bearer"=[])))]
async fn delivery_scene(
    State(api): State<Api>,
    headers: HeaderMap,
    input: Result<Json<roadrunner_simulation::SimulationScenario>, JsonRejection>,
) -> Result<axum::response::Response, ApiError> {
    crate::persistence::check_async(&api).await?;
    let principal = read(&api, &headers)?;
    if !principal.simulate || principal.resources.is_some() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "scene requires namespace-wide read and simulation capabilities",
        ));
    }
    let scenario = body(input)?;
    let busy = || {
        ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "visualization_busy",
            "scene export capacity unavailable; retry later",
        )
    };
    let export_permit = api
        .inner
        .export_admission
        .clone()
        .try_acquire_owned()
        .map_err(|_| busy())?;
    let worker = api
        .inner
        .workers
        .clone()
        .try_acquire_owned()
        .map_err(|_| busy())?;
    tokio::task::spawn_blocking(move || {
        let _worker = worker;
        let response = crate::delivery_scene::export(&api, &scenario)?;
        let bytes = encode_export(&response, 8 * 1024 * 1024)?;
        axum::response::Response::builder()
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(axum::body::Body::new(HeldExportBody {
                bytes: bytes.into(),
                _permit: export_permit,
            }))
            .map_err(|_| ApiError::internal())
    })
    .await
    .map_err(|_| ApiError::internal())?
}

/// Bounded immutable node catalog for explicit route-anchor selection.
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
struct AnchorPage {
    #[serde(default)]
    offset: usize,
}
#[utoipa::path(get,path="/v1/visualization/anchors",params(AnchorPage),responses((status=200,body=Object),(status=401,body=ErrorBody)),security(("bearer"=[])))]
async fn anchors(
    State(api): State<Api>,
    headers: HeaderMap,
    Query(page): Query<AnchorPage>,
) -> Result<Json<Value>, ApiError> {
    crate::persistence::check_async(&api).await?;
    read(&api, &headers)?;
    let graph = &api.inner.graph;
    if page.offset > graph.node_count() {
        return Err(ApiError::invalid("anchor offset exceeds node count"));
    }
    let end = page.offset.saturating_add(500).min(graph.node_count());
    let nodes: Vec<_> = graph.nodes()[page.offset..end]
        .iter()
        .map(|node| {
            json!({"node":node.id(),"coordinate":{
                "latitude":node.coordinate().latitude(),
                "longitude":node.coordinate().longitude()
            }})
        })
        .collect();
    Ok(Json(json!({
        "schema_version":1,"graph":graph.metadata(),
        "node_count":graph.node_count(),"edge_count":graph.edge_count(),
        "offset":page.offset,"next_offset":(end < graph.node_count()).then_some(end),
        "anchors":nodes,"selection_semantics":"explicit_graph_node_no_snapping/v1"
    })))
}

#[utoipa::path(get,path="/v1/orders/{id}",params(("id"=String,Path,description="Opaque namespace:order:id reference")),responses((status=200,body=Object),(status=404,body=ErrorBody)),security(("bearer"=[])))]
async fn order(
    State(api): State<Api>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    crate::persistence::check_async(&api).await?;
    let principal = read(&api, &headers)?;
    runtime::authorize_resource(&principal, &id)?;
    let oid = roadrunner_dispatch::OrderId::new(runtime::reference(&api, "order", &id)?);
    let c = api
        .inner
        .coordinator
        .lock()
        .map_err(|_| ApiError::internal())?;
    let order = c
        .state
        .data()
        .orders
        .get(&oid)
        .ok_or_else(ApiError::missing)?;
    Ok(Json(
        json!({"reference":id,"order":order,"readiness":c.state.data().readiness.get(&oid),"fulfillment":c.state.data().fulfillment.get(&oid),"assignment":c.state.data().assignments.get(&oid),"assigned_rider":c.state.data().assignments.get(&oid).map(|a|api.inner.namespace.reference("rider",a.rider.value())),"accepted":c.state.data().accepted.get(&oid),"revision":c.state.revision().number.to_string()}),
    ))
}
#[utoipa::path(get,path="/v1/riders/{id}",params(("id"=String,Path)),responses((status=200,body=Object),(status=404,body=ErrorBody)),security(("bearer"=[])))]
async fn rider(
    State(api): State<Api>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    crate::persistence::check_async(&api).await?;
    let principal = read(&api, &headers)?;
    runtime::authorize_resource(&principal, &id)?;
    let rid = roadrunner_dispatch::RiderId::new(runtime::reference(&api, "rider", &id)?);
    let c = api
        .inner
        .coordinator
        .lock()
        .map_err(|_| ApiError::internal())?;
    let rider = c
        .state
        .data()
        .riders
        .get(&rid)
        .ok_or_else(ApiError::missing)?;
    Ok(Json(
        json!({"reference":id,"rider":rider,"profile":c.state.data().profiles.get(&rid),"plan":c.state.data().plans.get(&rid),"plan_revision":c.state.plan_revisions().get(&rid).map(|r|r.value().to_string()),"execution":c.state.execution().get(&rid),"revision":c.state.revision().number.to_string()}),
    ))
}
#[utoipa::path(get,path="/v1/deliveries/{id}",params(("id"=String,Path,description="Delivery uses its opaque order reference")),responses((status=200,body=Object),(status=404,body=ErrorBody)),security(("bearer"=[])))]
async fn delivery(
    State(api): State<Api>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    crate::persistence::check_async(&api).await?;
    let principal = read(&api, &headers)?;
    runtime::authorize_resource(&principal, &id)?;
    let oid = roadrunner_dispatch::OrderId::new(runtime::reference(&api, "order", &id)?);
    let c = api
        .inner
        .coordinator
        .lock()
        .map_err(|_| ApiError::internal())?;
    if !c.state.data().accepted.contains_key(&oid) {
        return Err(ApiError::missing());
    }
    Ok(Json(
        json!({"order":id,"assignment":c.state.data().assignments.get(&oid),"assigned_rider":c.state.data().assignments.get(&oid).map(|a|api.inner.namespace.reference("rider",a.rider.value())),"terms":c.state.data().accepted.get(&oid),"fulfillment":c.state.data().fulfillment.get(&oid),"revision":c.state.revision().number.to_string()}),
    ))
}
#[utoipa::path(get,path="/v1/commands/{key}",params(("key"=String,Path,description="Original Idempotency-Key scoped to authenticated principal"),("X-Roadrunner-Namespace"=String,Header,description="Original world reference; old volatile namespace returns unknown conflict")),responses((status=200,body=Envelope),(status=202,body=Envelope),(status=404,body=ErrorBody)),security(("bearer"=[])))]
async fn outcome(
    State(api): State<Api>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    crate::persistence::check_async(&api).await?;
    let principal = credential(&api, &headers)?;
    let namespace = headers
        .get("x-roadrunner-namespace")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::BAD_REQUEST,
                "missing_namespace",
                "X-Roadrunner-Namespace required for command lookup",
            )
        })?;
    if namespace != api.namespace() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "namespace_unavailable",
            "discarded volatile command outcomes are unknown",
        ));
    }
    let c = api
        .inner
        .coordinator
        .lock()
        .map_err(|_| ApiError::internal())?;
    let record = c
        .commands
        .get(&(principal.principal.clone(), key))
        .ok_or_else(ApiError::missing)?;
    if principal.resources.is_some() && record.command.namespace_wide() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "original command requires namespace-wide authorization",
        ));
    }
    for resource in record.command.resources() {
        runtime::authorize_resource(&principal, resource)?;
    }
    Ok((record.status, Json(record.envelope.clone())))
}
async fn route_query(
    api: Api,
    headers: HeaderMap,
    request: RouteRequest,
    multiple: bool,
    evidence: bool,
) -> Result<axum::response::Response, ApiError> {
    crate::persistence::check_async(&api).await?;
    read(&api, &headers)?;
    let export_permit = if evidence {
        Some(
            api.inner
                .export_admission
                .clone()
                .try_acquire_owned()
                .map_err(|_| {
                    ApiError::new(
                        StatusCode::SERVICE_UNAVAILABLE,
                        "visualization_busy",
                        "visualization export admission unavailable; retry later",
                    )
                })?,
        )
    } else {
        None
    };
    let (traffic, lock_wait_ns, lock_hold_ns) = {
        let capture_started = std::time::Instant::now();
        let coordinator = api
            .inner
            .coordinator
            .lock()
            .map_err(|_| ApiError::internal())?;
        let capture_acquired = std::time::Instant::now();
        let traffic = coordinator.traffic.clone();
        drop(coordinator);
        (
            traffic,
            capture_acquired.duration_since(capture_started).as_nanos(),
            capture_acquired.elapsed().as_nanos(),
        )
    };
    if evidence {
        tracing::debug!(target:"roadrunner::visualization",lock_wait_ns,lock_hold_ns,"captured routing context");
    }
    let permit = if evidence {
        api.inner.workers.clone().try_acquire_owned().map_err(|_| {
            ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "visualization_busy",
                "visualization worker unavailable; retry later",
            )
        })?
    } else {
        api.inner
            .workers
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| ApiError::internal())?
    };
    tokio::task::spawn_blocking(move || {
        let mut response =
            crate::routing::query(&api, &request, traffic.as_ref(), multiple, evidence)?;
        if evidence {
            let bytes = encode_export(&response, 32 * 1024 * 1024)?;
            return axum::response::Response::builder()
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::new(HeldExportBody {
                    bytes: bytes.into(),
                    _permit: export_permit.ok_or_else(ApiError::internal)?,
                }))
                .map_err(|_| ApiError::internal());
        }
        if let Some(store) = &api.inner.storage {
            let records = store
                .lock()
                .map_err(|_| ApiError::internal())?
                .record_routes(&runtime::value(&request)?, &response)
                .map_err(crate::persistence::storage_error)?;
            response["route_records"] = runtime::value(records)?;
        }
        let result = Json(response).into_response();
        drop(permit);
        Ok(result)
    })
    .await
    .map_err(|_| ApiError::internal())?
}
#[utoipa::path(post,path="/v1/routes",request_body=RouteRequest,responses((status=200,body=RoutesResponse),(status=404,body=ErrorBody),(status=409,body=ErrorBody),(status=422,body=ErrorBody)),security(("bearer"=[])))]
async fn route(
    State(api): State<Api>,
    headers: HeaderMap,
    input: Result<Json<RouteRequest>, JsonRejection>,
) -> Result<axum::response::Response, ApiError> {
    route_query(api, headers, body(input)?, false, false).await
}
#[utoipa::path(post,path="/v1/routes/alternatives",request_body=RouteRequest,responses((status=200,body=RoutesResponse),(status=404,body=ErrorBody),(status=422,body=ErrorBody)),security(("bearer"=[])))]
async fn alternative_routes(
    State(api): State<Api>,
    headers: HeaderMap,
    input: Result<Json<RouteRequest>, JsonRejection>,
) -> Result<axum::response::Response, ApiError> {
    route_query(api, headers, body(input)?, true, false).await
}

#[utoipa::path(post,path="/v1/visualization/routes",request_body=RouteRequest,responses((status=200,body=Object),(status=413,body=ErrorBody)),security(("bearer"=[])))]
async fn visualization_route(
    State(api): State<Api>,
    headers: HeaderMap,
    input: Result<Json<RouteRequest>, JsonRejection>,
) -> Result<axum::response::Response, ApiError> {
    route_query(api, headers, body(input)?, false, true).await
}
#[utoipa::path(post,path="/v1/visualization/routes/alternatives",request_body=RouteRequest,responses((status=200,body=Object),(status=413,body=ErrorBody)),security(("bearer"=[])))]
async fn visualization_alternatives(
    State(api): State<Api>,
    headers: HeaderMap,
    input: Result<Json<RouteRequest>, JsonRejection>,
) -> Result<axum::response::Response, ApiError> {
    route_query(api, headers, body(input)?, true, true).await
}

macro_rules! create_handler {
    ($fn:ident,$path:literal,$ty:ty,$variant:ident) => {
        #[utoipa::path(post,path=$path,request_body=$ty,params(("Idempotency-Key"=String,Header)),responses((status=200,body=Envelope),(status=202,body=Envelope),(status=409,body=CommandFailure),(status=422,body=CommandFailure),(status=400,body=ErrorBody),(status=401,body=ErrorBody),(status=403,body=ErrorBody),(status=413,body=ErrorBody),(status=415,body=ErrorBody),(status=503,body=ErrorBody)),security(("bearer"=[])))]
        async fn $fn(State(api):State<Api>,headers:HeaderMap,input:Result<Json<$ty>,JsonRejection>)->Result<impl IntoResponse,ApiError>{command(api,headers,Command::$variant(body(input)?)).await}
    };
}
macro_rules! resource_handler {
    ($fn:ident,$method:ident,$path:literal,$ty:ty,$variant:ident) => {
        #[utoipa::path($method,path=$path,request_body=$ty,params(("id"=String,Path),("Idempotency-Key"=String,Header)),responses((status=200,body=Envelope),(status=202,body=Envelope),(status=409,body=CommandFailure),(status=422,body=CommandFailure),(status=400,body=ErrorBody),(status=401,body=ErrorBody),(status=403,body=ErrorBody),(status=413,body=ErrorBody),(status=415,body=ErrorBody),(status=503,body=ErrorBody)),security(("bearer"=[])))]
        async fn $fn(State(api):State<Api>,headers:HeaderMap,Path(id):Path<String>,input:Result<Json<$ty>,JsonRejection>)->Result<impl IntoResponse,ApiError>{command(api,headers,Command::$variant(id,body(input)?)).await}
    };
}
create_handler!(create_order, "/v1/orders", CreateOrder, CreateOrder);
create_handler!(create_rider, "/v1/riders", CreateRider, CreateRider);
create_handler!(dispatch, "/v1/dispatch", DispatchRequest, Dispatch);
create_handler!(fleet, "/v1/dispatch/fleet", FleetRequest, Fleet);
create_handler!(recovery, "/v1/dispatch/recovery", RecoveryRequest, Recovery);
create_handler!(traffic, "/v1/traffic", TrafficRequest, Traffic);
create_handler!(simulation, "/v1/simulations", SimulationRequest, Simulation);
resource_handler!(
    location,
    patch,
    "/v1/riders/{id}/location",
    UpdateLocation,
    Location
);
resource_handler!(
    cancel,
    post,
    "/v1/orders/{id}/cancel",
    NamespaceRequest,
    Cancel
);
resource_handler!(ready, post, "/v1/orders/{id}/ready", ObserveRequest, Ready);
resource_handler!(
    forecast,
    patch,
    "/v1/orders/{id}/forecast",
    ForecastRequest,
    Forecast
);
resource_handler!(
    start,
    post,
    "/v1/riders/{id}/actions/start",
    StartRequest,
    Start
);
resource_handler!(
    service,
    post,
    "/v1/riders/{id}/actions/service",
    NamespaceRequest,
    Service
);
resource_handler!(
    wait,
    post,
    "/v1/riders/{id}/actions/wait",
    NamespaceRequest,
    Wait
);
resource_handler!(
    arrival,
    post,
    "/v1/riders/{id}/actions/arrival",
    EffectRequest,
    Arrival
);
resource_handler!(
    completion,
    post,
    "/v1/riders/{id}/actions/completion",
    EffectRequest,
    Completion
);
resource_handler!(
    delay,
    post,
    "/v1/riders/{id}/actions/delay",
    DelayRequest,
    Delay
);

#[derive(OpenApi)]
#[openapi(
    paths(
        anchors,
        debug_graph,
        delivery_scene,
        visualization_route,
        visualization_alternatives,
        state,
        order,
        rider,
        delivery,
        outcome,
        route,
        alternative_routes,
        create_order,
        create_rider,
        dispatch,
        fleet,
        recovery,
        traffic,
        simulation,
        location,
        cancel,
        ready,
        forecast,
        start,
        service,
        wait,
        arrival,
        completion,
        delay
    ),
    components(schemas(Envelope, ErrorBody)),
    info(
        title = "Roadrunner Phase 20 API",
        description = "One serialized operational authority with optional PostgreSQL/PostGIS durability. Bearer capabilities and Idempotency-Key are required for commands. Clients supply intent and explicit graph projections; server owns identities, clocks, policy, execution and temporal publication. All numeric times are absolute seconds in /v1/state time_domain. Simulation is isolated. Durable mode preserves outcomes across restart under one fenced writer. Event consumers process committed integration facts; they cannot bypass operational commands. No multi-primary guarantee."
    )
)]
struct ApiDoc;
pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    let mut doc = ApiDoc::openapi();
    if let Some(c) = doc.components.as_mut() {
        c.add_security_scheme(
            "bearer",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
        );
    }
    doc
}
pub(crate) fn router(api: Api) -> Router {
    Router::new()
        .route("/v1/state", get(state))
        .route("/v1/visualization/anchors", get(anchors))
        .route("/v1/visualization/graph", get(debug_graph))
        .route("/v1/visualization/deliveries", post(delivery_scene))
        .route("/v1/routes", post(route))
        .route("/v1/visualization/routes", post(visualization_route))
        .route(
            "/v1/visualization/routes/alternatives",
            post(visualization_alternatives),
        )
        .route("/v1/routes/alternatives", post(alternative_routes))
        .route("/v1/orders", post(create_order))
        .route("/v1/orders/{id}", get(order))
        .route("/v1/orders/{id}/cancel", post(cancel))
        .route("/v1/orders/{id}/ready", post(ready))
        .route("/v1/orders/{id}/forecast", patch(forecast))
        .route("/v1/riders", post(create_rider))
        .route("/v1/riders/{id}", get(rider))
        .route("/v1/riders/{id}/location", patch(location))
        .route("/v1/riders/{id}/actions/start", post(start))
        .route("/v1/riders/{id}/actions/service", post(service))
        .route("/v1/riders/{id}/actions/wait", post(wait))
        .route("/v1/riders/{id}/actions/arrival", post(arrival))
        .route("/v1/riders/{id}/actions/completion", post(completion))
        .route("/v1/riders/{id}/actions/delay", post(delay))
        .route("/v1/dispatch", post(dispatch))
        .route("/v1/dispatch/fleet", post(fleet))
        .route("/v1/dispatch/recovery", post(recovery))
        .route("/v1/deliveries/{id}", get(delivery))
        .route("/v1/commands/{key}", get(outcome))
        .route("/v1/simulations", post(simulation))
        .route("/v1/traffic", post(traffic))
        .fallback(|| async { ApiError::missing() })
        .method_not_allowed_fallback(|| async {
            ApiError::new(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "method unsupported for this resource",
            )
        })
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(api)
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", openapi()))
}

// Encoding is bounded and runs on the worker; body backpressure retains its admission permit.
fn encode_export(value: &Value, limit: usize) -> Result<Vec<u8>, ApiError> {
    struct Budget {
        remaining: usize,
        bytes: Vec<u8>,
    }
    impl std::io::Write for Budget {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.remaining = self
                .remaining
                .checked_sub(bytes.len())
                .ok_or_else(|| std::io::Error::other("export byte limit"))?;
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Budget {
        remaining: limit,
        bytes: Vec::new(),
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| {
        ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "visualization_byte_limit",
            "visualization response exceeds serialized byte budget",
        )
    })?;
    Ok(writer.bytes)
}
struct HeldExportBody {
    bytes: axum::body::Bytes,
    _permit: tokio::sync::OwnedSemaphorePermit,
}
impl http_body::Body for HeldExportBody {
    type Data = axum::body::Bytes;
    type Error = std::convert::Infallible;
    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let this = self.get_mut();
        if this.bytes.is_empty() {
            std::task::Poll::Ready(None)
        } else {
            let n = this.bytes.len().min(64 * 1024);
            std::task::Poll::Ready(Some(Ok(http_body::Frame::data(this.bytes.split_to(n)))))
        }
    }
    fn is_end_stream(&self) -> bool {
        self.bytes.is_empty()
    }
    fn size_hint(&self) -> http_body::SizeHint {
        http_body::SizeHint::with_exact(self.bytes.len() as u64)
    }
}
#[cfg(test)]
mod export_budget_tests {
    #[tokio::test]
    async fn admission_rejects_without_consuming_operational_workers() {
        let (api, _, digest) = crate::test_support::fixture();
        let admission = api
            .inner
            .export_admission
            .clone()
            .try_acquire_many_owned(4)
            .unwrap_or_else(|e| panic!("{e}"));
        let request = serde_json::json!({"origin":crate::test_support::location(&digest,0),"destination":crate::test_support::location(&digest,1)});
        let response = crate::test_support::request(
            &api,
            "POST",
            "/v1/visualization/routes",
            None,
            request,
            Some(crate::test_support::TOKEN),
        )
        .await;
        assert_eq!(response.0, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.1["code"], "visualization_busy");
        assert_eq!(api.inner.workers.available_permits(), 4);
        drop(admission);
        assert_eq!(api.inner.export_admission.available_permits(), 4);
    }
    #[tokio::test]
    async fn response_body_holds_only_export_slot_and_read_capability_is_required() {
        let (mut api, _, digest) = crate::test_support::fixture();
        std::sync::Arc::get_mut(&mut api.inner)
            .unwrap_or_else(|| panic!("unique fixture"))
            .config
            .credentials
            .insert(
                "denied-test-token-1234".into(),
                crate::Credential {
                    principal: "denied".into(),
                    read: false,
                    write: false,
                    simulate: false,
                    observation_source: None,
                    resources: None,
                },
            );
        let request = serde_json::json!({"origin":crate::test_support::location(&digest,0),"destination":crate::test_support::location(&digest,1)});
        let denied = crate::test_support::request(
            &api,
            "POST",
            "/v1/visualization/routes",
            None,
            request.clone(),
            Some("denied-test-token-1234"),
        )
        .await;
        assert_eq!(denied.0, axum::http::StatusCode::FORBIDDEN);
        let headers = axum::http::HeaderMap::from_iter([(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer test-operator-token-1234"),
        )]);
        let response = super::route_query(
            api.clone(),
            headers,
            serde_json::from_value(request).unwrap_or_else(|e| panic!("{e}")),
            false,
            true,
        )
        .await
        .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(api.inner.workers.available_permits(), 4);
        assert_eq!(api.inner.export_admission.available_permits(), 3);
        drop(response);
        assert_eq!(api.inner.export_admission.available_permits(), 4);
    }
    #[test]
    fn exact_encoded_bytes_are_bounded() {
        assert!(super::encode_export(&serde_json::json!({"x":"abc"}), 11).is_ok());
        assert!(super::encode_export(&serde_json::json!({"x":"abc"}), 10).is_err());
    }
    #[test]
    fn slow_or_dropped_body_retains_then_releases_admission() {
        let workers = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
        let permit = workers
            .clone()
            .try_acquire_owned()
            .unwrap_or_else(|e| panic!("{e}"));
        let body = super::HeldExportBody {
            bytes: axum::body::Bytes::from_static(b"data"),
            _permit: permit,
        };
        assert!(workers.clone().try_acquire_owned().is_err());
        drop(body);
        assert!(workers.try_acquire_owned().is_ok());
    }
}

#[cfg(test)]
mod anchor_catalog_tests {
    use super::*;
    use roadrunner_core::{geo::CanonicalCoordinate, graph::*};

    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn pages_are_capped_and_dense_order_is_preserved() {
        let (original, clock, _) = crate::test_support::fixture();
        let mut builder = GraphBuilder::new(
            GraphSnapshotId::new(99),
            GraphMetadata::new(
                "synthetic/v1",
                "test",
                GraphBuildIdentity::new("anchor-pages", "v1", "v1", "synthetic"),
            ),
        );
        for i in 0..501_u32 {
            builder
                .add_node(
                    BuilderNodeId::new(u64::from(i)),
                    CanonicalCoordinate::new(0, i32::try_from(i).unwrap()).unwrap(),
                )
                .unwrap();
        }
        let api = Api::with_clock(
            builder.finalize().unwrap(),
            original.inner.config.clone(),
            original.inner.namespace,
            clock,
        )
        .unwrap();
        let headers = HeaderMap::from_iter([(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", crate::test_support::TOKEN)
                .parse()
                .unwrap(),
        )]);
        let first = anchors(
            State(api.clone()),
            headers.clone(),
            Query(AnchorPage { offset: 0 }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(first["anchors"].as_array().unwrap().len(), 500);
        assert_eq!(first["next_offset"], 500);
        let last = anchors(State(api), headers, Query(AnchorPage { offset: 500 }))
            .await
            .unwrap()
            .0;
        assert_eq!(last["anchors"].as_array().unwrap().len(), 1);
        assert_eq!(last["anchors"][0]["node"], 500);
        assert_eq!(last["next_offset"], json!(null));
        assert_eq!(last["graph"], first["graph"]);
    }
}

#[cfg(test)]
mod graph_admission_tests {
    #[tokio::test]
    #[allow(clippy::unwrap_used)]
    async fn graph_read_rejects_saturation_and_holds_body_admission() {
        let (api, _, digest) = crate::test_support::fixture();
        let held = api
            .inner
            .export_admission
            .clone()
            .acquire_many_owned(4)
            .await
            .unwrap();
        let headers = axum::http::HeaderMap::from_iter([(
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderValue::from_static("Bearer test-operator-token-1234"),
        )]);
        let query = || crate::graph_debug::GraphQuery {
            graph_digest: digest.clone(),
            node_offset: 0,
            edge_offset: 0,
            objective: crate::model::Objective::TravelTime,
        };
        let e = super::debug_graph(
            axum::extract::State(api.clone()),
            headers.clone(),
            axum::extract::Query(query()),
        )
        .await
        .unwrap_err();
        assert_eq!(e.status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        drop(held);
        let response = super::debug_graph(
            axum::extract::State(api.clone()),
            headers,
            axum::extract::Query(query()),
        )
        .await
        .unwrap();
        assert_eq!(api.inner.workers.available_permits(), 4);
        assert_eq!(api.inner.export_admission.available_permits(), 3);
        drop(response);
        assert_eq!(api.inner.export_admission.available_permits(), 4);
    }
}
