//! Phase 12 linear-versus-indexed rider radius queries.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion};
use roadrunner_core::geo::{Coordinate, KilometersPerHour, Meters};
use roadrunner_core::spatial::{
    IndexedRiderLocator, LinearRiderLocator, RiderId, RiderLocation, RiderLookup,
};

fn riders(count: usize) -> Vec<RiderLocation> {
    let mut seed = 12_u64;
    (0..count)
        .map(|id| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let latitude = 6.4 + (f64::from((seed >> 32) as u32) / f64::from(u32::MAX)) * 0.25;
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let longitude = 3.25 + (f64::from((seed >> 32) as u32) / f64::from(u32::MAX)) * 0.25;
            RiderLocation {
                id: RiderId::new(id as u64),
                coordinate: Coordinate::new(latitude, longitude)
                    .unwrap_or_else(|error| panic!("coordinate: {error}")),
            }
        })
        .collect()
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    let mut group = criterion.benchmark_group("rider_lookup_5km_limit10");
    group.sample_size(30);
    let center = Coordinate::new(6.5244, 3.3792).unwrap_or_else(|error| panic!("center: {error}"));
    let radius = Meters::new(5_000.0).unwrap_or_else(|error| panic!("radius: {error}"));
    let speed = KilometersPerHour::new(30.0).unwrap_or_else(|error| panic!("speed: {error}"));
    for count in [100, 1_000, 10_000, 100_000] {
        let locations = riders(count);
        let linear = LinearRiderLocator::new(locations.clone(), speed)
            .unwrap_or_else(|error| panic!("linear snapshot: {error}"));
        let indexed = IndexedRiderLocator::new(locations, speed)
            .unwrap_or_else(|error| panic!("indexed snapshot: {error}"));
        assert_eq!(
            linear.nearest_riders(center, radius, 10),
            indexed.nearest_riders(center, radius, 10)
        );
        group.bench_with_input(
            BenchmarkId::new("linear", count),
            &linear,
            |bencher, lookup| {
                bencher.iter(|| black_box(lookup.nearest_riders(black_box(center), radius, 10)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("indexed", count),
            &indexed,
            |bencher, lookup| {
                bencher.iter(|| black_box(lookup.nearest_riders(black_box(center), radius, 10)));
            },
        );
    }
    group.finish();
    criterion.final_summary();
}
