//! H3 interoperability, deterministic aggregation and dispatch lifecycle boundaries.

#[path = "support/operational_fixture.rs"]
#[allow(dead_code)]
mod fixture;

use fixture::{Fixture, at, ok};
use roadrunner_core::geo::Coordinate;
use roadrunner_dispatch::spatial::{H3Cell, H3DispatchProjection, H3Error, H3Resolution};
use roadrunner_dispatch::*;

fn resolution(value: u8) -> H3Resolution {
    ok(H3Resolution::try_from(value))
}

fn project(f: &Fixture, world: &World, res: u8) -> H3DispatchProjection {
    let provider = ok(CoreRouteProvider::new(&f.graph, TrafficContext::FreeFlow));
    let snapshot = ok(DispatchSnapshot::new(
        world,
        at(100.0),
        RoutingEpoch(at(0.0)),
        &provider,
        &f.anchors,
    ));
    ok(H3DispatchProjection::from_snapshot(
        &snapshot,
        resolution(res),
    ))
}

fn members(grid: &H3DispatchProjection) -> (Vec<RiderId>, Vec<OrderId>) {
    let mut riders: Vec<_> = grid
        .cells()
        .iter()
        .flat_map(|c| c.idle_riders.iter().copied())
        .collect();
    let mut orders: Vec<_> = grid
        .cells()
        .iter()
        .flat_map(|c| c.unassigned_orders.iter().copied())
        .collect();
    riders.sort_unstable();
    orders.sort_unstable();
    (riders, orders)
}

#[test]
fn matches_upstream_reference_and_serializes_cells_as_strings() {
    // H3's quick-start example: https://h3geo.org/docs/quickstart/
    let point = ok(Coordinate::new(37.7955, -122.3937));
    let cell = H3Cell::at(point, resolution(10));
    assert_eq!(cell.to_string(), "8a283082a677fff");
    assert_eq!(ok(serde_json::to_string(&cell)), "\"8a283082a677fff\"");
    assert_eq!(
        ok(serde_json::from_str::<H3Cell>("\"8a283082a677fff\"")),
        cell
    );
    assert_eq!(cell.resolution(), resolution(10));
    assert_eq!(ok(cell.parent(resolution(10))), cell);
    assert_eq!(cell.parent(resolution(11)), Err(H3Error::FinerResolution));
    assert!(serde_json::from_str::<H3Cell>("617700169958293503").is_err());
    for invalid in ["0", "not-h3", "14928308280fffff", "ffffffffffffffff"] {
        assert!(H3Cell::try_from(invalid.to_owned()).is_err());
    }
    assert!(H3Resolution::try_from(16).is_err());
    assert!(serde_json::from_str::<H3Resolution>("255").is_err());
    assert!(serde_json::from_str::<H3Resolution>("-1").is_err());
    assert_eq!(ok(serde_json::to_string(&resolution(0))), "0");
}

#[test]
fn supports_global_coordinates_and_pentagon_hierarchy() {
    for res in 0..=15 {
        for (latitude, longitude) in [(90.0, 0.0), (-90.0, 180.0), (0.0, 0.0), (6.5244, 3.3792)] {
            let cell = H3Cell::at(ok(Coordinate::new(latitude, longitude)), resolution(res));
            assert_eq!(cell.resolution(), resolution(res));
            assert!(cell.parent(resolution(0)).is_ok());
        }
        assert_eq!(
            H3Cell::at(ok(Coordinate::new(0.0, 180.0)), resolution(res)),
            H3Cell::at(ok(Coordinate::new(0.0, -180.0)), resolution(res))
        );
    }
    let pentagon = ok(H3Cell::try_from("8009fffffffffff".to_owned()));
    assert_eq!(pentagon.resolution(), resolution(0));
    assert_eq!(ok(pentagon.parent(resolution(0))), pentagon);
}

