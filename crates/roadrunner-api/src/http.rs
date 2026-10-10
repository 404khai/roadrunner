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
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
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
#[utoipa::path(get,path="/v1/orders/{id}",params(("id"=String,Path,description="Opaque namespace:order:id reference")),responses((status=200,body=Object),(status=404,body=ErrorBody)),security(("bearer"=[])))]
async fn order(
    State(api): State<Api>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
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
) -> Result<Json<Value>, ApiError> {
    read(&api, &headers)?;
    let traffic = api
        .inner
        .coordinator
        .lock()
        .map_err(|_| ApiError::internal())?
        .traffic
        .clone();
    let permit = api
        .inner
        .workers
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| ApiError::internal())?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        crate::routing::query(&api, &request, traffic.as_ref(), multiple).map(Json)
    })
    .await
    .map_err(|_| ApiError::internal())?
}
#[utoipa::path(post,path="/v1/routes",request_body=RouteRequest,responses((status=200,body=RoutesResponse),(status=404,body=ErrorBody),(status=409,body=ErrorBody),(status=422,body=ErrorBody)),security(("bearer"=[])))]
async fn route(
    State(api): State<Api>,
    headers: HeaderMap,
    input: Result<Json<RouteRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    route_query(api, headers, body(input)?, false).await
}
#[utoipa::path(post,path="/v1/routes/alternatives",request_body=RouteRequest,responses((status=200,body=RoutesResponse),(status=404,body=ErrorBody),(status=422,body=ErrorBody)),security(("bearer"=[])))]
async fn alternative_routes(
    State(api): State<Api>,
    headers: HeaderMap,
    input: Result<Json<RouteRequest>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    route_query(api, headers, body(input)?, true).await
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
        description = "One volatile authority. Bearer capabilities and Idempotency-Key are required for commands. Clients supply intent and explicit graph projections; server owns identities, clocks, policy, execution and temporal publication. All numeric times are absolute seconds in /v1/state time_domain. Simulation is isolated. No persistence or multi-writer guarantee."
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
        .route("/v1/routes", post(route))
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
