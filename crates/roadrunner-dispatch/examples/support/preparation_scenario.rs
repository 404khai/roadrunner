//! Shared synthetic fixture for the example and Phase 14 correctness tests.

use std::error::Error;

use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour, Seconds, haversine_distance};
use roadrunner_core::graph::{
    AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph, GraphBuildIdentity,
    GraphBuilder, GraphMetadata, GraphSnapshotId, NodeId,
};
use roadrunner_dispatch::{
    AssignmentDecision, Availability, CandidatePolicy, CapacityUnits, CoreRouteProvider,
    DecisionId, DispatchDecisionOutcome, DispatchInstant, DispatchSnapshot, FulfillmentState,
    Order, OrderId, OrderReadiness, PreparationAwareStrategy, RiderId, RiderPlan, RiderProfile,
    RiderState, RouteOutcome, RouteProvider, RoutingAnchor, RoutingAnchors, RoutingEpoch,
    TrafficContext, World, WorldData, basic_dispatch, onboard_load, preparation_aware_dispatch,
    validate_world,
};
use serde::Serialize;

pub type ScenarioResult<T> = Result<T, Box<dyn Error>>;

pub struct Scenario {
    pub graph: FrozenGraph,
    pub data: WorldData,
    pub anchors: RoutingAnchors,
    pub at: DispatchInstant,
    pub epoch: RoutingEpoch,
    pub order: OrderId,
    pub near: RiderId,
    pub far: RiderId,
}

fn example_graph() -> ScenarioResult<FrozenGraph> {
    let mut builder = GraphBuilder::new(
        GraphSnapshotId::new(14),
        GraphMetadata::new(
            "dispatch-motorcycle/v1",
            "synthetic preparation scenario",
            GraphBuildIdentity::new("phase14-scenario", "v1", "v1", "synthetic"),
        ),
    );
    let points = [
        CanonicalCoordinate::new(0, 0)?,
        CanonicalCoordinate::new(0, 10_000)?,
        CanonicalCoordinate::new(0, 30_000)?,
        CanonicalCoordinate::new(0, 40_000)?,
        CanonicalCoordinate::new(0, 50_000)?,
    ];
    for (id, point) in (0_u64..).zip(points) {
        builder.add_node(BuilderNodeId::new(id), point)?;
    }
    // Controlled free-flow travel: near → pickup 5m; far → pickup 15m; delivery 10m.
    for (id, from, to, duration) in [
        (0, 1_usize, 0_usize, 300.0),
        (1, 2, 0, 900.0),
        (2, 0, 3, 600.0),
    ] {
        let distance = haversine_distance(points[from].to_coordinate(), points[to].to_coordinate());
        builder.add_segment(
            BuilderSegmentId::new(id),
            BuilderNodeId::new(u64::try_from(from)?),
            BuilderNodeId::new(u64::try_from(to)?),
            vec![points[from], points[to]],
            Some(EdgeProperties::new(
                KilometersPerHour::new(distance.value() / duration * 3.6)?,
                None,
                AccessClass::General,
            )),
            None,
        )?;
    }
    Ok(builder.finalize()?)
}

impl Scenario {
    pub fn new(ready_after_seconds: f64) -> ScenarioResult<Self> {
        let graph = example_graph()?;
        let order = OrderId::new(1);
        let near = RiderId::new(1);
        let far = RiderId::new(2);
        let at = DispatchInstant::new(10_000.0)?;
        let epoch = RoutingEpoch(at);
        let anchor = |node| -> ScenarioResult<RoutingAnchor> {
            Ok(RoutingAnchor {
                coordinate: graph
                    .node(NodeId::new(node))
                    .ok_or("missing node")?
                    .coordinate(),
                graph_digest: graph.metadata().snapshot_digest().to_owned(),
                node: NodeId::new(node),
            })
        };
        let pickup = anchor(0)?;
        let dropoff = anchor(3)?;
        let mut data = WorldData::default();
        data.orders.insert(
            order,
            Order {
                id: order,
                pickup: pickup.coordinate,
                dropoff: dropoff.coordinate,
                created_at: at,
                deadline: None,
                demand: CapacityUnits::new(1),
            },
        );
        data.readiness.insert(
            order,
            OrderReadiness {
                expected_at: Some(at.checked_add(Seconds::new(ready_after_seconds)?)?),
                observed_at: None,
            },
        );
        data.fulfillment
            .insert(order, FulfillmentState::AwaitingPickup);
        let mut anchors = RoutingAnchors::default();
        anchors.pickups.insert(order, pickup);
        anchors.dropoffs.insert(order, dropoff);
        for (rider, node) in [(near, 1), (far, 2)] {
            let origin = anchor(node)?;
            data.profiles.insert(
                rider,
                RiderProfile {
                    id: rider,
                    routing_profile: graph.metadata().routing_profile().to_owned(),
                    max_capacity: CapacityUnits::new(1),
                },
            );
            data.riders.insert(
                rider,
                RiderState {
                    coordinate: origin.coordinate,
                    availability: Availability::Available,
                },
            );
            data.plans.insert(rider, RiderPlan::default());
            anchors.riders.insert(rider, origin);
        }
        Ok(Self {
            graph,
            data,
            anchors,
            at,
            epoch,
            order,
            near,
            far,
        })
    }