#[test]
fn aggregates_snapshot_without_mutation_and_retains_source() {
    let f = Fixture::new();
    let world = ok(World::new(71, f.data.clone()));
    let before = world.data().clone();
    let grid = project(&f, &world, 9);
    assert_eq!(grid.world_identity(), world.identity());
    assert_eq!(grid.world_version(), world.version());
    assert_eq!(grid.evaluated_at(), at(100.0));
    assert_eq!(grid.resolution(), resolution(9));
    assert_eq!(
        members(&grid),
        (
            vec![RiderId::new(1), RiderId::new(2)],
            vec![OrderId::new(1), OrderId::new(2), OrderId::new(3)]
        )
    );
    let json = ok(serde_json::to_value(&grid));
    assert_eq!(json["world_identity"], 71);
    assert_eq!(json["world_version"], 0);
    assert_eq!(json["evaluated_at"], 100.0);
    assert_eq!(json["resolution"], 9);
    assert_eq!(world.data(), &before);
    assert_eq!(grid, project(&f, &world, 9));
    assert!(
        grid.cells()
            .windows(2)
            .all(|pair| pair[0].cell < pair[1].cell)
    );

    let coarse = ok(grid.coarsen(resolution(0)));
    assert_eq!(members(&coarse), members(&grid));
    assert_eq!(
        coarse,
        ok(ok(grid.coarsen(resolution(7))).coarsen(resolution(0)))
    );
    assert_eq!(grid, ok(grid.coarsen(resolution(9))));
    assert_eq!(grid.coarsen(resolution(10)), Err(H3Error::FinerResolution));
    let json_coarse = ok(serde_json::to_value(&coarse));
    for key in ["world_identity", "world_version", "evaluated_at"] {
        assert_eq!(json[key], json_coarse[key]);
    }
    for cell in coarse.cells() {
        assert!(cell.idle_riders.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            cell.unassigned_orders
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
    }
}

#[test]
fn committed_and_cancelled_work_and_unavailable_riders_are_excluded() {
    let mut f = Fixture::new();
    let rider = RiderId::new(1);
    let order = OrderId::new(1);
    f.data
        .assignments
        .insert(order, CommittedAssignment { order, rider });
    f.data
        .plans
        .get_mut(&rider)
        .unwrap_or_else(|| panic!("rider"))
        .stops = vec![Stop::Pickup(order), Stop::Dropoff(order)];
    f.data
        .fulfillment
        .insert(OrderId::new(2), FulfillmentState::Cancelled { at: at(1.0) });
    f.data
        .riders
        .get_mut(&RiderId::new(2))
        .unwrap_or_else(|| panic!("rider"))
        .availability = Availability::Unavailable;
    let mut world = ok(World::new(72, f.data.clone()));
    let grid = project(&f, &world, 9);
    assert_eq!(members(&grid), (vec![], vec![OrderId::new(3)]));

    // Accepted custody and completion never become new pickup demand.
    ok(world.observe_ready(order, at(2.0)));
    ok(world.pickup(rider, order, at(3.0)));
    assert_eq!(members(&project(&f, &world, 9)), members(&grid));
    ok(world.deliver(rider, order, at(4.0)));
    f.anchors
        .riders
        .insert(rider, f.anchors.dropoffs[&order].clone());
    assert_eq!(
        members(&project(&f, &world, 9)),
        (vec![rider], vec![OrderId::new(3)])
    );
    assert_eq!(members(&grid), (vec![], vec![OrderId::new(3)]));
}

#[test]
fn empty_snapshot_remains_empty_and_cannot_be_refined() {
    let mut f = Fixture::new();
    f.data = WorldData::default();
    f.anchors = RoutingAnchors::default();
    let world = ok(World::new(73, f.data.clone()));
    let grid = project(&f, &world, 0);
    assert_eq!(grid.cells(), []);
    assert_eq!(ok(grid.coarsen(resolution(0))).cells(), []);
    assert_eq!(grid.coarsen(resolution(1)), Err(H3Error::FinerResolution));
}
