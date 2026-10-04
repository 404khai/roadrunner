//! Runnable Phase 13 assignment and shared execution transitions on synthetic roads.

use std::error::Error;

use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour, Meters};
use roadrunner_core::graph::{
    AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph, GraphBuildIdentity,
    GraphBuilder, GraphMetadata, GraphSnapshotId, NodeId,
};
use roadrunner_dispatch::{
    Availability, CandidatePolicy, CandidateResult, CapacityUnits, CoreRouteProvider, DecisionId,
    DispatchDecisionOutcome, DispatchInstant, DispatchSnapshot, FulfillmentState, Order, OrderId,
    OrderReadiness, RiderId, RiderPlan, RiderProfile, RiderState, RoutingAnchor, RoutingAnchors,
    RoutingEpoch, TrafficContext, World, WorldData, basic_dispatch, onboard_load, validate_world,
};

fn example_graph() -> Result<FrozenGraph, Box<dyn Error>> {
    let profile = "dispatch-motorcycle/v1";
    let mut builder = GraphBuilder::new(
        GraphSnapshotId::new(1),
        GraphMetadata::new(
            profile,
            "synthetic Phase 13 example",
            GraphBuildIdentity::new("basic-dispatch-example", "v1", "v1", "synthetic"),
        ),
    );
    let points = [
        CanonicalCoordinate::new(0, 0)?,
        CanonicalCoordinate::new(0, 10_000)?,
        CanonicalCoordinate::new(0, 20_000)?,
        CanonicalCoordinate::new(0, 30_000)?,
    ];
    for (id, point) in (0_u64..).zip(points) {
        builder.add_node(BuilderNodeId::new(id), point)?;
    }
    // Rider 9 is nearer geographically but its only road to pickup is slower.
    for (id, from, to, speed) in [(0, 1_usize, 0_usize, 3.6), (1, 2, 0, 72.0), (2, 0, 3, 36.0)] {
        builder.add_segment(
            BuilderSegmentId::new(id),
            BuilderNodeId::new(u64::try_from(from)?),
            BuilderNodeId::new(u64::try_from(to)?),
            vec![points[from], points[to]],
            Some(EdgeProperties::new(
                KilometersPerHour::new(speed)?,
                None,
                AccessClass::General,
            )),
            None,
        )?;
    }
    Ok(builder.finalize()?)
}

fn example_world(graph: &FrozenGraph) -> Result<(World, RoutingAnchors), Box<dyn Error>> {
    let profile = graph.metadata().routing_profile();
    let anchor = |node| -> Result<RoutingAnchor, Box<dyn Error>> {
        Ok(RoutingAnchor {
            coordinate: graph
                .node(NodeId::new(node))
                .ok_or("missing node")?
                .coordinate(),
            graph_digest: graph.metadata().snapshot_digest().to_owned(),
            node: NodeId::new(node),
        })
    };
    let order = OrderId::new(1);
    let near = RiderId::new(9);
    let fast = RiderId::new(2);
    let epoch = RoutingEpoch(DispatchInstant::new(0.0)?);
    let mut data = WorldData::default();
    let mut anchors = RoutingAnchors::default();
    let pickup = anchor(0)?;
    let dropoff = anchor(3)?;
    data.orders.insert(
        order,
        Order {
            id: order,
            pickup: pickup.coordinate,
            dropoff: dropoff.coordinate,
            created_at: epoch.0,
            deadline: None,
            demand: CapacityUnits::new(2),
        },
    );
    data.readiness.insert(
        order,
        OrderReadiness {
            expected_at: None,
            observed_at: Some(epoch.0),
        },
    );
    data.fulfillment
        .insert(order, FulfillmentState::AwaitingPickup);
    anchors.pickups.insert(order, pickup.clone());
    anchors.dropoffs.insert(order, dropoff.clone());
    for (rider, node) in [(near, 1), (fast, 2)] {
        let origin = anchor(node)?;
        data.profiles.insert(
            rider,
            RiderProfile {
                id: rider,
                routing_profile: profile.to_owned(),
                max_capacity: CapacityUnits::new(3),
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
    Ok((World::new(1, data)?, anchors))
}

fn main() -> Result<(), Box<dyn Error>> {
    let graph = example_graph()?;
    let (mut world, anchors) = example_world(&graph)?;
    let order = OrderId::new(1);
    let near = RiderId::new(9);
    let fast = RiderId::new(2);
    let at = DispatchInstant::new(100.0)?;
    let epoch = RoutingEpoch(DispatchInstant::new(0.0)?);
    let pickup = &anchors.pickups[&order];
    let dropoff = &anchors.dropoffs[&order];
    let provider = CoreRouteProvider::new(&graph, TrafficContext::FreeFlow)?;
    let snapshot = DispatchSnapshot::new(&world, at, epoch, &provider, &anchors)?;
    let complete = basic_dispatch(
        &snapshot,
        order,
        DecisionId::new(1),
        CandidatePolicy::Exhaustive,
    )?;
    let shortlist = basic_dispatch(
        &snapshot,
        order,
        DecisionId::new(2),
        CandidatePolicy::Spatial {
            radius: Meters::new(1000.0)?,
            limit: 1,
        },
    )?;
    let DispatchDecisionOutcome::Assigned(selected) = complete.outcome() else {
        return Err("expected exhaustive assignment".into());
    };
    let DispatchDecisionOutcome::Assigned(screened) = shortlist.outcome() else {
        return Err("expected spatial assignment".into());
    };
    assert_eq!(selected.rider, fast);
    assert_eq!(screened.rider, near);
    assert!(world.data().assignments.is_empty());
    let evaluation = complete
        .evidence()
        .candidates
        .iter()
        .find_map(|candidate| match &candidate.result {
            CandidateResult::Feasible { evaluation, .. } if candidate.rider == fast => {
                Some(evaluation)
            }
            _ => None,
        })
        .ok_or("missing selected evaluation")?;

    // Drive execution explicitly; this example is not a discrete-event simulator.
    world.commit(&complete)?;
    world.update_rider_state(
        fast,
        RiderState {
            coordinate: pickup.coordinate,
            availability: Availability::Available,
        },
    )?;
    world.pickup(fast, order, evaluation.pickup_departure)?;
    assert_eq!(onboard_load(world.data(), fast)?, CapacityUnits::new(2));
    world.update_rider_state(
        fast,
        RiderState {
            coordinate: dropoff.coordinate,
            availability: Availability::Available,
        },
    )?;
    world.deliver(fast, order, evaluation.delivery_completed_at)?;
    assert_eq!(
        world.data().fulfillment[&order],
        FulfillmentState::Delivered {
            rider: fast,
            picked_up_at: evaluation.pickup_departure,
            at: evaluation.delivery_completed_at,
        }
    );
    assert!(world.data().assignments.is_empty());
    assert_eq!(world.data().plans[&fast], RiderPlan::default());
    assert_eq!(onboard_load(world.data(), fast)?, CapacityUnits::new(0));
    validate_world(world.data())?;

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "exhaustive_decision": complete,
            "spatial_decision": shortlist,
            "execution": {
                "fulfillment": world.data().fulfillment[&order],
                "remaining_plan": world.data().plans[&fast],
                "onboard_load": onboard_load(world.data(), fast)?,
                "world_version": world.version(),
            }
        }))?
    );
    Ok(())
}
