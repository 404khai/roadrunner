//! Bounded Phase 33 stage probe. Diagnostics never enter domain evidence.
use roadrunner_core::{
    cost::{RoutingContext, TravelTimeCost},
    geo::{CanonicalCoordinate, KilometersPerHour, Seconds},
    graph::{
        AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph,
        GraphBuildIdentity, GraphBuilder, GraphMetadata, GraphSnapshotId, NodeId,
        decode_graph_artifact, encode_graph_artifact,
    },
    routing::{TravelTimeHaversine, astar, dijkstra},
};
use roadrunner_dispatch::{
    Availability, CandidatePolicy, CapacityUnits, CoreRouteProvider, DispatchInstant,
    DispatchSnapshot, FulfillmentState, Order, OrderId, OrderReadiness, RiderId, RiderPlan,
    RiderProfile, RiderState, RoutingAnchor, RoutingAnchors, RoutingEpoch, TrafficContext, World,
    WorldData, generate_candidates, validate_world,
};
use roadrunner_simulation::{
    ActualReadiness, DispatchPolicy, OrderInput, RiderInput, SimulationScenario, TrafficChange,
    TrafficOverride, simulate, simulate_scene,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, error::Error, hint::black_box, time::Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: u32,
    case_id: String,
    workload: String,
    grid_side: u32,
    artifact_path: Option<String>,
    riders: u32,
    orders: u32,
    history_orders: u32,
    queries: u32,
    seed: u32,
    capture: bool,
    churn: bool,
    diagnostics: bool,
}
fn ns(t: Instant) -> u128 {
    t.elapsed().as_nanos()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn graph(input: &Input) -> Result<FrozenGraph> {
    if let Some(path) = &input.artifact_path {
        return Ok(decode_graph_artifact(&std::fs::read(path)?)?);
    }
    let side = input.grid_side;
    let mut b = GraphBuilder::new(
        GraphSnapshotId::new(33),
        GraphMetadata::new(
            "phase33-synthetic/v1",
            "synthetic",
            GraphBuildIdentity::new("phase33-grid", "v1", "v1", "bidirectional-orthogonal"),
        ),
    );
    let point = |id: u32| {
        CanonicalCoordinate::new(
            65_000_000 + i32::try_from(id / side).unwrap_or(0) * 100,
            33_000_000 + i32::try_from(id % side).unwrap_or(0) * 100,
        )
    };
    for id in 0..side * side {
        b.add_node(BuilderNodeId::new(u64::from(id)), point(id)?)?;
    }
    let properties = EdgeProperties::new(KilometersPerHour::new(36.0)?, None, AccessClass::General);
    let mut segment = 0;
    for from in 0..side * side {
        for to in [
            ((from % side) + 1 < side).then_some(from + 1),
            (from / side + 1 < side).then_some(from + side),
        ]
        .into_iter()
        .flatten()
        {
            b.add_segment(
                BuilderSegmentId::new(segment),
                BuilderNodeId::new(u64::from(from)),
                BuilderNodeId::new(u64::from(to)),
                vec![point(from)?, point(to)?],
                Some(properties),
                Some(properties),
            )?;
            segment += 1;
        }
    }
    Ok(b.finalize()?.with_forbidden_maneuvers(Vec::new())?)
}
fn anchor(g: &FrozenGraph, node: u32) -> Result<RoutingAnchor> {
    Ok(RoutingAnchor {
        node: NodeId::new(node),
        coordinate: g
            .node(NodeId::new(node))
            .ok_or("absent probe node")?
            .coordinate(),
        graph_digest: g.metadata().snapshot_digest().into(),
    })
}
fn candidates(g: &FrozenGraph, input: &Input) -> Result<Value> {
    let mut data = WorldData::default();
    let mut anchors = RoutingAnchors::default();
    let nodes = u32::try_from(g.node_count())?;
    for id in 0..input.riders {
        let rider = RiderId::new(u64::from(id));
        let a = anchor(g, (id + input.seed) % nodes)?;
        data.profiles.insert(
            rider,
            RiderProfile {
                id: rider,
                routing_profile: g.metadata().routing_profile().into(),
                max_capacity: CapacityUnits::new(1),
            },
        );
        data.riders.insert(
            rider,
            RiderState {
                coordinate: a.coordinate,
                availability: Availability::Available,
            },
        );
        data.plans.insert(rider, RiderPlan::default());
        anchors.riders.insert(rider, a);
    }
    for id in 0..=input.history_orders {
        let order = OrderId::new(u64::from(id));
        let a = anchor(g, id % nodes)?;
        data.orders.insert(
            order,
            Order {
                id: order,
                pickup: a.coordinate,
                dropoff: a.coordinate,
                created_at: DispatchInstant::new(0.0)?,
                deadline: None,
                demand: CapacityUnits::new(1),
            },
        );
        data.readiness.insert(
            order,
            OrderReadiness {
                expected_at: Some(DispatchInstant::new(0.0)?),
                observed_at: Some(DispatchInstant::new(0.0)?),
            },
        );
        data.fulfillment.insert(
            order,
            if id == 0 {
                FulfillmentState::AwaitingPickup
            } else {
                FulfillmentState::Delivered {
                    rider: RiderId::new(0),
                    picked_up_at: DispatchInstant::new(0.0)?,
                    at: DispatchInstant::new(0.0)?,
                }
            },
        );
        anchors.pickups.insert(order, a.clone());
        anchors.dropoffs.insert(order, a);
    }
    let world = World::new(33, data)?;
    let provider = CoreRouteProvider::new(g, TrafficContext::FreeFlow)?;
    let t = Instant::now();
    validate_world(world.data())?;
    let validation_ns = ns(t);
    let t = Instant::now();
    let snapshot = DispatchSnapshot::new(
        &world,
        DispatchInstant::new(0.0)?,
        RoutingEpoch(DispatchInstant::new(0.0)?),
        &provider,
        &anchors,
    )?;
    let snapshot_ns = ns(t);
    let t = Instant::now();
    let mut selected = Vec::new();
    for _ in 0..input.queries {
        selected = black_box(generate_candidates(
            &snapshot,
            OrderId::new(0),
            CandidatePolicy::Exhaustive,
        )?);
    }
    let exhaustive_ns = ns(t);
    let t = Instant::now();
    let mut spatial = Vec::new();
    for _ in 0..input.queries {
        spatial = black_box(generate_candidates(
            &snapshot,
            OrderId::new(0),
            CandidatePolicy::Spatial {
                radius: roadrunner_core::geo::Meters::new(5000.0)?,
                limit: 10,
            },
        )?);
    }
    Ok(
        json!({"validation_ns":validation_ns,"snapshot_ns":snapshot_ns,"exhaustive_ns":exhaustive_ns,"spatial_eligibility_build_query_ns":ns(t),"queries_per_policy":input.queries,"riders":input.riders,"retained_delivered_orders":input.history_orders,"selected":selected,"spatial":spatial,"semantic_hash":hash(&serde_json::to_vec(&(selected,spatial))?)}),
    )
}
fn routing(g: &FrozenGraph, input: &Input) -> Result<Value> {
    let from = NodeId::new(0);
    let to = if input.artifact_path.is_some() {
        g.edges().first().ok_or("empty probe graph")?.to()
    } else {
        NodeId::new(u32::try_from(g.node_count() - 1)?)
    };
    let context = RoutingContext::default();
    let t = Instant::now();
    let maximum = KilometersPerHour::new(
        g.edges()
            .iter()
            .map(|e| e.properties().effective_free_flow_speed().value())
            .fold(1.0_f64, f64::max),
    )?;
    let heuristic = TravelTimeHaversine::for_graph(g, maximum)?;
    let preparation_ns = ns(t);
    let t = Instant::now();
    let mut d = None;
    for _ in 0..input.queries {
        d = Some(black_box(dijkstra(g, from, to, &TravelTimeCost, &context)?));
    }
    let dijkstra_ns = ns(t);
    let t = Instant::now();
    let mut a = None;
    for _ in 0..input.queries {
        a = Some(black_box(astar(
            g,
            from,
            to,
            &TravelTimeCost,
            &heuristic,
            &context,
        )?));
    }
    let astar_prepared_ns = ns(t);
    let d = d.ok_or("zero queries")?;
    let a = a.ok_or("zero queries")?;
    if d.total_cost() != a.total_cost() {
        return Err("A*/Dijkstra objective mismatch".into());
    }
    Ok(
        json!({"origin":from,"destination":to,"queries_per_algorithm":input.queries,"heuristic_preparation_ns":preparation_ns,"dijkstra_ns":dijkstra_ns,"astar_prepared_ns":astar_prepared_ns,"dijkstra_expanded_states":d.expanded_states(),"astar_expanded_states":a.expanded_states(),"semantic_hash":hash(&serde_json::to_vec(&(d,a))?)}),
    )
}
fn simulation(g: &FrozenGraph, input: &Input) -> Result<Value> {
    let nodes = u32::try_from(g.node_count())?;
    let scenario = SimulationScenario {
        scenario_id: Some(input.case_id.clone()),
        schema_version: 1,
        seed: u64::from(input.seed),
        start_seconds: Seconds::ZERO,
        end_seconds: Seconds::new(f64::from(input.orders) * 100.0 + 10000.0)?,
        routing_epoch_seconds: Seconds::ZERO,
        graph_snapshot_digest: Some(g.metadata().snapshot_digest().into()),
        dispatch: DispatchPolicy::Basic,
        riders: (0..input.riders)
            .map(|id| RiderInput {
                id: u64::from(id),
                node: (id + input.seed) % nodes,
                capacity: 1,
                available: true,
            })
            .collect(),
        orders: (0..input.orders)
            .map(|id| {
                let created = Seconds::new(f64::from(id) * 100.0)?;
                Ok(OrderInput {
                    admission: None,
                    id: u64::from(id),
                    pickup_node: (id + input.seed) % nodes,
                    dropoff_node: (id + input.seed + 1) % nodes,
                    created_at_seconds: created,
                    expected_ready_at_seconds: Some(created),
                    actual_readiness: ActualReadiness::Fixed {
                        at_seconds: created,
                    },
                    deadline_seconds: None,
                    demand: 1,
                })
            })
            .collect::<Result<Vec<_>>>()?,
        initial_traffic: vec![],
        traffic_changes: if input.churn {
            vec![TrafficChange {
                at_seconds: Seconds::new(150.0)?,
                overrides: vec![TrafficOverride {
                    edge_id: 0,
                    multiplier: roadrunner_core::cost::TrafficMultiplier::new(2.0)?,
                }],
            }]
        } else {
            vec![]
        },
        dynamic_events: vec![],
    };
    let scenario_bytes = serde_json::to_vec(&scenario)?;
    let t = Instant::now();
    let result = if input.capture {
        simulate_scene(g, &scenario)?
    } else {
        simulate(g, &scenario)?
    };
    let execution_ns = ns(t);
    let t = Instant::now();
    let bytes = serde_json::to_vec(&result)?;
    let serialization_ns = ns(t);
    let diagnostics = if input.diagnostics {
        let mut buffers = BTreeSet::new();
        let mut physical_records = 0;
        for action in &result.trace.starts {
            if buffers.insert(action.traffic.as_ptr()) {
                physical_records += action.traffic.capacity();
            }
        }
        json!({"logical_traffic_records":result.trace.starts.iter().map(|s|s.traffic.len()).sum::<usize>(),"distinct_traffic_buffers":buffers.len(),"traffic_vector_capacity_bytes":physical_records*size_of::<TrafficOverride>(),"payload_scope":"live Vec element capacity only; excludes allocator/Arc headers, other data and RSS","scene_frames":result.scene_frames.as_ref().map_or(0,Vec::len),"source_transition_records":result.trace.history.len(),"recognized_effect_records":result.trace.effects.len()})
    } else {
        Value::Null
    };
    Ok(
        json!({"scenario_sha256":hash(&scenario_bytes),"execution_ns":execution_ns,"serialization_ns":serialization_ns,"result_bytes":bytes.len(),"semantic_hash":hash(&bytes),"logical_seconds":result.ended_at.value()-result.started_at.value(),"events":result.events.len(),"starts":result.trace.starts.len(),"future_queue_entries":result.future_events,"summary":result.summary,"diagnostics":diagnostics}),
    )
}
fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: phase33_probe manifest.json")?;
    let bytes = std::fs::read(path)?;
    let input: Input = serde_json::from_slice(&bytes)?;
    if input.schema_version != 1
        || !(2..=64).contains(&input.grid_side)
        || input.riders == 0
        || input.riders > 256
        || input.orders > 256
        || input.history_orders > 1024
        || !(1..=100).contains(&input.queries)
        || input.seed > 10000
    {
        return Err("probe manifest exceeds bounded Phase 33 envelope".into());
    }
    let t = Instant::now();
    let g = graph(&input)?;
    let graph_load_or_build_ns = ns(t);
    let result = match input.workload.as_str() {
        "candidates" => candidates(&g, &input)?,
        "routing" => routing(&g, &input)?,
        "simulation" => simulation(&g, &input)?,
        "graph" => {
            let t = Instant::now();
            let cloned = black_box(g.clone());
            let clone_ns = ns(t);
            drop(cloned);
            let t = Instant::now();
            let encoded = encode_graph_artifact(&g)?;
            let encode_ns = ns(t);
            let t = Instant::now();
            let decoded = decode_graph_artifact(&encoded)?;
            json!({"clone_ns":clone_ns,"encode_ns":encode_ns,"decode_verify_ns":ns(t),"artifact_bytes":encoded.len(),"semantic_hash":decoded.metadata().snapshot_digest()})
        }
        _ => return Err("unknown probe workload".into()),
    };
    println!(
        "{}",
        serde_json::to_string(
            &json!({"schema_version":1,"manifest_sha256":hash(&bytes),"manifest":serde_json::from_slice::<Value>(&bytes)?,"graph_digest":g.metadata().snapshot_digest(),"graph_nodes":g.node_count(),"graph_edges":g.edge_count(),"maneuver_aware":g.metadata().turn_restrictions_enforced(),"graph_load_or_build_ns":graph_load_or_build_ns,"result":result})
        )?
    );
    Ok(())
}
