//! Correctness checks for linear and indexed rider lookup.

use roadrunner_core::geo::{Coordinate, KilometersPerHour, Meters};
use roadrunner_core::spatial::{
    IndexedRiderLocator, LinearRiderLocator, RiderId, RiderLocation, RiderLookup, RiderLookupError,
};

fn coordinate(latitude: f64, longitude: f64) -> Coordinate {
    Coordinate::new(latitude, longitude).unwrap_or_else(|error| panic!("coordinate: {error}"))
}

fn meters(value: f64) -> Meters {
    Meters::new(value).unwrap_or_else(|error| panic!("radius: {error}"))
}

fn speed() -> KilometersPerHour {
    KilometersPerHour::new(36.0).unwrap_or_else(|error| panic!("speed: {error}"))
}

fn rider(id: u64, latitude: f64, longitude: f64) -> RiderLocation {
    RiderLocation {
        id: RiderId::new(id),
        coordinate: coordinate(latitude, longitude),
    }
}

fn compare(riders: Vec<RiderLocation>, center: Coordinate, radius: Meters, limit: usize) {
    let linear = LinearRiderLocator::new(riders.clone(), speed())
        .unwrap_or_else(|error| panic!("linear: {error}"));
    let indexed = IndexedRiderLocator::new(riders, speed())
        .unwrap_or_else(|error| panic!("indexed: {error}"));
    assert_eq!(linear.rider_count(), indexed.rider_count());
    assert_eq!(
        linear.nearest_riders(center, radius, limit),
        indexed.nearest_riders(center, radius, limit)
    );
}

#[test]
fn empty_zero_limit_boundary_and_ties() {
    let center = coordinate(0.0, 0.0);
    compare(Vec::new(), center, meters(1_000.0), 5);
    let riders = vec![rider(9, 0.0, 0.0), rider(2, 0.0, 0.0), rider(7, 0.0, 0.01)];
    compare(riders.clone(), center, meters(0.0), 0);
    compare(riders.clone(), center, meters(0.0), 1);
    compare(riders.clone(), center, meters(0.0), 10);
    compare(riders, center, meters(2_000.0), 2);
    let index = IndexedRiderLocator::new([rider(9, 0.0, 0.0)], speed())
        .unwrap_or_else(|error| panic!("index: {error}"));
    let hit = index.nearest_riders(center, meters(0.0), 1);
    assert!(hit[0].estimated_arrival.value().abs() < f64::EPSILON);
}

#[test]
fn global_geometry_matches_scan() {
    let riders = vec![
        rider(1, 0.0, 179.99),
        rider(2, 0.0, -179.99),
        rider(3, 89.99, 90.0),
        rider(4, 89.99, -90.0),
        rider(5, -89.99, 0.0),
        rider(6, 45.0, 0.0),
    ];
    for center in [
        coordinate(0.0, 180.0),
        coordinate(89.99, 180.0),
        coordinate(-90.0, 0.0),
    ] {
        for radius in [
            meters(0.0),
            meters(5_000.0),
            meters(500_000.0),
            meters(50_000_000.0),
        ] {
            compare(riders.clone(), center, radius, 6);
        }
    }
}

#[test]
fn seeded_queries_match_scan() {
    let mut seed = 0x1234_5678_9abc_def0_u64;
    let mut next = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        f64::from((seed >> 32) as u32) / f64::from(u32::MAX)
    };
    let riders: Vec<_> = (0..1_000)
        .map(|id| {
            rider(
                id,
                next().mul_add(180.0, -90.0),
                next().mul_add(360.0, -180.0),
            )
        })
        .collect();
    let linear = LinearRiderLocator::new(riders.clone(), speed())
        .unwrap_or_else(|error| panic!("linear: {error}"));
    let indexed = IndexedRiderLocator::new(riders, speed())
        .unwrap_or_else(|error| panic!("indexed: {error}"));
    for _ in 0..100 {
        let center = coordinate(next().mul_add(180.0, -90.0), next().mul_add(360.0, -180.0));
        let radius = meters(next() * 20_000_000.0);
        for limit in [0, 1, 10, 100] {
            assert_eq!(
                linear.nearest_riders(center, radius, limit),
                indexed.nearest_riders(center, radius, limit)
            );
        }
    }
}

#[test]
fn configuration_validation_and_eta() {
    let duplicates = [rider(1, 0.0, 0.0), rider(1, 0.0, 1.0)];
    assert!(matches!(
        IndexedRiderLocator::new(duplicates, speed()),
        Err(RiderLookupError::DuplicateRider(1))
    ));
    assert!(matches!(
        LinearRiderLocator::new([rider(1, 0.0, 0.0)], KilometersPerHour::ZERO),
        Err(RiderLookupError::ZeroSpeed)
    ));
    let tiny = KilometersPerHour::new(f64::MIN_POSITIVE)
        .unwrap_or_else(|error| panic!("tiny speed: {error}"));
    assert!(matches!(
        IndexedRiderLocator::new([rider(1, 0.0, 0.0)], tiny),
        Err(RiderLookupError::SpeedTooLow)
    ));
    let index = IndexedRiderLocator::new([rider(1, 0.0, 0.01)], speed())
        .unwrap_or_else(|error| panic!("index: {error}"));
    let hit = index.nearest_riders(coordinate(0.0, 0.0), meters(2_000.0), 1);
    assert_eq!(hit.len(), 1);
    assert!((hit[0].estimated_arrival.value() - hit[0].distance.value() / 10.0).abs() < 1.0e-9);
}