    pub fn world(&self) -> ScenarioResult<World> {
        Ok(World::new(14, self.data.clone())?)
    }
}

#[derive(Debug, Serialize)]
pub struct ObservedMetrics {
    pub rider: RiderId,
    pub pickup_arrival: DispatchInstant,
    pub pickup_completed_at: DispatchInstant,
    pub delivery_completed_at: DispatchInstant,
    pub waiting: Seconds,
    pub delivery_travel: Seconds,
    pub completion_time: Seconds,
}

#[derive(Debug, Serialize)]
pub struct ScenarioRun {
    pub decision: AssignmentDecision,
    pub observed: ObservedMetrics,
    pub final_fulfillment: FulfillmentState,
}

/// Each strategy starts from a fresh equivalent world and identical exogenous ready event.
/// The Phase 13 baseline may underestimate completion; execution obeys actual readiness.
pub fn run(
    scenario: &Scenario,
    preparation_aware: bool,
    actual_ready_at: DispatchInstant,
) -> ScenarioResult<ScenarioRun> {
    let mut world = scenario.world()?;
    let provider = CoreRouteProvider::new(&scenario.graph, TrafficContext::FreeFlow)?;
    let snapshot = DispatchSnapshot::new(
        &world,
        scenario.at,
        scenario.epoch,
        &provider,
        &scenario.anchors,
    )?;
    let decision = if preparation_aware {
        preparation_aware_dispatch(
            &snapshot,
            scenario.order,
            DecisionId::new(14),
            CandidatePolicy::Exhaustive,
            PreparationAwareStrategy::default(),
        )?
    } else {
        basic_dispatch(
            &snapshot,
            scenario.order,
            DecisionId::new(13),
            CandidatePolicy::Exhaustive,
        )?
    };
    let DispatchDecisionOutcome::Assigned(proposal) = decision.outcome() else {
        return Err("expected assigned scenario".into());
    };
    let rider = proposal.rider;
    // Execute both road legs against the fixed graph rather than reporting predictions.
    let RouteOutcome::RouteFound(first) = provider.route(
        scenario.anchors.riders[&rider].node,
        scenario.anchors.pickups[&scenario.order].node,
        scenario.epoch.departure_seconds(scenario.at)?,
    )?
    else {
        return Err("expected execution pickup route".into());
    };
    let arrival = scenario.at.checked_add(first.route.elapsed_travel_time())?;
    let pickup_completed_at = if arrival > actual_ready_at {
        arrival
    } else {
        actual_ready_at
    };
    world.commit(&decision)?;
    world.observe_ready(scenario.order, actual_ready_at)?;
    world.pickup(rider, scenario.order, pickup_completed_at)?;
    let RouteOutcome::RouteFound(leg) = provider.route(
        scenario.anchors.pickups[&scenario.order].node,
        scenario.anchors.dropoffs[&scenario.order].node,
        scenario.epoch.departure_seconds(pickup_completed_at)?,
    )?
    else {
        return Err("expected execution road route".into());
    };
    let delivery_completed_at = pickup_completed_at.checked_add(leg.route.elapsed_travel_time())?;
    world.deliver(rider, scenario.order, delivery_completed_at)?;
    validate_world(world.data())?;
    assert_eq!(world.data().plans[&rider], RiderPlan::default());
    assert!(world.data().assignments.is_empty());
    assert_eq!(onboard_load(world.data(), rider)?, CapacityUnits::new(0));
    let final_fulfillment = world.data().fulfillment[&scenario.order];
    let FulfillmentState::Delivered {
        at, picked_up_at, ..
    } = final_fulfillment
    else {
        return Err("expected completed delivery".into());
    };
    let observed = ObservedMetrics {
        rider,
        pickup_arrival: arrival,
        pickup_completed_at: picked_up_at,
        delivery_completed_at: at,
        waiting: picked_up_at.duration_since(arrival)?,
        delivery_travel: at.duration_since(picked_up_at)?,
        completion_time: at.duration_since(scenario.at)?,
    };
    Ok(ScenarioRun {
        decision,
        observed,
        final_fulfillment,
    })
}
