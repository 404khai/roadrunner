use crate::error::ApiError;
use roadrunner_core::geo::Coordinate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Point {
    #[schema(minimum = -90, maximum = 90)]
    pub latitude: f64,
    #[schema(minimum = -180, maximum = 180)]
    pub longitude: f64,
}
impl Point {
    pub fn coordinate(&self) -> Result<Coordinate, ApiError> {
        Coordinate::new(self.latitude, self.longitude).map_err(ApiError::invalid)
    }
}
/// A graph-qualified, explicit node projection. No snapping or off-road connector.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct Location {
    pub graph_digest: String,
    pub node: u32,
    pub coordinate: Point,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RouteRequest {
    pub origin: Location,
    pub destination: Location,
    /// RFC 3339 UTC/offset timestamp. Defaults to the authoritative server clock.
    pub departure_time: Option<String>,
    #[serde(default)]
    pub algorithm: Algorithm,
    #[serde(default)]
    pub objective: Objective,
    /// Count including primary, 1..=5; used only by alternatives endpoint.
    pub alternatives: Option<usize>,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Algorithm {
    #[default]
    Dijkstra,
    Astar,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Objective {
    Distance,
    #[default]
    TravelTime,
    TrafficAware,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateOrder {
    pub namespace: String,
    pub pickup: Location,
    pub dropoff: Location,
    pub demand: u64,
    /// Absolute seconds in the advertised server time domain.
    pub deadline_seconds: Option<f64>,
    /// Configured policy name; clients cannot assert accepted terms.
    pub policy: String,
    pub expected_ready_seconds: Option<f64>,
    /// Required together with `expected_ready_seconds`.
    pub forecast_valid_until_seconds: Option<f64>,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateRider {
    pub namespace: String,
    pub location: Location,
    pub capacity: u64,
    pub available: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[allow(clippy::struct_field_names)] // Public source identity name distinguishes it from HTTP command identity.
pub(crate) struct Observation {
    /// Source bound to the authenticated observer credential.
    pub source: String,
    /// Stable source identity, independently recognized across HTTP commands.
    pub observation_id: String,
    /// Contiguous sequence per source/resource, beginning at 1.
    pub sequence: u64,
    /// Absolute seconds; future observations reject. Receipt is recorded separately.
    pub observed_at_seconds: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct UpdateLocation {
    pub namespace: String,
    pub location: Location,
    pub available: bool,
    pub observation: Observation,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DispatchRequest {
    pub namespace: String,
    pub order: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct FleetRequest {
    pub namespace: String,
    pub orders: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecoveryRequest {
    pub namespace: String,
    pub trigger: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct NamespaceRequest {
    pub namespace: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObserveRequest {
    pub namespace: String,
    pub observation: Observation,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct EffectRequest {
    pub namespace: String,
    pub action: String,
    pub generation: String,
    pub effect_id: String,
    pub observation: Observation,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct StartRequest {
    pub namespace: String,
    pub plan_revision: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DelayRequest {
    pub namespace: String,
    pub seconds: f64,
    pub observation: Observation,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ForecastRequest {
    pub namespace: String,
    pub expected_ready_seconds: f64,
    pub valid_until_seconds: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TrafficRequest {
    pub namespace: String,
    pub graph_digest: String,
    /// Complete replacement; omitted directed edges return to normal traffic.
    pub overrides: Vec<TrafficOverride>,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TrafficOverride {
    pub edge: u32,
    pub multiplier: f64,
}
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SimulationRequest {
    pub namespace: String,
    /// Existing versioned `SimulationScenario` contract; runs on the pinned server graph.
    #[schema(value_type = SimulationScenarioSchema)]
    pub scenario: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, ToSchema)]
pub(crate) struct Envelope {
    pub schema_version: u32,
    pub namespace: String,
    pub command: String,
    /// pending, committed, completed, noncommit, or rejected.
    pub outcome: String,
    /// True only when this command published a business effect. Context adoption is recorded separately in revision/history.
    pub published: bool,
    /// Lossless decimal revision at completion, independent of command count.
    pub revision: String,
    /// Immutable original result/evidence, never recomputed for a retry.
    pub result: serde_json::Value,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct GeoJsonLineString {
    #[serde(rename = "type")]
    #[schema(example = "LineString")]
    kind: String,
    /// `GeoJSON` longitude/latitude positions in directed traversal order.
    coordinates: Vec<[f64; 2]>,
}
#[derive(Serialize, ToSchema)]
pub(crate) struct ObjectiveCost {
    /// distance or `travel_time`; value units are meters or seconds respectively.
    kind: String,
    value: f64,
}
#[derive(Serialize, ToSchema)]
pub(crate) struct RouteView {
    rank: usize,
    graph_digest: String,
    routing_profile: String,
    /// dijkstra, `a_star`, or yen.
    algorithm: String,
    nodes: Vec<u32>,
    edges: Vec<u32>,
    distance_meters: f64,
    eta_seconds: f64,
    cost: ObjectiveCost,
    expanded_states: usize,
    geometry: GeoJsonLineString,
}
#[derive(Serialize, ToSchema)]
pub(crate) struct RoutesResponse {
    routes: Vec<RouteView>,
    objective: Objective,
    departure_seconds: f64,
    traffic_digest: Option<String>,
    /// Alternatives only: true when enumeration hit a budget.
    truncated: Option<bool>,
    /// Alternatives only: `requested_count`, exhausted, `cost_limit`, or budget.
    termination: Option<String>,
    #[schema(value_type = Object)]
    options: Option<serde_json::Value>,
    search_states: Option<usize>,
}

// Documentation mirrors the versioned simulation wire contract. Runtime validation
// still deserializes SimulationScenario in the simulation crate; the API has no model authority.
#[derive(Serialize, ToSchema)]
pub(crate) struct SimulationScenarioSchema {
    scenario_id: Option<String>,
    #[schema(minimum = 1, maximum = 4)]
    schema_version: u32,
    seed: u64,
    start_seconds: f64,
    end_seconds: f64,
    routing_epoch_seconds: f64,
    graph_snapshot_digest: String,
    dispatch: SimulationDispatchSchema,
    riders: Vec<SimulationRiderSchema>,
    orders: Vec<SimulationOrderSchema>,
    initial_traffic: Option<Vec<SimulationTrafficSchema>>,
    traffic_changes: Option<Vec<SimulationTrafficChangeSchema>>,
    /// Optional schema 4 dynamic events; see docs/simulation.md and phase-19 fixtures.
    dynamic_events: Option<Vec<serde_json::Value>>,
}
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[expect(
    dead_code,
    reason = "Schema reflection only; simulation owns runtime deserialization"
)]
enum SimulationDispatchSchema {
    Basic,
    NearestRider,
    LowestPickupEta,
    LowestCompletionTime,
    PreparationAware {
        idle_penalty_weight: f64,
    },
    MultiOrder {
        work_budget: u64,
        forecast_validity_seconds: f64,
    },
    FleetBatch {
        work_budget: u64,
        forecast_validity_seconds: f64,
        algorithm: FleetAlgorithmSchema,
    },
    Dynamic {
        work_budget: u64,
        forecast_validity_seconds: f64,
        algorithm: FleetAlgorithmSchema,
        recovery: RecoveryPolicySchema,
    },
}
#[derive(Serialize, ToSchema)]
#[expect(
    dead_code,
    reason = "Schema reflection only; simulation owns runtime deserialization"
)]
enum FleetAlgorithmSchema {
    Greedy,
    LocalSearch,
    MultiStartLocal,
}
#[derive(Serialize, ToSchema)]
struct RecoveryPolicySchema {
    version: u32,
    reroute_penalty: f64,
    assignment_stability_penalty: f64,
    minimum_improvement: f64,
    cooldown: f64,
}
#[derive(Serialize, ToSchema)]
struct SimulationRiderSchema {
    id: u64,
    node: u32,
    capacity: u64,
    available: bool,
}
#[derive(Serialize, ToSchema)]
struct SimulationOrderSchema {
    id: u64,
    pickup_node: u32,
    dropoff_node: u32,
    created_at_seconds: f64,
    expected_ready_at_seconds: Option<f64>,
    actual_readiness: ActualReadinessSchema,
    deadline_seconds: Option<f64>,
    demand: u64,
    admission: Option<OrderPolicySchema>,
}
#[derive(Serialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[expect(
    dead_code,
    reason = "Schema reflection only; simulation owns runtime deserialization"
)]
enum ActualReadinessSchema {
    Fixed { at_seconds: f64 },
    SeededDelay { min_seconds: f64, max_seconds: f64 },
}
#[derive(Serialize, ToSchema)]
struct OrderPolicySchema {
    id: String,
    version: u32,
    deadline: AdmissionDeadlineSchema,
    max_completion_delay: Option<f64>,
    pickup_service: f64,
    dropoff_service: f64,
    readiness: ReadinessRuleSchema,
}
#[derive(Serialize, ToSchema)]
#[expect(
    dead_code,
    reason = "Schema reflection only; simulation owns runtime deserialization"
)]
enum AdmissionDeadlineSchema {
    SoftObserved,
    Hard,
}
#[derive(Serialize, ToSchema)]
#[expect(
    dead_code,
    reason = "Schema reflection only; simulation owns runtime deserialization"
)]
enum ReadinessRuleSchema {
    LegacyV1,
    ValidForecastV1,
    CreatedAtFallbackV1,
}
#[derive(Serialize, ToSchema)]
struct SimulationTrafficSchema {
    edge_id: u32,
    multiplier: f64,
}
#[derive(Serialize, ToSchema)]
struct SimulationTrafficChangeSchema {
    at_seconds: f64,
    overrides: Vec<SimulationTrafficSchema>,
}

#[derive(Serialize, ToSchema)]
#[serde(untagged)]
#[expect(
    dead_code,
    reason = "OpenAPI response union; HTTP chooses its actual response type"
)]
pub(crate) enum CommandFailure {
    Recorded(Envelope),
    BeforeReservation(crate::error::ErrorBody),
}
